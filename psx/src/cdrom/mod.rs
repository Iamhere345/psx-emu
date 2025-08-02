#![allow(unused)]
use std::collections::VecDeque;

use disc::{CdIndex, Disc};
use log::*;

use crate::{cdrom::disc::Sector, interrupts::{InterruptFlag, Interrupts}, scheduler::{EventType, Scheduler, SchedulerEvent}};
use self::commands::*;

mod commands;
pub mod disc;

struct CdromInterrupts {
	int_flags: u8,
	int_mask: u8,
	int_queue: VecDeque<u8>,
}

impl CdromInterrupts {
	fn new() -> Self {
		Self {
			int_flags: 0,
			int_mask: 0,
			int_queue: VecDeque::new(),
		}
	}

	fn raise_interrupt(&mut self, int: u8, irq: &mut Interrupts) {
		self.int_flags = int & 0x1F;

		if self.int_flags & self.int_mask != 0 {
			trace!("INT{int}");
			irq.raise_interrupt(InterruptFlag::Cdrom);
		}
	}

	fn read_flags(&self) -> u8 {
		self.int_flags | 0xE0
	}

	fn write_mask(&mut self, mask: u8) {
		self.int_mask = mask & 0x1F;
	}

	fn read_mask(&self) -> u8 {
		self.int_mask | 0xE0
	}

	fn ack_interrupt(&mut self, int: u8, params: &mut VecDeque<u8>) {
		trace!("ACK INT{int}");
		self.int_flags &= !(int & 0x1F);

		if (int >> 6) & 1 != 0 {
			// bit 6 clears param FIFO
			trace!("ack clear");
			params.clear();
		}
	}
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum DriveSpeed {
	SingleSpeed = 0,
	DoubleSpeed = 1,
}

impl DriveSpeed {
	pub fn from_bits(bit: bool) -> Self {
		match bit {
			true => Self::DoubleSpeed,
			false => Self::SingleSpeed,
		}
	}
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum DriveState {
	Read = 0x20,
	Seek = 0x40,
	Play = 0x80,
	Idle = 0x00,
}

#[derive(Debug, Clone, Copy)]
enum SectorSize {
	DataOnly,
	WholeSector
}

impl SectorSize {
	pub fn from_bits(bit: bool) -> Self {
		match bit {
			true => Self::WholeSector,
			false => Self::DataOnly,
		}
	}
}

pub struct DataFifo {
	buffer: [u8; 0x930],
	index: usize,
	len: usize
}

impl DataFifo {
	pub fn new() -> Self {
		Self {
			buffer: [0; 0x930],
			index: 0,
			len: 0,
		}
	}

	pub fn read_sector(&mut self, data: &[u8]) {
		self.buffer[..data.len()].copy_from_slice(data);
		self.index = 0;
		self.len = data.len();
	}

	pub fn pop(&mut self) -> u8 {
		if self.index == self.len {
			// if the fifo has been fully read return the last value
			self.buffer[self.len - 1]
		} else {
			let data = self.buffer[self.index];
			self.index += 1;
	
			data
		}
	}

	pub fn is_empty(&self) -> bool {
		self.index == self.len
	}
}

#[derive(Clone, PartialEq)]
pub struct CmdResponse {
	int_level: u8,
	result: Vec<u8>,
	
	second_response: Option<(Box<CmdResponse>, u64)>,
	on_complete: Option<fn(&mut Cdrom) -> Option<(CmdResponse, u64)>>,
}

impl CmdResponse {
	pub fn int3_status(cdrom: &Cdrom) -> Self {
		Self {
			int_level: 3,
			result: vec![cdrom.get_stat()],

			second_response: None,
			on_complete: None,
		}
	}

	pub fn error(cdrom: &Cdrom, error: u8,) -> Self {
		Self {
			int_level: 5,
			result: vec![cdrom.get_stat() | 1, error],

			second_response: None,
			on_complete: None,
		}
	}
}

pub struct Cdrom {
	params_fifo: VecDeque<u8>,
	result_fifo: VecDeque<u8>,
	data_fifio: DataFifo,
	bank: u8,

	int_regs: CdromInterrupts,

	disc: Option<Disc>,

	seek_target: CdIndex,
	current_seek: CdIndex,
	seek_complete: bool,

	read_offset: CdIndex,
	read_paused: bool,

	drive_speed: DriveSpeed,
	drive_state: DriveState,
	sector_size: SectorSize,
	last_sector_size: SectorSize,
	ignore_cur_sector_size: bool,
	motor_on: bool,
}

impl Cdrom {
	pub fn new() -> Self {
		Self {
			params_fifo: VecDeque::new(),
			result_fifo: VecDeque::new(),
			data_fifio: DataFifo::new(),
			bank: 0,

			int_regs: CdromInterrupts::new(),

			disc: None,

			seek_target: CdIndex::ZERO,
			current_seek: CdIndex::ZERO,
			seek_complete: false,

			read_offset: CdIndex::ZERO,
			read_paused: false,

			drive_speed: DriveSpeed::SingleSpeed,
			drive_state: DriveState::Idle,
			sector_size: SectorSize::DataOnly,
			last_sector_size: SectorSize::DataOnly,
			ignore_cur_sector_size: false,
			motor_on: true,
		}
	}

	pub fn load_disc(&mut self, disc: Disc) {
		self.disc = Some(disc);
	}

	pub fn read8(&mut self, addr: u32) -> u8 {
		let reg = addr & 0xF;

		if reg != 2 {
			trace!("[{}][0x{addr:X}] CDROM read", self.bank);
		}

		match reg {
			// status register
			0 => self.read_status(),
			// result
			1 => { 
				let result = self.result_fifo.pop_front().or(Some(0)).unwrap();
				//trace!("read result fifo: 0x{result:X}");

				result 
			},
			// RDDATA
			2 => {
				//trace!("read data fifo: 0x{data:X}");
				self.data_fifio.pop()
			},
			// either int mask or flags
			3 => match self.bank {
				0 | 2 => self.int_regs.read_mask(),
				1 | 3 => self.int_regs.read_flags(),
				_ => unreachable!(),
			}
			
			_ => todo!("CDROM read [0x{addr:X}][{}]", self.bank),
		}
	}

	pub fn write8(&mut self, addr: u32, write: u8, scheduler: &mut Scheduler) {
		let reg = addr & 0xF;

		trace!("[{}][0x{addr:X}] CDROM write 0x{write:X}", self.bank);

		match self.bank {
			0 => match reg {
				0 => self.write_status(write),
				1 => self.exec_cmd(write, scheduler),
				2 => { 
					trace!("write 0x{write:X} to fifo"); 
					self.params_fifo.push_back(write); 
				},
				3 => trace!("request register write: BFRD: {} DRQSTS: {}", (write >> 7) & 1, !self.data_fifio.is_empty()),
				_ => todo!("CDROM write [0x{addr:X}][{}] 0x{write:X}", self.bank),
			},
			1 => match reg {
				0 => self.write_status(write),
				2 => self.int_regs.write_mask(write),
				3 => self.int_regs.ack_interrupt(write, &mut self.params_fifo),
				_ => todo!("CDROM write [0x{addr:X}][{}] 0x{write:X}", self.bank),
			},
			2 => match reg {
				0 => self.write_status(write),
				2 => warn!("Unhandled write to ATV0"),
				3 => warn!("Unhandled write to ATV1"),
				_ => todo!("CDROM write [0x{addr:X}][{}] 0x{write:X}", self.bank),
			},
			3 => match reg {
				0 => self.write_status(write),
				1 => warn!("Unhandled write to ATV2"),
				2 => warn!("Unhandled write to ATV3"),
				3 => warn!("Unhandled write to ADPCTL"),
				_ => todo!("CDROM write [0x{addr:X}][{}] 0x{write:X}", self.bank),
			},
			
			_ => unimplemented!("CDROM bank {}", self.bank),
		}
	}

	pub fn read_status(&mut self) -> u8 {
		let result = self.bank
			| (u8::from(self.params_fifo.is_empty()) << 3)
			| (u8::from(!(self.params_fifo.len() >= 16)) << 4)
			| (u8::from(!self.result_fifo.is_empty()) << 5)
			| (u8::from(!self.data_fifio.is_empty()) << 6)
			| (u8::from(self.int_regs.int_flags != 0) << 7);
		
		trace!("read status: 0b{result:b}");

		if !self.data_fifio.is_empty() {
			//debug!("read DRQSTS true");
		}

		result
	}

	pub fn write_status(&mut self, write: u8) {
		self.bank = write & 3;
	}

	fn exec_cmd(&mut self, cmd: u8, scheduler: &mut Scheduler) {
		//info!("exec cmd 0x{cmd:X}");

		let (response, delay) = match cmd {
			// Nop
			0x1 => self.nop(),
			// Setloc
			0x2 => self.set_loc(),
			// Play
			0x3 => self.play(),
			// ReadN
			0x6 => self.read_n(),
			// Stop
			0x8 => self.stop(),
			// Pause
			0x9 => self.pause(),
			// GetLocP
			0x11 => self.get_loc_p(),
			// Init
			0xA => self.init(),
			// Demute (stubbed)
			0xC => (CmdResponse::int3_status(&self), AVG_CYCLES),
			// Setfilter (stubbed)
			0xD => (CmdResponse::int3_status(&self), AVG_CYCLES),
			// Setmode
			0xE => self.set_mode(),
			// GetTN
			0x13 => self.get_tn(),
			// GetTD
			0x14 => self.get_td(),
			// SeekL
			0x15 => self.seek_l(),
			// SeekP
			0x16 => self.seek_p(),
			// Test
			0x19 => self.test(),
			// GetID
			0x1A => self.get_id(),
			// ReadS (currently the same as ReadN)
			0x1B => self.read_n(),

			_ => todo!("cmd 0x{cmd:X}")
		};

		self.params_fifo.clear();

		scheduler.schedule_event(SchedulerEvent::new(EventType::CdromCmd(response)), delay);
	}

	// different from STATUS/ADDRESS register
	pub fn get_stat(&self) -> u8 {
		let result = (u8::from(self.motor_on) << 1) // motor state
			| (u8::from(self.disc.is_none()) << 4)		// shell open
			| (self.drive_state as u8);			// reading data sectors
		
		trace!("getstat: 0b{result:b} (drive state: {:?}", self.drive_state);

		result
	}

	pub fn handle_cmd_response(&mut self, response: CmdResponse, scheduler: &mut Scheduler, irq: &mut Interrupts) {

		// stops an extra INT1 from being raised after a pause
		// TODO find a better way to fix this
		if response.int_level == 1 && self.read_paused {
			return;
		}

		self.int_regs.raise_interrupt(response.int_level, irq);

		for result in response.result {
			self.result_fifo.push_back(result);
		}

		if let Some((second_response, delay)) = response.second_response {
			scheduler.schedule_event(SchedulerEvent::new(EventType::CdromCmd(*second_response)), delay);
		}

		if let Some(on_complete) = response.on_complete {
			if let Some((next_response, delay)) = on_complete(self) {
				//trace!("ReadN next INT1 scheduled");
				scheduler.schedule_event(SchedulerEvent::new(EventType::CdromCmd(next_response)), delay);
			}
		}
		
	}
}
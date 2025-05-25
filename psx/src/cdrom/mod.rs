#![allow(unused)]
use std::collections::VecDeque;

use disc::{CdIndex, Disc};
use log::*;

use crate::{interrupts::{InterruptFlag, Interrupts}, scheduler::{EventType, Scheduler, SchedulerEvent}};
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

#[derive(Clone, Copy, Debug)]
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

#[derive(Debug)]
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
	data_fifio: VecDeque<u8>,
	bank: u8,

	int_regs: CdromInterrupts,

	disc: Option<Disc>,

	seek_target: CdIndex,
	current_seek: CdIndex,
	seek_complete: bool,

	read_offset: CdIndex,
	read_paused: bool,
	reading: bool,

	drive_speed: DriveSpeed,
	sector_size: SectorSize,
	motor_on: bool,
}

impl Cdrom {
	pub fn new() -> Self {
		Self {
			params_fifo: VecDeque::new(),
			result_fifo: VecDeque::new(),
			data_fifio: VecDeque::new(),
			bank: 0,

			int_regs: CdromInterrupts::new(),

			disc: None,

			seek_target: CdIndex::ZERO,
			current_seek: CdIndex::ZERO,
			seek_complete: false,

			read_offset: CdIndex::ZERO,
			read_paused: false,
			reading: false,

			drive_speed: DriveSpeed::SingleSpeed,
			sector_size: SectorSize::DataOnly,
			motor_on: true,
		}
	}

	pub fn load_disc(&mut self, disc: Disc) {
		self.disc = Some(disc);
	}

	pub fn read8(&mut self, addr: u32) -> u8 {
		let reg = addr & 0xF;

		//trace!("[{}][0x{addr:X}] CDROM read", self.bank);

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
				let data = self.data_fifio.pop_front().or(Some(0)).unwrap();
				//trace!("read data fifo: 0x{data:X}");

				data
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

		//trace!("[{}][0x{addr:X}] CDROM write 0x{write:X}", self.bank);

		match self.bank {
			0 => match reg {
				0 => self.write_status(write),
				1 => self.exec_cmd(write, scheduler),
				2 => { info!("write 0x{write:X} to fifo"); self.params_fifo.push_back(write) },
				3 => debug!("request register write: BFRD: {}", (write >> 7) & 1),
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
			| (u8::from(!self.data_fifio.is_empty()) << 6);

		//trace!("read status: 0b{result:b}");

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
			// ReadN
			0x6 => self.read_n(),
			// Pause
			0x9 => self.pause(),
			// Init
			0xA => self.init(),
			// Demute (stubbed)
			0xC => (CmdResponse::int3_status(&self), AVG_CYCLES),
			// Setmode
			0xE => self.set_mode(),
			// SeekL
			0x15 => self.seek_l(),
			// Test
			0x19 => self.test(),
			// GetID
			0x1A => self.get_id(),
			_ => todo!("cmd 0x{cmd:X}")
		};

		self.params_fifo.clear();

		scheduler.schedule_event(SchedulerEvent::new(EventType::CdromCmd(response)), delay);
	}

	// different from STATUS/ADDRESS register
	pub fn get_stat(&self) -> u8 {
		let result = (u8::from(self.motor_on) << 1) // motor state
			| (u8::from(self.disc.is_none()) << 4)		// shell open
			| (u8::from(self.reading) << 5);			// reading data sectors
		
		//trace!("getstat: 0b{result:b}");

		result
	}

	pub fn handle_cmd_response(&mut self, response: CmdResponse, scheduler: &mut Scheduler, irq: &mut Interrupts) {
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
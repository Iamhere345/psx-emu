#![allow(dead_code)]
use std::array;
use log::*;

use crate::{bus::Bus, interrupts::Interrupts, scheduler::{Scheduler, SchedulerEvent}};

const CHANNEL_MDECIN: usize = 0;
const CHANNEL_MDECOUT: usize = 1;
const CHANNEL_GPU: usize = 2;
const CHANNEL_CDROM: usize = 3;
const CHANNEL_SPU: usize = 4;
const CHANNEL_PIO: usize = 5;
const CHANNEL_OTC: usize = 6;

const CDROM_CLKS: u64 = 40;
const AVG_CLKS: u64 = 1;

#[derive(Clone, Copy, Debug, Default)]
pub enum SyncMode {
	// transfer data all at once after DREQ is first asserted
	#[default]
	Burst = 0,
	// split data into blocks, transfer next block whenever DREQ is asserted
	Slice = 1,
	// used for GPU OTC
	LinkedList = 2
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
enum DmaDirection {
	#[default]
	ToRam = 0,
	FromRam = 1,
}

#[derive(Clone, Copy, Debug, Default)]
enum StepDirection {
	#[default]
	Inc = 0,
	Dec = 1,
}


#[derive(Clone, Debug, Default)]
pub struct Channel {
	pub channel_num: usize,

	base_addr: u32,
	
	block_size: u32,
	block_amount: u32,

	transfer_dir: DmaDirection,
	step_dir: StepDirection,

	pub sync_mode: SyncMode,

	chopping_enabled: bool,
	chopping_dma_window_size: u8,
	chopping_cpu_window_size: u8,

	pub transfer_active: bool,
	pub manual_trigger: bool,

	// unimplemented
	pause_transfer: bool,
	bus_snooping: bool,
}

impl Channel {
	pub fn new(num: usize) -> Self {
		// all zeros besides bit 1 (for OTC)
		Self { channel_num: num, step_dir: StepDirection::Dec, ..Default::default() }
	}

	pub fn read32(&self, addr: u32) -> u32 {
		match (addr >> 2) & 3 {
			0 => self.base_addr,
			1 => ((self.block_amount as u32) << 16) | self.block_size as u32,
			// channel control register
			2 => {
				self.transfer_dir as u32
					| (self.step_dir as u32) << 1
					| (self.chopping_enabled as u32) << 8
					| (self.sync_mode as u32) << 9
					| (self.chopping_dma_window_size as u32) << 16
					| (self.chopping_cpu_window_size as u32) << 20
					| (self.transfer_active as u32) << 24
					| (self.manual_trigger as u32) << 28
					| (self.pause_transfer as u32) << 29
					| (self.bus_snooping as u32) << 30

			},
			_ => unreachable!()
		}
	}

	pub fn write32(&mut self, addr: u32, write: u32) {

		trace!("[DMA{}] write 0x{write:X} to register {}", self.channel_num, (addr >> 0x2) & 0x3);

		match (addr >> 0x2) & 0x3 {
			0 => self.base_addr = write,
			1 => {
				self.block_size = write & 0xFFFF;
				self.block_amount = (write >> 16) & 0xFFFF;

				if self.block_size == 0 {
					self.block_size = 0x10000;
				}

				if self.block_amount == 0 {
					self.block_amount = 0x10000;
				}

				trace!("[DMA{}] block size: 0x{:X} block amount: 0x{:X}", self.channel_num, self.block_size, self.block_amount);
			},
			// channel control register
			2 => {				
				self.transfer_active = (write >> 24) & 1 != 0;
				self.manual_trigger = (write >> 28) & 1 != 0;

				self.bus_snooping = (write >> 30) & 1 != 0;

				if self.channel_num == CHANNEL_OTC {
					// other bits aren't writable in DMA6
					return;
				}

				self.transfer_dir = match (write & 1) != 0 {
					true => DmaDirection::FromRam,
					false => DmaDirection::ToRam
				};

				self.step_dir = match ((write >> 1) & 1) != 0 {
					true => StepDirection::Dec,
					false => StepDirection::Inc,
				};

				self.chopping_enabled = (write >> 8) & 1 != 0;

				self.sync_mode = match (write >> 9) & 3 {
					0 => SyncMode::Burst,
					1 => SyncMode::Slice,
					2 => SyncMode::LinkedList,
					_ => unreachable!(),
				};

				self.chopping_dma_window_size = ((write >> 16) & 7) as u8;
				self.chopping_cpu_window_size = ((write >> 20) & 7) as u8;

				self.pause_transfer = (write >> 29) & 1 != 0;

				self.bus_snooping = (write >> 30) & 1 != 0;
				
				trace!("DMA{}: new control {self:X?} (write 0x{write:X})", self.channel_num);

			},
			_ => unreachable!()
		}
	}

	pub fn active(&self) -> bool {
		let trigger = match self.sync_mode {
			SyncMode::Burst => self.manual_trigger,
			_ => true
		};

		trace!("DMA{} active ({} && {})", self.channel_num, self.transfer_active, trigger);
		
		// TODO check which is more accurate
		//self.transfer_active && trigger
		self.transfer_active
	}
}

#[derive(Default)]
struct DmaControlRegister {
	channel_enable: [bool; 7],
	channel_priority: [u8; 7],
}

impl DmaControlRegister {
	pub fn new() -> Self {
		let mut dmac = Self::default();
		dmac.write(0x07654321);

		dmac
	}

	pub fn read(&self) -> u32 {
		let priority_bits = self
			.channel_priority
			.into_iter()
			.enumerate()
			.map(|(channel, priority)| u32::from(priority) << (4 * channel))
			.reduce(|a, b| a | b)
			.unwrap();

		let enable_bits = self
			.channel_enable
			.into_iter()
			.enumerate()
			.map(|(channel, enabled)| u32::from(enabled) << (3 + 4 * channel))
			.reduce(|a, b| a | b)
			.unwrap();
		
		priority_bits | enable_bits
	}

	pub fn write(&mut self, write: u32) {
		trace!("DPCR: write: 0x{write:08X} channel_enable: {:?} channel_priority: {:?}", self.channel_enable, self.channel_priority);

		self.channel_priority = array::from_fn(|i| ((write >> (4 * i)) & 7) as u8);
		self.channel_enable = array::from_fn(|i| ((write >> (3 + 4 * i)) & 1) != 0);
	}

}

struct DmaInterruptRegister {
	channel_int: u8,
	channel_mask: u8,

	int_cond: u8,

	bus_error: bool,

	master_enable: bool,
	master_flag: bool,
}

impl DmaInterruptRegister {
	pub fn new() -> Self {
		Self {
			channel_int: 0,
			channel_mask: 0,

			int_cond: 0,

			bus_error: false,

			master_enable: false,
			master_flag: false,
		}
	}

	pub fn read(&self) -> u32 {

		self.int_cond as u32
			| (self.bus_error as u32) << 15
			| (self.channel_mask as u32) << 16
			| (self.master_enable as u32) << 23
			| (self.channel_int as u32) << 24
			| (self.master_flag as u32) << 31

	}

	pub fn write(&mut self, write: u32) {

		self.int_cond = (write & 0x7F) as u8;
		self.bus_error = (write >> 15) & 0x1 != 0;
		self.channel_mask = ((write >> 16) & 0x7F) as u8;
		self.master_enable = (write >> 23) & 0x1 != 0;
		self.channel_int &= !((write >> 24) & 0x7F) as u8;

		self.set_master_int();

	}

	pub fn raise_int(&mut self, flag: u8, interrupts: &mut Interrupts) {
		// interrupts are set ONLY if they are also set in the mask
		if self.channel_mask & (1 << flag) != 0 {
			trace!("DMA{flag} int");
			self.channel_int |= 1 << flag;
		}
		
		trace!("DMA{flag} no int. mask: 0b{:b} & 0b{:b}", self.channel_mask, 1 << flag);

		let old_master_int = self.master_flag;
		self.set_master_int();

		trace!("{} && !{}", self.master_flag, old_master_int);

		if self.master_flag {
			trace!("DMA{flag} raise int");
			interrupts.raise_interrupt(crate::interrupts::InterruptFlag::Dma);
		}
	}

	pub fn set_master_int(&mut self) {
		if self.bus_error || (self.master_enable && (self.channel_mask & self.channel_int) != 0) {
			self.master_flag = true;
		} else {
			self.master_flag = false;
		}
	}

	pub fn int_cond(&self, flag: u8) -> bool {
		(self.int_cond & (1 << flag)) != 0
	}
}

pub struct DmaController {
	pub channels: [Channel; 7],
	control: DmaControlRegister,
	irq: DmaInterruptRegister,
}

impl DmaController {
	pub fn new() -> Self {
		Self {
			channels: array::from_fn(|i| Channel::new(i)),
			control: DmaControlRegister::new(),
			irq: DmaInterruptRegister::new(),
		}
	}

	pub fn read8(&self, addr: u32) -> u8 {
		match addr & 0xFF {
			// DPCR
			0xF0 => (self.control.read() >> 0) as u8,
			0xF1 => (self.control.read() >> 8) as u8,
			0xF2 => (self.control.read() >> 16) as u8,
			0xF3 => (self.control.read() >> 24) as u8,
			// DICR
			0xF4 => (self.irq.read() >> 0) as u8,
			0xF5 => (self.irq.read() >> 8) as u8,
			0xF6 => (self.irq.read() >> 16) as u8,
			0xF7 => (self.irq.read() >> 24) as u8,

			_ => unreachable!("[0x{addr:X}] DMA read8")
		}
	}

	pub fn read32(&self, addr: u32) -> u32 {
		let channel = (addr >> 0x4) & 0x7;

		
		match addr {
			// channel registers
			0x1F801080	..= 0x1F8010EF => {
				//trace!("[0x{addr:X}] DMA{channel} read32");
				self.channels[channel as usize].read32(addr)
			},
			// DMA control
			0x1F8010F0 => {
				trace!("DPCR read");
				self.control.read()
			},
			// DMA interrupt
			0x1F8010F4 => {
				trace!("DICR read");
				self.irq.read()
			},

			_ => unreachable!()
		}
	}

	pub fn write32(&mut self, addr: u32, write: u32) {
		let channel = (addr >> 0x4) & 0x7;
		
		match addr {
			// channel registers
			0x1F801080	..= 0x1F8010EF => {
				if channel == 4 {
					error!("[0x{addr:X}] DMA{channel} write32 0x{write:X}");
				}
				trace!("[0x{addr:X}] DMA{channel} write32 0x{write:X}");
				self.channels[channel as usize].write32(addr, write)
			},
			// DMA control
			0x1F8010F0 => {
				trace!("DPCR write 0x{write:X}");
				self.control.write(write)
			},
			// DMA interrupt
			0x1F8010F4 => {
				trace!("DICR write 0x{write:X}");
				self.irq.write(write)
			},

			_ => unreachable!()
		}
	}

	pub fn write8(&mut self, addr: u32, write: u8) {
		match addr & 0xFF {
			// DPCR
			0xF0 => self.control.write(u32::from(write) << 00),
			0xF1 => self.control.write(u32::from(write) << 08),
			0xF2 => self.control.write(u32::from(write) << 16),
			0xF3 => self.control.write(u32::from(write) << 24),
			// DICR
			0xF4 => self.irq.write(u32::from(write) << 00),
			0xF5 => self.irq.write(u32::from(write) << 08),
			0xF6 => self.irq.write(u32::from(write) << 16),
			0xF7 => self.irq.write(u32::from(write) << 24),

			_ => unreachable!("[0x{addr:X}] DMA read8")
		};
	}

	pub fn raise_int(&mut self, flag: u8, interrupts: &mut Interrupts) {
		self.irq.raise_int(flag, interrupts);
	}

}

impl Bus {
	pub fn do_dma(&mut self, channel: usize, scheduler: &mut Scheduler) {

		trace!("doing DMA{channel} {:?}", self.dma.channels[channel].sync_mode);

		if !self.dma.control.channel_enable[channel] {
			warn!("triggered DMA{channel} when disabled in control reg");
			return;
		}

		if channel == CHANNEL_OTC {
			self.do_dma_otc(scheduler);
			return;
		}

		let words = match self.dma.channels[channel].sync_mode {
			SyncMode::LinkedList => self.do_dma_linked_list(channel, scheduler),
			_ => self.do_dma_block(channel, scheduler),
		};

		let dma_clks = match channel {
			CHANNEL_CDROM => CDROM_CLKS,
			_ => AVG_CLKS
		};

		scheduler.schedule_event(SchedulerEvent::new(crate::scheduler::EventType::DmaIrq(channel as u8)), words * dma_clks);

	}
	
	fn do_dma_linked_list(&mut self, channel_num: usize, scheduler: &mut Scheduler) -> u64 {
		
		assert_eq!(channel_num, 2);
		assert_eq!(self.dma.channels[channel_num].transfer_dir, DmaDirection::FromRam);
		
		trace!("start linked list DMA{channel_num} step: {:?}", self.dma.channels[channel_num].step_dir);

		let channel = self.dma.channels[channel_num].clone();

		let mut addr = channel.base_addr;
		let mut words_sent = 0;

		loop {

			let header = self.read32(addr, scheduler);
			let words_to_send = header >> 24;
			let next_addr = header & 0xFFFFFF;

			//trace!("node: 0x{header:X} word count: 0x{words_to_send:X} next addr: 0x{next_addr:X}");

			for i in 0..words_to_send {

				let data = self.read32( addr.wrapping_add(4 * (i + 1)), scheduler);
				self.gpu.gp0_cmd(data);

				//trace!("[0x{i:X}] linked list write 0x{data:X} to GP0");
				words_sent += 1;
			}

			addr = next_addr;

			// the end node only needs bit 23 to be set
			if next_addr & (1 << 23) != 0 {
				//trace!("linked list end (old addr is 0x{addr:X})");
				break;
			}

		}

		self.dma.channels[channel_num].transfer_active = false;
		self.dma.channels[channel_num].manual_trigger = false;

		words_sent

	}

	fn do_dma_otc(&mut self, scheduler: &mut Scheduler) -> u64 {

		let mut addr = self.dma.channels[CHANNEL_OTC].base_addr;
		let mut dma_len = self.dma.channels[CHANNEL_OTC].block_size as u32;

		if dma_len == 0 {
			dma_len = 0x10000;
		}

		trace!("DMA6 len: 0x{dma_len:X} start: 0x{addr:X}");
		
		for i in 0..dma_len {

			//println!("[0x{addr:X}] writing OTC");
			
			let next_addr = if i == dma_len - 1 {
				trace!("DMA6 end: 0x{addr:X}");
				0xFFFFFF
			} else {
				addr.wrapping_sub(4) & 0x1FFFFF
			};


			self.write32(addr, next_addr, scheduler);
			addr = next_addr;

		}

		self.dma.channels[CHANNEL_OTC].transfer_active = false;
		self.dma.channels[CHANNEL_OTC].manual_trigger = false;

		dma_len as u64

	}

	fn do_dma_block(&mut self, channel_num: usize, scheduler: &mut Scheduler) -> u64 {

		let channel = self.dma.channels[channel_num].clone();

		let step = match channel.step_dir {
			StepDirection::Inc => 4,
			StepDirection::Dec => -4,
		};

		let mut addr = channel.base_addr;
		let words_left = match channel.sync_mode {
			SyncMode::Burst => channel.block_size,
			SyncMode::Slice => channel.block_size * channel.block_amount,
			SyncMode::LinkedList => unimplemented!()
		};

		trace!("doing DMA{channel_num} start: 0x{addr:X} words: 0x{words_left:X}");

		for _ in 0..words_left {

			match channel.transfer_dir {
				DmaDirection::FromRam => {
					let word = self.read32(addr, scheduler);

					match channel_num {
						CHANNEL_GPU => {
							//trace!("dma block write 0x{word:X} to GP0");
							self.gpu.gp0_cmd(word);
						},
						CHANNEL_SPU => {
							self.spu.write_sram(word as u16);
							self.spu.write_sram((word >> 16) as u16);
						},
						CHANNEL_MDECIN => {
							// stubbed
						},
						_ => todo!("FromRam DMA{channel_num}")
					}
				},

				DmaDirection::ToRam => {
					let word = match channel_num {
						CHANNEL_GPU => {
							let read = self.gpu.read32(0x1F801810);
							//trace!("DMA block read 0x{read:X} from GP0");

							read
						},
						CHANNEL_CDROM => {
							let mut data = [0; 4];

							for byte in &mut data {
								*byte = self.cdrom.read8(0x1F801802);
								//trace!("DMA read 0x{byte:X} from CDROM");
							}

							u32::from_le_bytes(data)
						},
						CHANNEL_MDECOUT => {
							// stubbed
							0xFF
						},
						CHANNEL_SPU => {
							u32::from(self.spu.read_sram())
								| u32::from(self.spu.read_sram()) << 16
						}
						_ => todo!("ToRam DMA{channel_num} mode {:?}", channel.sync_mode),
					};
					
					self.write32(addr, word, scheduler);
				}
			}

			addr = ((addr as i32).wrapping_add(step) as u32) & 0x1FFFFFFF;

		}

		trace!("DMA{channel_num} finished");

		self.dma.channels[channel_num].transfer_active = false;
		self.dma.channels[channel_num].manual_trigger = false;

		words_left as u64

	}

}
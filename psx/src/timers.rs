use log::error;

use crate::scheduler::Scheduler;

#[derive(Clone, Copy)]
enum ResetMode {
	AfterOverflow = 0,
	AfterTarget = 1,
}

impl ResetMode {
	fn from_bits(bits: u32) -> Self {
		if bits == 0 {
			Self::AfterOverflow
		} else {
			Self::AfterTarget
		}
	}
}

pub struct Timers {
	timers: [Timer; 3]
}

impl Timers {
	pub fn new() -> Self {
		Self {
			timers: [Timer::new(0), Timer::new(1), Timer::new(2)]
		}
	}

	pub fn read32(&self, addr: u32, scheduler: &mut Scheduler) -> u32 {
		let index = ((addr >> 4) & 3) as usize;
		let reg = addr & 0xF;

		if index >= 3 {
			error!("timer index {index}");
		}

		match reg {
			0 => self.timers[index].read_counter(scheduler),
			4 => self.timers[index].read_mode(),
			8 => self.timers[index].read_target(),
			_ => unimplemented!("read timer{index} register {reg}")
		}
	}

	pub fn write32(&mut self, addr: u32, write: u32, scheduler: &mut Scheduler) {
		let index = ((addr >> 4) & 3) as usize;
		let reg = addr & 0xF;

		if index >= 3 {
			error!("timer index {index}");
		}

		match reg {
			0 => self.timers[index].write_counter(write as u16, scheduler),
			4 => self.timers[index].write_mode(write, scheduler),
			8 => self.timers[index].write_target(write as u16, scheduler),
			_ => unimplemented!("write timer{index} register [{reg}] 0x{write:X}")
		};
	}
}

pub struct Timer {
	timer_num: u8,

	counter: u16,
	target: u16,

	use_sync_mode: bool,	// (0=Free Run, 1=Synchronize via Bit1-2)
	sync_mode: u8,
	reset_after: ResetMode,
	irq_at_target: bool,
	irq_at_overflow: bool,
	irq_repeat: bool,		// 0=One-shot, 1=Repeatedly
	irq_pulse: bool,		// 0=Short Bit10=0 Pulse, 1=Toggle Bit10 on/off
	clock_src: u8,
	irq: bool,				// (0=Yes, 1=No) (Set to 1 after Writing)
	reached_target: bool,	// (0=No, 1=Yes) (Reset after Reading) (read-only)
	reached_overflow: bool,	// (0=No, 1=Yes) (Reset after Reading) (read-only)
}

impl Timer {
	pub fn new(timer: u8) -> Self {
		Self {
			timer_num: timer,

			counter: 0,
			target: 0,

			use_sync_mode: false,
			sync_mode: 0,
			reset_after: ResetMode::AfterOverflow,
			irq_at_target: false,
			irq_at_overflow: false,
			irq_repeat: false,
			irq_pulse: false,
			clock_src: 0,
			irq: true,
			reached_target: false,
			reached_overflow: false,
		}
	}

	pub fn read_counter(&self, scheduler: &mut Scheduler) -> u32 {
		self.counter as u32
	}

	pub fn write_counter(&mut self, write: u16, scheduler: &mut Scheduler) {
		self.counter = write;

		// TODO reshedule events
	}

	pub fn read_target(&self) -> u32 {
		self.target as u32
	}

	pub fn write_target(&mut self, write: u16, scheduler: &mut Scheduler) {
		self.target = write;

		// TODO reschedule events
	}

	pub fn read_mode(&self) -> u32 {
		u32::from(self.use_sync_mode)
			| u32::from(self.sync_mode) << 1
			| (self.reset_after as u32) << 3
			| u32::from(self.irq_at_target) << 4
			| u32::from(self.irq_at_overflow) << 5
			| u32::from(self.irq_repeat) << 6
			| u32::from(self.irq_pulse) << 7
			| u32::from(self.clock_src) << 8
			| u32::from(self.irq) << 10
			| u32::from(self.reached_target) << 11
			| u32::from(self.reached_overflow) << 12
	}

	pub fn write_mode(&mut self, write: u32, scheduler: &mut Scheduler) {
		self.use_sync_mode = write & 1 != 0;
		self.sync_mode = (write >> 1) as u8 & 3;
		self.reset_after = ResetMode::from_bits((write >> 3) & 1);
		self.irq_at_target = (write >> 4) & 1 != 0;
		self.irq_at_overflow = (write >> 5) & 1 != 0;
		self.irq_repeat = (write >> 6) & 1 != 0;
		self.irq_pulse = (write >> 7) & 1 != 0;
		self.clock_src = (write >> 8) as u8 & 3;
		self.irq = true;
		self.reached_target = (write >> 11) & 1 != 0;
		self.reached_overflow = (write >> 2) & 1 != 0;
	}
}
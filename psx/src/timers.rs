use log::*;

use crate::{interrupts::*, scheduler::*};

#[derive(Clone, Copy, PartialEq, Debug)]
enum ResetMode {
	AfterOverflow = 0,
	AfterTarget = 1,
}

#[derive(Debug)]
enum ClockSource {
	System,
	SystemDiv,
	Dot,
	Hblank
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

	pub fn read32(&mut self, addr: u32, scheduler: &mut Scheduler) -> u32 {
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

	pub fn overflow_event(&mut self, timer_num: u8, scheduler: &mut Scheduler, interrupts: &mut Interrupts) {

		trace!("Timer{timer_num} overflow");

		let timer = &mut self.timers[timer_num as usize];

		if timer.irq_at_overflow && timer.irq {
			// false=irq fired
			timer.irq = false;
			interrupts.raise_interrupt(timer.irq_src());
		}

		timer.reached_overflow = true;

		timer.counter = 0;

		let overflow_cycles = timer.convert_cycles(0xFFFF);
		timer.overflow_cycles_away = overflow_cycles;
		scheduler.schedule_event(SchedulerEvent::new(EventType::TimerOverflow(timer_num)), overflow_cycles);

	}

	pub fn target_event(&mut self, timer_num: u8, scheduler: &mut Scheduler, interrupts: &mut Interrupts) {
		let timer = &mut self.timers[timer_num as usize];

		if timer.irq_at_target && timer.irq {
			// false=irq fired
			timer.irq = false;
			interrupts.raise_interrupt(timer.irq_src());
		}

		timer.reached_target = true;

		if timer.reset_after == ResetMode::AfterTarget {
			timer.counter = 0;

			// only reschedule the overflow event
			scheduler.remove_event(EventType::TimerOverflow(timer_num));

			let overflow_cycles = timer.convert_cycles(0xFFFF);
			timer.overflow_cycles_away = overflow_cycles;
			scheduler.schedule_event(SchedulerEvent::new(EventType::TimerOverflow(timer_num)), overflow_cycles);
		}

		let cycles = if timer.counter == timer.target {
			0xFFFF - timer.counter + timer.target
		} else {
			timer.target
		};

		trace!("Timer{timer_num} target 0x{:X} reset mode: {:?} (0xFFFF - 0x{:X} + 0x{:X})", timer.convert_cycles(cycles), timer.reset_after, timer.counter, timer.target);

		let target_cycles = timer.convert_cycles(cycles);
		scheduler.schedule_event(SchedulerEvent::new(EventType::TimerTarget(timer_num)), target_cycles);

	}
}

// TODO IRQ repeat/pulse
// TODO sync mode
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
	clock_src: ClockSource,
	clock_src_raw: u8,
	irq: bool,				// (0=Yes, 1=No) (Set to 1 after Writing)
	reached_target: bool,	// (0=No, 1=Yes) (Reset after Reading) (read-only)
	reached_overflow: bool,	// (0=No, 1=Yes) (Reset after Reading) (read-only)

	overflow_cycles_away: u64,
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
			clock_src: ClockSource::System,
			clock_src_raw: 0,
			irq: true,
			reached_target: false,
			reached_overflow: false,

			overflow_cycles_away: 0,
		}
	}

	pub fn read_counter(&self, scheduler: &mut Scheduler) -> u32 {
		let overflow_ev = scheduler.get_event(EventType::TimerOverflow(self.timer_num));

		if let Some(event) = overflow_ev {
			let cycles_away = scheduler.event_cycles_away(event);
			let counter = 0xFFFF - ((cycles_away as f32) / (self.overflow_cycles_away as f32) * 0xFFFF as f32) as u16;
			trace!("read Timer{} src: {:?} counter 0x{:X} (0xFFFF - (0x{cycles_away:X} / 0x{:X}) * 0xFFFF", self.timer_num, self.clock_src, counter, self.overflow_cycles_away);

			return counter as u32;
		} else {
			error!("couldn't get overflow event");
		}

		0
	}

	pub fn write_counter(&mut self, write: u16, scheduler: &mut Scheduler) {
		self.counter = write;

		self.reschedule_events(scheduler);
	}

	pub fn read_target(&self) -> u32 {
		self.target as u32
	}

	pub fn write_target(&mut self, write: u16, scheduler: &mut Scheduler) {
		self.target = write;

		self.reschedule_events(scheduler);
	}

	pub fn read_mode(&mut self) -> u32 {
		let read = u32::from(self.use_sync_mode)
			| u32::from(self.sync_mode) << 1
			| (self.reset_after as u32) << 3
			| u32::from(self.irq_at_target) << 4
			| u32::from(self.irq_at_overflow) << 5
			| u32::from(self.irq_repeat) << 6
			| u32::from(self.irq_pulse) << 7
			| u32::from(self.clock_src_raw) << 8
			| u32::from(self.irq) << 10
			| u32::from(self.reached_target) << 11
			| u32::from(self.reached_overflow) << 12;
		
		// bits reset after reading
		self.reached_target = false;
		self.reached_overflow = false;

		read
	}

	// resets counter
	pub fn write_mode(&mut self, write: u32, scheduler: &mut Scheduler) {
		self.use_sync_mode = write & 1 != 0;
		self.sync_mode = (write >> 1) as u8 & 3;
		self.reset_after = ResetMode::from_bits((write >> 3) & 1);
		self.irq_at_target = (write >> 4) & 1 != 0;
		self.irq_at_overflow = (write >> 5) & 1 != 0;
		self.irq_repeat = (write >> 6) & 1 != 0;
		self.irq_pulse = (write >> 7) & 1 != 0;
		self.clock_src_raw = (write >> 8) as u8 & 3;
		self.clock_src = self.get_clock_src(self.clock_src_raw);
		self.irq = true;
		self.reached_target = (write >> 11) & 1 != 0;
		self.reached_overflow = (write >> 2) & 1 != 0;

		self.counter = 0;

		trace!("clock src: {:?}", self.clock_src);

		if self.use_sync_mode {
			error!("using sync mode for Timer{}", self.timer_num);
		}

		self.reschedule_events(scheduler);
	}

	fn reschedule_events(&mut self, scheduler: &mut Scheduler) {
		// remove old events
		scheduler.remove_event(EventType::TimerTarget(self.timer_num));
		scheduler.remove_event(EventType::TimerOverflow(self.timer_num));

		// schedule new events
		if self.target != 0 {
			scheduler.schedule_event(SchedulerEvent::new(EventType::TimerTarget(self.timer_num)), self.convert_cycles(self.target));
		}

		let overflow_cycles_away = self.convert_cycles(0xFFFF - self.counter);
		scheduler.schedule_event(SchedulerEvent::new(EventType::TimerOverflow(self.timer_num)), overflow_cycles_away);
		self.overflow_cycles_away = overflow_cycles_away;
	}

	fn convert_cycles(&self, cycles: u16) -> u64 {	
		match self.clock_src {
			ClockSource::System => {
				return cycles as u64;
			},
			ClockSource::SystemDiv => {
				return (cycles as u64) * 8;
			}
			ClockSource::Hblank => {
				return (f64::from(cycles) * 3.2 * 853.0) as u64;
			}
			_ => todo!("{:?}", self.clock_src)
		}
	}

	fn get_clock_src(&self, clock_src: u8) -> ClockSource {
		match clock_src {
			0 => ClockSource::System,
			1 => match self.timer_num {
				0 => ClockSource::Dot,
				1 => ClockSource::Hblank,
				2 => ClockSource::System,
				_ => unreachable!(),
			},
			2 => match self.timer_num {
				0 | 1 => ClockSource::System,
				2 => ClockSource::SystemDiv,
				_ => unreachable!(),
			},
			3 => match self.timer_num {
				0 => ClockSource::Dot,
				1 => ClockSource::Hblank,
				2 => ClockSource::SystemDiv,
				_ => unreachable!(),
			}
			_ => unreachable!(),
		}
	}

	fn irq_src(&self) -> InterruptFlag {
		match self.timer_num {
			0 => InterruptFlag::Timer0,
			1 => InterruptFlag::Timer1,
			2 => InterruptFlag::Timer2,
			_ => unreachable!(),
		}
	}
}
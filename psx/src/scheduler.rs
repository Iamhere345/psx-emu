use std::{collections::BinaryHeap, i16};

use crate::{bus::Bus, interrupts::InterruptFlag, cdrom::CmdResponse};

const AUDIO_BUFFER_LEN: usize = 735 * 2; // 44100hz / 60hz * two channels (stereo)

#[derive(Clone, PartialEq)]
pub enum EventType {
	Vblank,
	SpuTick,
	TimerTarget(u8),
	TimerOverflow(u8),
	Sio0Irq,
	Sio0Rx(u8, bool),
	CdromCmd(CmdResponse),
	DmaIrq(u8),
}

#[derive(Clone)]
pub struct SchedulerEvent {
	pub event_type: EventType,
	pub cpu_timestamp: u64,
}

impl SchedulerEvent {
	pub fn new(ev_type: EventType) -> Self {
		Self {
			event_type: ev_type,
			cpu_timestamp: 0,
		}
	}
}

impl Ord for SchedulerEvent {
	fn cmp(&self, other: &Self) -> std::cmp::Ordering {
		self.cpu_timestamp.cmp(&other.cpu_timestamp).reverse() // reversed to turn the max heap into a min heap
	}
}

impl PartialOrd for SchedulerEvent {
	fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
		Some(self.cmp(other))
	}
}

impl Eq for SchedulerEvent {}

impl PartialEq for SchedulerEvent {
	fn eq(&self, other: &Self) -> bool {
		self.cpu_timestamp == other.cpu_timestamp
	}
}

pub struct Scheduler {
	event_queue: BinaryHeap<SchedulerEvent>,
	pub cpu_cycle_counter: u64,

	pub buffer_full: bool,
	audio_buffer: Vec<f32>,
	audio_callback: Box<dyn Fn(Vec<f32>)>,
}

impl Scheduler {
	pub fn new(audio_callback: Box<dyn Fn(Vec<f32>)>) -> Self {
		Self {
			event_queue: BinaryHeap::new(),
			cpu_cycle_counter: 0,

			audio_buffer: Vec::new(),
			buffer_full: false,
			audio_callback: audio_callback,
		}
	}

	pub fn schedule_event(&mut self, mut event: SchedulerEvent, cycles_away: u64) {
		event.cpu_timestamp = self.cpu_cycle_counter + cycles_away;
		self.event_queue.push(event);
	}

	pub fn remove_event(&mut self, event_type: EventType) {
		let mut events: Vec<SchedulerEvent> = self.event_queue.drain().collect();

		events.retain(|ev| ev.event_type != event_type);

		self.event_queue = BinaryHeap::from(events)
	}

	pub fn get_event(&self, event_type: EventType) -> Option<&SchedulerEvent> {
		for ev in self.event_queue.iter() {
			if ev.event_type == event_type {
				return Some(ev);
			}
		}

		None
	}

	pub fn event_cycles_away(&self, event: &SchedulerEvent) -> u64 {
		//trace!("counter: {} - timestamp: {} = {}", self.cpu_cycle_counter, event.cpu_timestamp, self.cpu_cycle_counter.saturating_sub(event.cpu_timestamp));
		event.cpu_timestamp.saturating_sub(self.cpu_cycle_counter)
	}

	pub fn next_event_ready(&self) -> bool {
		self.cpu_cycle_counter >= self.peek_event().cpu_timestamp
	}

	pub fn pop_event(&mut self) -> SchedulerEvent {
		self.event_queue.pop().expect("Scheduler ran out of events")
	}

	pub fn peek_event(&self) -> SchedulerEvent {
		self.event_queue.peek().expect("Scheduler ran out of events").clone()
	}

	pub fn tick_scheduler(&mut self, amount: u64) {
		self.cpu_cycle_counter += amount
	}

	pub fn handle_event(&mut self, event: SchedulerEvent, bus: &mut Bus) {
		match event.event_type {
			EventType::Vblank => {
				//log::info!("VBlank");
				bus.interrupts.raise_interrupt(InterruptFlag::Vblank);

				//log::info!("triggered: {}", bus.interrupts.triggered());

				self.schedule_event(SchedulerEvent::new(EventType::Vblank), 571212);
			},
			EventType::SpuTick => {
				let (sample_l, sample_r) = bus.spu.tick();

				// convert to f32 PCM sample in [-1, 1] range
				self.audio_buffer.push(f32::from(sample_r) / f32::from(i16::MAX));
				self.audio_buffer.push(f32::from(sample_l) / f32::from(i16::MAX));

				if self.audio_buffer.len() >= AUDIO_BUFFER_LEN {
					(self.audio_callback)(self.audio_buffer.clone());

					self.audio_buffer.clear();
					self.buffer_full = true;
				}

				self.schedule_event(SchedulerEvent::new(EventType::SpuTick), 768);
			}
			EventType::TimerTarget(timer) => {
				bus.timers.target_event(timer, self, &mut bus.interrupts);
			},
			EventType::TimerOverflow(timer) => {
				bus.timers.overflow_event(timer, self, &mut bus.interrupts);
			},
			EventType::Sio0Irq => {
				bus.sio0.irq_event(&mut bus.interrupts);
			},
			EventType::Sio0Rx(value, interrupt) => {
				bus.sio0.rx_event(self, value, interrupt);
			}
			EventType::CdromCmd(response) => {
				bus.cdrom.handle_cmd_response(response, self, &mut bus.interrupts);
			},
			EventType::DmaIrq(channel) => {
				bus.dma.raise_int(channel, &mut bus.interrupts);
			}
		}
	}

}
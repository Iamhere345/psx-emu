use std::collections::BinaryHeap;

use crate::{bus::Bus, interrupts::InterruptFlag};

#[derive(Clone, Copy, PartialEq)]
pub enum EventType {
	Vblank,
	TimerTarget(u8),
	TimerOverflow(u8),
}

#[derive(Clone, Copy)]
pub struct SchedulerEvent {
	pub event_type: EventType,
	pub cycles: u64,
	removed: bool,
}

impl SchedulerEvent {
	pub fn new(ev_type: EventType) -> Self {
		Self {
			event_type: ev_type,
			cycles: 0,
			removed: false,
		}
	}
}

impl Ord for SchedulerEvent {
	fn cmp(&self, other: &Self) -> std::cmp::Ordering {
		self.cycles.cmp(&other.cycles).reverse() // reversed to turn the max heap into a min heap
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
		self.cycles == other.cycles
	}
}

pub struct Scheduler {
	event_queue: BinaryHeap<SchedulerEvent>,
	pub cycles: u64,
}

impl Scheduler {
	pub fn new() -> Self {
		Self {
			event_queue: BinaryHeap::new(),
			cycles: 0,
		}
	}

	pub fn schedule_event(&mut self, mut event: SchedulerEvent, cycles_away: u64) {
		event.cycles = self.cycles + cycles_away;
		self.event_queue.push(event);
	}

	pub fn remove_event(&mut self, event_type: EventType) {
		let mut events: Vec<SchedulerEvent> = self.event_queue.drain().collect();

		for ev in events.iter_mut() {
			if ev.event_type == event_type {
				ev.removed = true;
			}
		}

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
		event.cycles.saturating_sub(self.cycles)
	}

	pub fn next_event(&mut self) -> Option<SchedulerEvent> {
		let mut next_ev = self.event_queue.pop();

		while next_ev.unwrap().removed {
			next_ev = self.event_queue.pop();
		}

		next_ev
	}

	pub fn tick_events(&mut self, amount: u64) {
		let mut events: Vec<SchedulerEvent> = self.event_queue.drain().collect();

		for ev in events.iter_mut() {
			ev.cycles = ev.cycles.saturating_sub(amount);
		}

		self.event_queue = BinaryHeap::from(events)
	}

	pub fn handle_event(&mut self, event: SchedulerEvent, bus: &mut Bus) {
		match event.event_type {
			EventType::Vblank => {
				//log::info!("VBlank");
				bus.interrupts.raise_interrupt(InterruptFlag::Vblank);

				//log::info!("triggered: {}", bus.interrupts.triggered());

				self.schedule_event(SchedulerEvent::new(EventType::Vblank), 571212);
			}
			EventType::TimerTarget(timer) => {
				bus.timers.target_event(timer, self, &mut bus.interrupts);
			},
			EventType::TimerOverflow(timer) => {
				bus.timers.overflow_event(timer, self, &mut bus.interrupts);
			},
		}
	}

}
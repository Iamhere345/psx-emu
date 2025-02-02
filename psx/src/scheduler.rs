use std::collections::BinaryHeap;

use crate::{bus::Bus, interrupts::InterruptFlag};

#[derive(Clone, Copy, PartialEq)]
pub enum EventType {
	Vblank,
}

#[derive(Clone, Copy)]
pub struct SchedulerEvent {
	pub event_type: EventType,
	pub cycles_away: u64,
}

impl SchedulerEvent {
	pub fn new(ev_type: EventType, cycles_away: u64) -> Self {
		Self {
			event_type: ev_type,
			cycles_away: cycles_away
		}
	}
}

impl Ord for SchedulerEvent {
	fn cmp(&self, other: &Self) -> std::cmp::Ordering {
		self.cycles_away.cmp(&other.cycles_away).reverse() // reversed to turn the max heap into a min heap
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
		self.cycles_away == other.cycles_away
	}
}

pub struct Scheduler {
	event_queue: BinaryHeap<SchedulerEvent>,
}

impl Scheduler {
	pub fn new() -> Self {
		Self {
			event_queue: BinaryHeap::new(),
		}
	}

	pub fn schedule_event(&mut self, event: SchedulerEvent) {
		self.event_queue.push(event);
	}

	pub fn next_event(&mut self) -> Option<SchedulerEvent> {
		self.event_queue.pop()
	}

	pub fn tick_events(&mut self, amount: u64) {
		let mut events: Vec<SchedulerEvent> = self.event_queue.drain().collect();

		for ev in events.iter_mut() {
			ev.cycles_away = ev.cycles_away.saturating_sub(amount);
		}

		self.event_queue = BinaryHeap::from(events)
	}

	pub fn handle_event(&mut self, event: SchedulerEvent, bus: &mut Bus) {
		match event.event_type {
			EventType::Vblank => {
				//log::info!("VBlank");
				bus.interrupts.raise_interrupt(InterruptFlag::Vblank);

				//log::info!("triggered: {}", bus.interrupts.triggered());

				self.schedule_event(SchedulerEvent::new(EventType::Vblank, 571212));
			}
		}
	}

}
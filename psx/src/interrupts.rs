use log::*;

pub enum InterruptFlag {
	Vblank 		= 1 << 0,
	Gpu 		= 1 << 1,
	Cdrom 		= 1 << 2,
	Dma 		= 1 << 3,
	Timer0 		= 1 << 4,
	Timer1 		= 1 << 5,
	Timer2 		= 1 << 6,
	Controller 	= 1 << 7,
	Sio 		= 1 << 8,
	Spu 		= 1 << 9,
}

pub struct Interrupts {
	reg_status: u32,
	reg_mask: u32,
}

impl Interrupts {
	pub fn new() -> Self {
		Self {
			reg_status: 0,
			reg_mask: 0
		}
	}

	pub fn read32(&self, addr: u32) -> u32 {
		
		let read = match addr {
			0x1F801070 => self.reg_status,
			0x1F801074 => self.reg_mask,
			_ => unreachable!("{addr:X}")
		};
		
		trace!("IRQ read [0x{addr:X}] 0x{read:X}");

		read
	}

	pub fn write32(&mut self, addr: u32, write: u32) {
		trace!("IRQ write [0x{addr:X}] 0x{write:X}");

		match addr {
			0x1F801070 => self.ack_interrupt(write),
			0x1F801074 => self.reg_mask = write,
			_ => unreachable!()
		}
	}

	fn ack_interrupt(&mut self, ack: u32) {
		self.reg_status &= ack;
	}

	pub fn raise_interrupt(&mut self, int: InterruptFlag) {
		self.reg_status |= int as u32;
	}

	pub fn triggered(&self) -> bool {
		self.reg_status & self.reg_mask != 0
	}
}
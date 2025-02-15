use std::collections::VecDeque;

use log::*;

use crate::interrupts::Interrupts;

const AVG_CYCLES: u64 = 0xC4E1;  

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

	fn raise_interrupt(&mut self, int: u8) {
		self.int_flags = int & 0x1F;
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
		self.int_flags &= !(int & 0x1F);

		if (int >> 6) & 1 != 0 {
			// bit 6 clears param FIFO
			params.clear();
		}
	}
}

pub struct Cdrom {
	params_fifo: VecDeque<u8>,
	result_fifo: VecDeque<u8>,
	bank: u8,

	int_regs: CdromInterrupts
}

impl Cdrom {
	pub fn new() -> Self {
		Self {
			params_fifo: VecDeque::new(),
			result_fifo: VecDeque::new(),
			bank: 0,

			int_regs: CdromInterrupts::new(),
		}
	}

	pub fn read8(&mut self, addr: u32) -> u8 {
		let reg = addr & 0xF;

		trace!("[{}][0x{addr:X}] CDROM read", self.bank);

		match reg {
			// status register
			0 => self.bank | 0x38,
			1 => self.result_fifo.pop_front().or(Some(0)).unwrap(),
			2 => { warn!("Unhandled read to RDDATA"); 0 },
			3 => match self.bank {
				0 | 2 => self.int_regs.read_mask(),
				1 | 3 => self.int_regs.read_flags(),
				_ => unreachable!(),
			}
			
			_ => todo!("CDROM read [0x{addr:X}][{}]", self.bank),
		}
	}

	pub fn write8(&mut self, addr: u32, write: u8, interrupts: &mut Interrupts) {
		let reg = addr & 0xF;

		trace!("[{}][0x{addr:X}] CDROM write 0x{write:X}", self.bank);

		match self.bank {
			0 => match reg {
				0 => self.bank = write & 3,
				1 => self.exec_cmd(write, interrupts),
				2 => self.params_fifo.push_back(write),
				_ => todo!("CDROM write [0x{addr:X}][{}] 0x{write:X}", self.bank),
			},
			1 => match reg {
				0 => self.bank = write & 3,
				2 => self.int_regs.write_mask(write),
				3 => self.int_regs.ack_interrupt(write, &mut self.params_fifo),
				_ => todo!("CDROM write [0x{addr:X}][{}] 0x{write:X}", self.bank),
			},
			2 => match reg {
				0 => self.bank = write & 3,
				_ => todo!("CDROM write [0x{addr:X}][{}] 0x{write:X}", self.bank),
			},
			3 => match reg {
				0 => self.bank = write & 3,
				_ => todo!("CDROM write [0x{addr:X}][{}] 0x{write:X}", self.bank),
			},
			
			_ => unimplemented!("CDROM bank {}", self.bank),
		}
	}

	fn exec_cmd(&mut self, cmd: u8, irq: &mut Interrupts) {
		match cmd {
			0x1 => {
				// status hardcoded to shell open
				self.result_fifo.push_back(0x10);

				self.int_regs.raise_interrupt(3);
				irq.raise_interrupt(crate::interrupts::InterruptFlag::Cdrom);
			}
			0x19 => if let Some(sub_cmd) = self.params_fifo.pop_front() {
				match sub_cmd {
					0x20 => {
						for x in [0x94, 0x09, 0x19, 0xC0].iter() {
							self.result_fifo.push_back(*x as u8);
						}

						self.int_regs.raise_interrupt(3);
						irq.raise_interrupt(crate::interrupts::InterruptFlag::Cdrom);
					}
					_ => todo!("subcommand 0x{cmd:X}")
				}
			}
			_ => todo!("cmd 0x{cmd:X}")
		}
	}
}
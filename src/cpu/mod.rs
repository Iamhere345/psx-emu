use std::mem;

use crate::bus::Bus;
use cop0::Cop0;
use instructions::Instruction;

mod instructions;
mod cop0;

struct Registers {
	gpr: [u32; 32],
	hi: u32,
	lo: u32,

	delayed_load: (u32, u32),
	delayed_load_next: (u32, u32),
}

impl Registers {
	pub fn new() -> Self {
		Self {
			gpr: [0; 32],
			hi: 0,
			lo: 0,

			delayed_load: (0, 0),
			delayed_load_next: (0, 0),
		}
	}
	
	pub fn read_gpr(&self, register: u32) -> u32 {
		self.gpr[register as usize]
	}
	
	pub fn write_gpr(&mut self, register: u32, write: u32) {
		if register == 0 {
			return;
		}
		
		self.gpr[register as usize] = write;

		if self.delayed_load.0 == register {
			self.delayed_load = (0, 0);
		}
	}

	pub fn write_gpr_delayed(&mut self, register: u32, write: u32) {
		if register == 0 {
			return;
		}

		self.delayed_load_next = (register, write);

		if self.delayed_load.0 == register {
			self.delayed_load = (0, 0);
		}
	}

	pub fn process_delayed_loads(&mut self) {

		let (reg, write) = self.delayed_load;
		self.gpr[reg as usize] = write;

		self.delayed_load = mem::take(&mut self.delayed_load_next);

	}

}

pub struct R3000 {
	registers: Registers,
	pc: u32,

	cop0: Cop0,

	delayed_branch: Option<u32>,
}

impl R3000 {
	pub fn new() -> Self {
		Self {
			registers: Registers::new(),
			pc: 0xbfc00000, // start of BIOS
			
			cop0: Cop0::new(),

			delayed_branch: None,
		}
	}

	pub fn run_instruction(&mut self, bus: &mut Bus) {

		self.check_tty_putchar();

		let instruction = bus.read32(self.pc);

		let next_pc = match self.delayed_branch.take() {
			Some(addr) => addr,
			None => self.pc.wrapping_add(4),
		};

		self.decode_and_exec(Instruction::from_u32(instruction), bus);

		self.registers.process_delayed_loads();

		self.pc = next_pc;
	}

	fn check_tty_putchar(&self) {
		let pc = self.pc & 0x1FFFFFFF;

		if (pc == 0xA0 && self.registers.read_gpr(9) == 0x3C) || (pc == 0xB0 && self.registers.read_gpr(9) == 0x3D) {
			let char = self.registers.read_gpr(4) as u8 as char;

			print!("{char}");
		}
	}
}
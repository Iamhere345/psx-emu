use std::mem;

use crate::bus::Bus;
use cop0::*;
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

	pub fn read_gpr_lwl_lwr(&mut self, register: u32) -> u32 {
		let (delayed_reg, delayed_val) = self.delayed_load;
		if register == delayed_reg { delayed_val } else { self.gpr[register as usize] }
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
	in_delay_slot: bool,
}

impl R3000 {
	pub fn new() -> Self {
		Self {
			registers: Registers::new(),
			pc: 0xbfc00000, // start of BIOS
			
			cop0: Cop0::new(),

			delayed_branch: None,
			in_delay_slot: false,
		}
	}

	pub fn run_instruction(&mut self, bus: &mut Bus) {

		self.check_tty_putchar();

		if self.pc % 4 != 0 {
			self.exception(Exception::AddrLoadError);

			self.registers.process_delayed_loads();
			return;
		}

		let instruction = bus.read32(self.pc);

		let (next_pc, in_delay_slot) = match self.delayed_branch.take() {
			Some(addr) => (addr, true),
			None => (self.pc.wrapping_add(4), false),
		};

		self.in_delay_slot = in_delay_slot;
		
		self.decode_and_exec(Instruction::from_u32(instruction), bus);
		
		self.registers.process_delayed_loads();

		self.pc = next_pc;

	}

	fn exception(&mut self, exception: Exception) {

		// TODO: other cause fields
		self.cop0.reg_cause = (exception as u32) << 2;

		self.cop0.reg_epc = match self.in_delay_slot {
			true => {self.cop0.reg_cause |= 1 << 31; self.pc.wrapping_sub(4)},
			false => self.pc
		};

		// SR exception stack (3 user/mode entries)
		// bits 5:0
		let mode = self.cop0.reg_sr & 0x3F;
		self.cop0.reg_sr &= !0x3F;
		self.cop0.reg_sr = (mode << 2) & 0x3F;

		self.pc = match self.cop0.reg_sr & (1 << 22) != 0  {
			true => 0xBFC00180,
			false => 0x80000080,
		};

		self.delayed_branch = None;

	}

	fn check_tty_putchar(&self) {
		let pc = self.pc & 0x1FFFFFFF;

		if (pc == 0xA0 && self.registers.read_gpr(9) == 0x3C) || (pc == 0xB0 && self.registers.read_gpr(9) == 0x3D) {
			let char = self.registers.read_gpr(4) as u8 as char;

			print!("{char}");
		}
	}
}
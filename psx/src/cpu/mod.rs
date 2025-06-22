use std::{fmt::Display, mem};

use log::*;

use crate::{bus::Bus, scheduler::Scheduler};
use crate::kernel::KernelFunction;
use cop0::*;
use instructions::Instruction;

pub mod instructions;
mod cop0;

#[derive(Debug)]
pub struct Registers {
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

impl Display for Registers {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		let mut regs: Vec<String> = Vec::new();

		for (i, reg) in self.gpr.iter().enumerate() {
			regs.push(format!("$r{i}: 0x{reg:08X}, "));
		}

		regs.push(format!("$hi: 0x{:08X}, ", self.hi));
		regs.push(format!("$lo: 0x{:08X}", self.lo));

		write!(f, "{}", regs.concat())
	}
}

pub struct R3000 {
	pub registers: Registers,
	pub pc: u32,
	last_pc: u32,
	last_instruction: u32,

	cop0: Cop0,

	delayed_branch: Option<u32>,
	in_delay_slot: bool,
	exception: bool,

	pub tty_buf: String,
	pub kernel_log: Vec<String>,

	pub debug: bool,
}

impl R3000 {
	pub fn new() -> Self {
		Self {
			registers: Registers::new(),
			pc: 0xBFC00000, // start of BIOS
			last_pc: 0,
			last_instruction: 0,
			
			cop0: Cop0::new(),

			delayed_branch: None,
			in_delay_slot: false,
			exception: false,

			tty_buf: String::new(),
			kernel_log: Vec::new(),

			debug: false,
		}
	}

	pub fn run_instruction(&mut self, bus: &mut Bus, scheduler: &mut Scheduler) {

		self.check_tty_putchar();

		if self.pc % 4 != 0 {
			self.exception(Exception::AddrLoadError);
			self.cop0.reg_badvaddr = self.pc;

			self.registers.process_delayed_loads();
			return;
		}

		// check if last jump was to a kernel function
		if self.in_delay_slot {
			self.log_kernel_func();
		}

		let instruction = bus.read32(self.pc, scheduler);
		self.last_instruction = instruction;

		let (next_pc, in_delay_slot) = match self.delayed_branch.take() {
			Some(addr) => (addr, true),
			None => (self.pc.wrapping_add(4), false),
		};

		self.in_delay_slot = in_delay_slot;

		self.cop0.reg_cause.set_hw_interrupt(bus.interrupts.triggered());
		if self.cop0.interrupt_pending() && self.cop0.reg_sr.cur_int_enable {
			//log::trace!("interrupt (status: 0b{:b})", bus.interrupts.read32(0x1F801070));
			// TODO GTE instructions need to be run before the interrupt is serviced
			self.exception(Exception::Interrupt);
		} else {
			self.decode_and_exec(Instruction::from_u32(instruction), bus, scheduler);
		}

		/* print!("{:08x} {instruction:08x} ", self.pc);

		for reg in self.registers.gpr {
			print!("{reg:08x} ")
		}

		print!("\n"); */
		if self.debug {
			//log::trace!("[0x{:X}] {} (0x{instruction:X}) registers: {:X?}", self.pc, self.dissasemble(Instruction::from_u32(instruction), bus, scheduler), self.registers);
		}

		self.registers.process_delayed_loads();

		if !self.exception {
			self.last_pc = self.pc;
			self.pc = next_pc;
		} else {
			self.exception = false;
		}

	}

	fn exception(&mut self, exception: Exception) {
		self.cop0.reg_cause.exception = exception;
		
		self.cop0.reg_epc = match self.in_delay_slot {
			true => {self.cop0.reg_cause.branch_delay = true; self.pc.wrapping_sub(4)},
			false => {self.cop0.reg_cause.branch_delay = false; self.pc}
		};
		
		trace!("exception triggered: {exception:?}, in delay slot: {}, epc: 0x{:X} badvaddr: 0x{:X}", self.in_delay_slot, self.cop0.reg_epc, self.cop0.reg_badvaddr);

		self.cop0.reg_sr.push_exception();

		self.pc = match self.cop0.reg_sr.boot_exception_vector {
			true => 0xBFC00180,
			false => 0x80000080,
		};

		self.delayed_branch = None;
		self.exception = true;

	}

	fn check_tty_putchar(&mut self) {
		let pc = self.pc & 0x1FFFFFFF;

		if (pc == 0xA0 && self.registers.read_gpr(9) == 0x3C) || (pc == 0xB0 && self.registers.read_gpr(9) == 0x3D) {
			let char = self.registers.read_gpr(4) as u8 as char;

			print!("{char}");

			self.tty_buf.push(char);
		}
	}

	fn log_kernel_func(&mut self) {
		let kernel_func = match self.pc {
			0xA0 => {
				let num = self.registers.read_gpr(9);
				KernelFunction::a_func(num)
			},
			0xB0 => {
				let num = self.registers.read_gpr(9);
				KernelFunction::b_func(num)
			},
			0xC0 => {
				let num = self.registers.read_gpr(9);
				KernelFunction::c_func(num)
			},

			_ => return
		};

		match kernel_func {
			KernelFunction::ReturnFromException | KernelFunction::Rand
				| KernelFunction::TestEvent | KernelFunction::Unknown => return,
			_ => {}
		}

		let num_args = kernel_func.num_args();

		let args = (4..4 + num_args).into_iter()
			.map(|i| self.registers.read_gpr(i as u32))
			.collect::<Vec<u32>>()
			.into_iter()
			.map(|reg| format!("0x{reg:08X}"))
			.collect::<Vec<_>>()
			.join(", ");

		if kernel_func == KernelFunction::PutChar {
			self.kernel_log.push(format!("{kernel_func:?}('{}')", char::from_u32(self.registers.read_gpr(4)).unwrap_or('?')));
		} else {
			self.kernel_log.push(format!("{kernel_func:?}({args})"));
		}

	}
}

impl Drop for R3000 {
	fn drop(&mut self) {
		println!("CPU dropped. Last CPU state:\n[0x{:08X}][0x{:08X}] {}\nregs: {}", self.last_pc, self.last_instruction, Instruction::from_u32(self.last_instruction).dissasemble_str(), self.registers)
	}
}
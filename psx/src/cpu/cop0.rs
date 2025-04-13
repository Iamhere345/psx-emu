#![allow(dead_code)]

use log::*;

#[derive(Debug, Clone, Copy, Default)]
pub enum Exception {
	#[default]
	Interrupt = 0x00,
	AddrLoadError = 0x04,
	AddrStoreError = 0x05,
	BusFetchError = 0x06,
	BusLoadStoreError = 0x07,
	Syscall = 0x08,
	Breakpoint = 0x09,
	ReservedInstruction = 0xA,
	CopUnusable = 0xB,
	ArithmeticOverflow = 0xC,
}

#[derive(Default)]
pub struct StatusRegister {
	pub cur_int_enable: bool,
	pub cur_usr_mode: bool,
	pub prev_int_enable: bool,
	pub prev_usr_mode: bool,
	pub old_int_enable: bool,
	pub old_usr_mode: bool,

	pub interrupt_mask: u8,
	pub boot_exception_vector: bool,

	pub isolate_cache: bool,

	pub cop0_enable: bool,
	pub cop1_enable: bool,
	pub cop2_enable: bool,
	pub cop3_enable: bool,
}

impl StatusRegister {
	pub fn read(&self) -> u32 {
		u32::from(self.cur_int_enable)
			| (u32::from(self.cur_usr_mode) << 1)
			| (u32::from(self.prev_int_enable) << 2)
			| (u32::from(self.prev_usr_mode) << 3)
			| (u32::from(self.old_int_enable) << 4)
			| (u32::from(self.old_usr_mode) << 5)
			| (u32::from(self.interrupt_mask) << 8)
			| (u32::from(self.isolate_cache) << 16)
			| (u32::from(self.boot_exception_vector) << 22)
			| (u32::from(self.cop0_enable) << 28)
			| (u32::from(self.cop1_enable) << 29)
			| (u32::from(self.cop2_enable) << 30)
			| (u32::from(self.cop3_enable) << 31)
	}

	pub fn write(&mut self, write: u32) {
		self.cur_int_enable = (write >> 0) & 1 != 0;
		self.cur_usr_mode = (write >> 1) & 1 != 0;
		self.prev_int_enable = (write >> 2) & 1 != 0;
		self.prev_usr_mode = (write >> 3) & 1 != 0;
		self.old_int_enable = (write >> 4) & 1 != 0;
		self.old_usr_mode = (write >> 5) & 1 != 0;

		trace!("set int mask: 0b{:b}", (write >> 8) as u8);
		self.interrupt_mask = (write >> 8) as u8;
		self.boot_exception_vector = (write >> 22) & 1 != 0;

		self.isolate_cache = (write >> 16) & 1 != 0;

		self.cop0_enable = (write >> 28) & 1 != 0;
		self.cop1_enable = (write >> 29) & 1 != 0;
		self.cop2_enable = (write >> 30) & 1 != 0;
		self.cop3_enable = (write >> 31) & 1 != 0;
	}

	pub fn push_exception(&mut self) {
		self.old_int_enable = self.prev_int_enable;
		self.old_usr_mode = self.prev_usr_mode;
		
		self.prev_int_enable = self.cur_int_enable;
		self.prev_usr_mode = self.cur_usr_mode;

		self.cur_int_enable = false;
		self.cur_usr_mode = false;
	}

	// used by RFE
	pub fn pop_exception(&mut self) {
		self.cur_int_enable = self.prev_int_enable;
		self.cur_usr_mode = self.prev_usr_mode;

		self.prev_int_enable = self.old_int_enable;
		self.prev_usr_mode = self.old_usr_mode;
	}

}

#[derive(Default)]
pub struct CauseRegister {
	pub exception: Exception,
	pub interrupt_pending: u8,
	pub cop_num: u8,
	pub branch_delay: bool,
}

impl CauseRegister {
	fn read(&self) -> u32 {
		((self.exception as u32) << 2)
			| (u32::from(self.interrupt_pending) << 8)
			| (u32::from(self.cop_num) << 28)
			| (u32::from(self.branch_delay) << 31)
	}

	fn write(&mut self, write: u32) {
		// only bits 8-9 of CAUSE are writable
		self.interrupt_pending = (self.interrupt_pending & 0xFC) | ((write >> 8) & 3) as u8;
	}

	pub fn set_hw_interrupt(&mut self, set: bool) {
		self.interrupt_pending = (self.interrupt_pending & 3) | (u8::from(set) << 2);
	}
}

pub struct Cop0 {
	pub reg_bpc: u32,				// Breakpoint on execute (R/W)
	pub reg_bda: u32,				// Breakpoint on data access (R/W)
	pub reg_jumpdest: u32,			// Randomly memorized jump address (R)
	pub reg_dcic: u32,				// Breakpoint control (R/W)
	pub reg_badvaddr: u32,			// Bad Virtual Address (R)
	pub reg_bdam: u32,				// Data Access breakpoint mask (R/W)
	pub reg_bpcm: u32,				// Execute breakpoint mask (R/W)
	pub reg_sr: StatusRegister,		// System status register (R/W)
	pub reg_cause: CauseRegister,	// Describes the most recently recognised exception (R)
	pub reg_epc: u32,				// Return Address from Trap (R)
	pub reg_prid: u32,				// Processor ID (R)
}

impl Cop0 {
	pub fn new() -> Self {
		Self {
			reg_bpc: 0,
			reg_bda: 0,
			reg_jumpdest: 0,
			reg_dcic: 0,
			reg_badvaddr: 0,
			reg_bdam: 0,
			reg_bpcm: 0,
			reg_sr: StatusRegister::default(),
			reg_cause: CauseRegister::default(),
			reg_epc: 0,
			reg_prid: 0x2,
		}
	}

	pub fn read_reg(&self, reg_index: u32) -> u32 {
		match reg_index {
			3 => self.reg_bpc,
			5 => self.reg_bda,
			6 => self.reg_jumpdest,
			7 => self.reg_dcic,
			8 => self.reg_badvaddr,
			9 => self.reg_bdam,
			11 => self.reg_bpcm,
			12 => self.reg_sr.read(),
			13 => self.reg_cause.read(),
			14 => self.reg_epc,
			15 => self.reg_prid,
			16 ..= 31 => 0,

			_ => panic!("unimplemented or unsupported cop0 register read cop0r{}", reg_index)
		}
	}

	pub fn write_reg(&mut self, reg_index: u32, write: u32) {
		
		if reg_index == 13 {
			log::info!("write 0x{:X}", write);
		}

		match reg_index {
			3 => self.reg_bpc = write,
			5 => self.reg_bda = write,
			6 => {},
			7 => self.reg_dcic = write,
			8 => {},
			9 => self.reg_bdam = write,
			11 => self.reg_bpcm = write,
			12 => self.reg_sr.write(write),
			13 => self.reg_cause.write(write),
			14 => {},
			15 => {},
			16 ..= 31 => {},

			_ => panic!("unimplemented or unsupported cop0 register read cop0r{}", reg_index)
		}
	}

	pub fn interrupt_pending(&self) -> bool {
		self.reg_cause.interrupt_pending & self.reg_sr.interrupt_mask != 0
	}
}
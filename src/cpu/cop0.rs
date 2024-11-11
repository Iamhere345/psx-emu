#![allow(dead_code)]

#[derive(Debug, Clone, Copy)]
pub enum Exception {
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

pub struct Cop0 {
	pub reg_bpc: u32,		// Breakpoint on execute (R/W)
	pub reg_bda: u32,		// Breakpoint on data access (R/W)
	pub reg_jumpdest: u32,	// Randomly memorized jump address (R)
	pub reg_dcic: u32,		// Breakpoint control (R/W)
	pub reg_badvaddr: u32,	// Bad Virtual Address (R)
	pub reg_bdam: u32,		// Data Access breakpoint mask (R/W)
	pub reg_bpcm: u32,		// Execute breakpoint mask (R/W)
	pub reg_sr: u32,		// System status register (R/W)
	pub reg_cause: u32,		// Describes the most recently recognised exception (R)
	pub reg_epc: u32,		// Return Address from Trap (R)
	pub reg_prid: u32,		// Processor ID (R)
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
			reg_sr: 0,
			reg_cause: 0,
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
			12 => self.reg_sr,
			13 => self.reg_cause,
			14 => self.reg_epc,
			15 => self.reg_prid,
			16 ..= 31 => 0,

			_ => panic!("unimplemented or unsupported cop0 register read cop0r{}", reg_index)
		}
	}

	pub fn write_reg(&mut self, reg_index: u32, write: u32) {
		match reg_index {
			3 => self.reg_bpc = write,
			5 => self.reg_bda = write,
			6 => {},
			7 => self.reg_dcic = write,
			8 => {},
			9 => self.reg_bdam = write,
			11 => self.reg_bpcm = write,
			12 => self.reg_sr = write,
			13 => {},
			14 => {},
			15 => {},
			16 ..= 31 => {},

			_ => panic!("unimplemented or unsupported cop0 register read cop0r{}", reg_index)
		}
	}
}
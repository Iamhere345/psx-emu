#![allow(unused)]
use log::*;

use crate::{bus::Bus, scheduler::Scheduler};

use super::{Exception, R3000};

pub enum InstrField {
	Reg(u32),
	Tgt(u32),
	Imm(u32),
	Shamt(u32),
	Addr(u32, u32)
}

#[derive(Clone, Copy)]
pub struct Instruction {
	raw: u32,
}

impl Instruction {
	pub fn from_u32(instr: u32) -> Self {
		Self { raw: instr }
	}

	pub fn opcode(&self) -> u32 {
		self.raw >> 26
	}

	pub fn cop_opcode(&self) -> u32 {
		(self.raw >> 21) & 0x1F
	}

	pub fn cop_num(&self) -> u32 {
		(self.raw >> 26) & 3
	}

	pub fn reg_src(&self) -> u32 {
		(self.raw >> 21) & 0x1F
	}

	pub fn reg_tgt(&self) -> u32 {
		(self.raw >> 16) & 0x1F
	}

	pub fn reg_dst(&self) -> u32 {
		(self.raw >> 11) & 0x1F
	}

	pub fn imm16(&self) -> u32 {
		self.raw & 0xFFFF
	}

	// sign-extended version of imm16
	pub fn imm16_se(&self) -> u32 {
		let imm16 = (self.raw & 0xFFFF) as i16;

		imm16 as u32
	}

	pub fn imm26(&self) -> u32 {
		self.raw & 0x3FFFFFF
	}

	pub fn shamt(&self) -> u32 {
		(self.raw >> 6) & 0x1F
	}

	pub fn funct(&self) -> u32 {
		self.raw & 0x3F
	}

	pub fn dissasemble(&self) -> (String, Vec<InstrField>) {

		macro_rules! rd_rs_rt {
			() => {
				vec![InstrField::Reg(self.reg_dst()), InstrField::Reg(self.reg_src()), InstrField::Reg(self.reg_tgt())]
			};
		}

		macro_rules! rt_rs_imm {
			() => {
				vec![InstrField::Reg(self.reg_tgt()), InstrField::Reg(self.reg_src()), InstrField::Imm(self.imm16())]
			};
		}

		macro_rules! rt_rs_imm_se {
			() => {
				vec![InstrField::Reg(self.reg_tgt()), InstrField::Reg(self.reg_src()), InstrField::Imm(self.imm16_se())]
			};
		}

		macro_rules! rs_imm_se {
			() => {
				vec![InstrField::Reg(self.reg_src()), InstrField::Imm(self.imm16_se())]
			};
		}

		macro_rules! rt_rd {
			() => {
				vec![InstrField::Reg(self.reg_tgt()), InstrField::Reg(self.reg_dst())]
			};
		}

		macro_rules! rs_rt {
			() => {
				vec![InstrField::Reg(self.reg_src()), InstrField::Reg(self.reg_tgt())]
			};
		}

		macro_rules! rt_addr {
			() => {
				vec![InstrField::Reg(self.reg_tgt()), InstrField::Addr(self.imm16_se() << 2, self.reg_src())]
			};
		}

		match self.opcode() {

			0x00 => match self.funct() {
				0x00 => ("sll".to_string(), vec![InstrField::Reg(self.reg_dst()), InstrField::Reg(self.reg_tgt()), InstrField::Shamt(self.shamt())]),
				0x02 => ("srl".to_string(), vec![InstrField::Reg(self.reg_dst()), InstrField::Reg(self.reg_tgt()), InstrField::Shamt(self.shamt())]),
				0x03 => ("sra".to_string(), vec![InstrField::Reg(self.reg_dst()), InstrField::Reg(self.reg_tgt()), InstrField::Shamt(self.shamt())]),
				0x04 => ("sllv".to_string(), vec![InstrField::Reg(self.reg_dst()), InstrField::Reg(self.reg_tgt()), InstrField::Reg(self.reg_src())]),
				0x06 => ("srlv".to_string(), vec![InstrField::Reg(self.reg_dst()), InstrField::Reg(self.reg_tgt()), InstrField::Reg(self.reg_src())]),
				0x07 => ("srav".to_string(), vec![InstrField::Reg(self.reg_dst()), InstrField::Reg(self.reg_tgt()), InstrField::Reg(self.reg_src())]),
				0x08 => ("jr".to_string(), vec![InstrField::Reg(self.reg_src())]),
				0x09 => ("jalr".to_string(), vec![InstrField::Reg(self.reg_dst()), InstrField::Reg(self.reg_src())]),
				0x0C => ("syscall".to_string(), vec![]),
				0x0D => ("break".to_string(), vec![]),
				0x10 => ("mfhi".to_string(), vec![InstrField::Reg(self.reg_dst())]),
				0x11 => ("mthi".to_string(), vec![InstrField::Reg(self.reg_src())]),
				0x12 => ("mflo".to_string(), vec![InstrField::Reg(self.reg_dst())]),
				0x13 => ("mtlo".to_string(), vec![InstrField::Reg(self.reg_src())]),
				0x18 => ("mult".to_string(), rs_rt!()),
				0x19 => ("multu".to_string(), rs_rt!()),
				0x1A => ("div".to_string(), rs_rt!()),
				0x1B => ("divu".to_string(), rs_rt!()),
				0x20 => ("add".to_string(), rd_rs_rt!()),
				0x21 => ("addu".to_string(), rd_rs_rt!()),
				0x22 => ("sub".to_string(), rd_rs_rt!()),
				0x23 => ("subu".to_string(), rd_rs_rt!()),
				0x24 => ("and".to_string(), rd_rs_rt!()),
				0x25 => ("or".to_string(), rd_rs_rt!()),
				0x26 => ("xor".to_string(), rd_rs_rt!()),
				0x27 => ("nor".to_string(), rd_rs_rt!()),
				0x2A => ("slt".to_string(), rd_rs_rt!()),
				0x2B => ("sltu".to_string(), rd_rs_rt!()),

				_ => ("illegal".to_string(), vec![]),
			}

			0x01 => ("bcondz".to_string(), rs_imm_se!()),

			0x02 => ("j".to_string(), vec![InstrField::Tgt(self.imm26() << 2)]),
			0x03 => ("jal".to_string(), vec![InstrField::Tgt(self.imm26() << 2)]),
			0x04 => ("beq".to_string(), vec![InstrField::Reg(self.reg_src()), InstrField::Reg(self.reg_tgt()), InstrField::Tgt(self.imm16_se() << 2)]),
			0x05 => ("bne".to_string(), vec![InstrField::Reg(self.reg_src()), InstrField::Reg(self.reg_tgt()), InstrField::Tgt(self.imm16_se() << 2)]),
			0x06 => ("blez".to_string(), rs_imm_se!()),
			0x07 => ("bgtz".to_string(), rs_imm_se!()),
			0x08 => ("addi".to_string(), rt_rs_imm_se!()),
			0x09 => ("addiu".to_string(), rt_rs_imm_se!()),
			0x0A => ("slti".to_string(), rt_rs_imm_se!()),
			0x0B => ("sltiu".to_string(), rt_rs_imm_se!()),
			0x0C => ("andi".to_string(), rt_rs_imm!()),
			0x0D => ("ori".to_string(), rt_rs_imm!()),
			0x0E => ("xori".to_string(), rt_rs_imm!()),
			0x0F => ("lui".to_string(), vec![InstrField::Reg(self.reg_tgt()), InstrField::Imm(self.imm16() << 16)]),

			0x10 => match self.cop_opcode() {
				0x00 => ("mfc".to_string(), rt_rd!()),
				0x02 => ("cfc".to_string(), rt_rd!()),
				0x04 => ("mtc".to_string(), rt_rd!()),
				0x06 => ("ctc".to_string(), rt_rd!()),
				0x10 => ("rfe".to_string(), vec![]),
				_ => ("illegal".to_string(), vec![]),
			}

			0x11 => ("copn".to_string(), vec![]),
			0x12 => ("gte".to_string(), vec![]),
			0x13 => ("copn".to_string(), vec![]),

			0x20 => ("lb".to_string(), rt_addr!()),
			0x21 => ("lh".to_string(), rt_addr!()),
			0x22 => ("lwl".to_string(), rt_addr!()),
			0x23 => ("lw".to_string(), rt_addr!()),
			0x24 => ("lbu".to_string(), rt_addr!()),
			0x25 => ("lhu".to_string(), rt_addr!()),
			0x26 => ("lwr".to_string(), rt_addr!()),
			0x28 => ("sb".to_string(), rt_addr!()),
			0x29 => ("sh".to_string(), rt_addr!()),
			0x2A => ("swl".to_string(), rt_addr!()),
			0x2B => ("sw".to_string(), rt_addr!()),
			0x2E => ("swr".to_string(), rt_addr!()),

			0x30 => ("lwcn".to_string(), rt_addr!()),
			0x31 => ("lwcn".to_string(), rt_addr!()),
			0x32 => ("lwc_gte".to_string(), rt_addr!()),
			0x33 => ("lwcn".to_string(), rt_addr!()),
			0x38 => ("swcn".to_string(), rt_addr!()),
			0x39 => ("swcn".to_string(), rt_addr!()),
			0x3A => ("swc_gte".to_string(), rt_addr!()),
			0x3B => ("swcn".to_string(), rt_addr!()),

			_ => ("illegal".to_string(), vec![]),
		}

	}

	pub fn dissasemble_str(&self) -> String {
		let (mut mnemonic, fields) = self.dissasemble();

		let mut first_field = true;

		for field in fields {

			if !first_field {
				mnemonic.push_str(", ");
			} else {
				mnemonic.push_str(" ");

				first_field = false;
			}

			match field {
				InstrField::Reg(reg) => {
					mnemonic.push_str(format!("$r{reg}").as_str());
				},
				InstrField::Tgt(tgt) => {
					mnemonic.push_str(format!("0x{tgt:X}").as_str());
				},
				InstrField::Imm(imm) => {
					mnemonic.push_str(format!("0x{imm:X}").as_str());
				},
				InstrField::Shamt(shamt) => {
					mnemonic.push_str(format!("0x{shamt:X}").as_str());
				},
				InstrField::Addr(offset, base) => {
					mnemonic.push_str(format!("0x{:X}", offset.wrapping_add(base)).as_str());
				},
			};
		}

		mnemonic
	}

}

impl R3000 {
	pub fn decode_and_exec(&mut self, instr: Instruction, bus: &mut Bus, scheduler: &mut Scheduler) {

		match instr.opcode() {

			0x00 => match instr.funct() {
				0x00 => self.op_sll(instr),
				0x02 => self.op_srl(instr),
				0x03 => self.op_sra(instr),
				0x04 => self.op_sllv(instr),
				0x06 => self.op_srlv(instr),
				0x07 => self.op_srav(instr),
				0x08 => self.op_jr(instr),
				0x09 => self.op_jalr(instr),
				0x0C => self.op_syscall(),
				0x0D => self.op_break(),
				0x10 => self.op_mfhi(instr),
				0x11 => self.op_mthi(instr),
				0x12 => self.op_mflo(instr),
				0x13 => self.op_mtlo(instr),
				0x18 => self.op_mult(instr),
				0x19 => self.op_multu(instr),
				0x1A => self.op_div(instr),
				0x1B => self.op_divu(instr),
				0x20 => self.op_add(instr),
				0x21 => self.op_addu(instr),
				0x22 => self.op_sub(instr),
				0x23 => self.op_subu(instr),
				0x24 => self.op_and(instr),
				0x25 => self.op_or(instr),
				0x26 => self.op_xor(instr),
				0x27 => self.op_nor(instr),
				0x2A => self.op_slt(instr),
				0x2B => self.op_sltu(instr),

				_ => self.op_illegal(instr),
			}

			0x01 => self.op_bcondz(instr),

			0x02 => self.op_j(instr),
			0x03 => self.op_jal(instr),
			0x04 => self.op_beq(instr),
			0x05 => self.op_bne(instr),
			0x06 => self.op_blez(instr),
			0x07 => self.op_bgtz(instr),
			0x08 => self.op_addi(instr),
			0x09 => self.op_addiu(instr),
			0x0A => self.op_slti(instr),
			0x0B => self.op_sltiu(instr),
			0x0C => self.op_andi(instr),
			0x0D => self.op_ori(instr),
			0x0E => self.op_xori(instr),
			0x0F => self.op_lui(instr),

			// COP0
			0x10 => match instr.cop_opcode() {
				0x00 => self.op_mfcn(instr),
				0x04 => self.op_mtcn(instr),
				0x10 => self.op_rfe(instr),
				_ => self.op_illegal(instr),
			},

			// COP1
			0x11 => self.op_copn(),
			// COP2
			0x12 => match instr.cop_opcode() {
				0x00 => self.op_mfcn(instr),
				0x02 => self.op_mfcn(instr), // CFC2
				0x04 => self.op_mtcn(instr),
				0x06 => {}, // CTC2
				0x10..=0x1F => self.op_gte(instr), // COP2 imm25
				_ => self.op_illegal(instr),
			},
			// COP3
			0x13 => self.op_copn(),

			0x20 => self.op_lb(instr, bus, scheduler),
			0x21 => self.op_lh(instr, bus, scheduler),
			0x22 => self.op_lwl(instr, bus, scheduler),
			0x23 => self.op_lw(instr, bus, scheduler),
			0x24 => self.op_lbu(instr, bus, scheduler),
			0x25 => self.op_lhu(instr, bus, scheduler),
			0x26 => self.op_lwr(instr, bus, scheduler),
			0x28 => self.op_sb(instr, bus, scheduler),
			0x29 => self.op_sh(instr, bus, scheduler),
			0x2A => self.op_swl(instr, bus, scheduler),
			0x2B => self.op_sw(instr, bus, scheduler),
			0x2E => self.op_swr(instr, bus, scheduler),

			0x30 ..= 0x33 => self.op_lwcn(instr, bus, scheduler),
			0x38 ..= 0x3B => self.op_swcn(instr, bus, scheduler),

			_ => self.op_illegal(instr),
		}

	}

	// ? Load/Store Instructions
	fn op_lui(&mut self, instr: Instruction) {
		let tgt = instr.reg_tgt();
		let imm = instr.imm16();

		self.registers.write_gpr(tgt, imm << 16);
	}

	fn op_sw(&mut self, instr: Instruction, bus: &mut Bus, scheduler: &mut Scheduler) {

		if self.cop0.read_reg(12) & 0x10000 != 0 {
			//println!("ignoring store while cache is isolated");
			return;
		}

		let offset = self.registers.read_gpr(instr.reg_src());

		let addr = offset.wrapping_add(instr.imm16_se());

		self.store32(bus, addr, self.registers.read_gpr(instr.reg_tgt()), scheduler);
	}

	fn op_lw(&mut self, instr: Instruction, bus: &mut Bus, scheduler: &mut Scheduler) {

		if self.cop0.read_reg(12) & 0x10000 != 0 {
			//println!("ignoring load while cache is isolated");
			return;
		}

		let offset = self.registers.read_gpr(instr.reg_src());
		let addr = offset.wrapping_add(instr.imm16_se());

		if addr % 4 == 0 {
			self.registers.write_gpr_delayed(instr.reg_tgt(), Self::load32(bus, addr, scheduler));
		} else {
			self.exception(Exception::AddrLoadError);
			self.cop0.reg_badvaddr = addr;
		}
	}

	fn op_sh(&mut self, instr: Instruction, bus: &mut Bus, scheduler: &mut Scheduler) {

		if self.cop0.read_reg(12) & 0x10000 != 0 {
			//println!("ignoring store while cache is isolated");
			return;
		}

		let offset = self.registers.read_gpr(instr.reg_src());
		let addr = offset.wrapping_add(instr.imm16_se());

		self.store16(bus, addr, self.registers.read_gpr(instr.reg_tgt()) as u16, scheduler);
	}

	fn op_lh(&mut self, instr: Instruction, bus: &mut Bus, scheduler: &mut Scheduler) {
		let addr = self.registers.read_gpr(instr.reg_src()).wrapping_add(instr.imm16_se());

		if addr % 2 == 0 {
			let new_val = Self::load16(bus, addr, scheduler) as i16;
			self.registers.write_gpr_delayed(instr.reg_tgt(), new_val as u32);
		} else {
			self.exception(Exception::AddrLoadError);
			self.cop0.reg_badvaddr = addr;
		}
	}

	fn op_lhu(&mut self, instr: Instruction, bus: &mut Bus, scheduler: &mut Scheduler) {
		let addr = self.registers.read_gpr(instr.reg_src()).wrapping_add(instr.imm16_se());

		if addr % 2 == 0 {
			self.registers.write_gpr_delayed(instr.reg_tgt(), Self::load16(bus, addr, scheduler) as u32);
		} else {
			self.exception(Exception::AddrLoadError);
			self.cop0.reg_badvaddr = addr;
		}
	}

	fn op_sb(&mut self, instr: Instruction, bus: &mut Bus, scheduler: &mut Scheduler) {

		if self.cop0.read_reg(12) & 0x10000 != 0 {
			//println!("ignoring store while cache is isolated");
			return;
		}

		let offset = self.registers.read_gpr(instr.reg_src());
		let addr = offset.wrapping_add(instr.imm16_se());

		self.store8(bus, addr, self.registers.read_gpr(instr.reg_tgt()) as u8, scheduler);
	}

	fn op_lb(&mut self, instr: Instruction, bus: &mut Bus, scheduler: &mut Scheduler) {

		let offset = self.registers.read_gpr(instr.reg_src());
		let addr = offset.wrapping_add(instr.imm16_se());

		let value = Self::load8(bus, addr, scheduler) as i8; // cast to i8 to sign extend

		self.registers.write_gpr_delayed(instr.reg_tgt(), value as u32);
	}

	fn op_lbu(&mut self, instr: Instruction, bus: &mut Bus, scheduler: &mut Scheduler) {

		let offset = self.registers.read_gpr(instr.reg_src());
		let addr = offset.wrapping_add(instr.imm16_se());

		let value = Self::load8(bus, addr, scheduler);

		self.registers.write_gpr_delayed(instr.reg_tgt(), value as u32);
	}

	fn op_lwl(&mut self, instr: Instruction, bus: &mut Bus, scheduler: &mut Scheduler) {

		let addr = self.registers.read_gpr(instr.reg_src()).wrapping_add(instr.imm16_se());

		let current_val = self.registers.read_gpr_lwl_lwr(instr.reg_tgt());

		let aligned_addr = addr & !0x3;
		let aligned_word = Self::load32(bus, aligned_addr, scheduler);

		let value = match addr & 0x3 {
			0 => (current_val & 0x00FFFFFF) | (aligned_word << 24),
			1 => (current_val & 0x0000FFFF) | (aligned_word << 16),
			2 => (current_val & 0x000000FF) | (aligned_word << 8),
			3 => (current_val & 0x00000000) | (aligned_word << 0),
			_ => unreachable!()
		};

		self.registers.write_gpr_delayed(instr.reg_tgt(), value);
	}

	fn op_lwr(&mut self, instr: Instruction, bus: &mut Bus, scheduler: &mut Scheduler) {

		let addr = self.registers.read_gpr(instr.reg_src()).wrapping_add(instr.imm16_se());

		let current_val = self.registers.read_gpr_lwl_lwr(instr.reg_tgt());

		let aligned_addr = addr & !0x3;
		let aligned_word = Self::load32(bus, aligned_addr, scheduler);

		let value = match addr & 0x3 {
			0 => (current_val & 0x00000000) | (aligned_word >> 0),
			1 => (current_val & 0xFF000000) | (aligned_word >> 8),
			2 => (current_val & 0xFFFF0000) | (aligned_word >> 16),
			3 => (current_val & 0xFFFFFF00) | (aligned_word >> 24),
			_ => unreachable!()
		};

		self.registers.write_gpr_delayed(instr.reg_tgt(), value);
	}

	fn op_swl(&mut self, instr: Instruction, bus: &mut Bus, scheduler: &mut Scheduler) {

		let addr = self.registers.read_gpr(instr.reg_src()).wrapping_add(instr.imm16_se());

		let reg_val = self.registers.read_gpr(instr.reg_tgt());

		let aligned_addr = addr & !0x3;
		let current_mem = Self::load32(bus, aligned_addr, scheduler);

		let value = match addr & 0x3 {
			0 => (current_mem & 0xFFFFFF00) | (reg_val >> 24),
			1 => (current_mem & 0xFFFF0000) | (reg_val >> 16),
			2 => (current_mem & 0xFF000000) | (reg_val >> 8),
			3 => (current_mem & 0x00000000) | (reg_val >> 0),
			_ => unreachable!()
		};

		self.store32(bus, aligned_addr, value, scheduler);
	}

	fn op_swr(&mut self, instr: Instruction, bus: &mut Bus, scheduler: &mut Scheduler) {

		let addr = self.registers.read_gpr(instr.reg_src()).wrapping_add(instr.imm16_se());

		let reg_val = self.registers.read_gpr(instr.reg_tgt());

		let aligned_addr = addr & !0x3;
		let current_mem = Self::load32(bus, aligned_addr, scheduler);

		let value = match addr & 0x3 {
			0 => (current_mem & 0x00000000) | (reg_val << 0),
			1 => (current_mem & 0x000000FF) | (reg_val << 8),
			2 => (current_mem & 0x0000FFFF) | (reg_val << 16),
			3 => (current_mem & 0x00FFFFFF) | (reg_val << 24),
			_ => unreachable!()
		};

		self.store32(bus, aligned_addr, value, scheduler);
	}

	fn op_mfhi(&mut self, instr: Instruction) {
		self.registers.write_gpr(instr.reg_dst(), self.registers.hi);
	}

	fn op_mflo(&mut self, instr: Instruction) {
		self.registers.write_gpr(instr.reg_dst(), self.registers.lo);
	}

	fn op_mthi(&mut self, instr: Instruction) {
		self.registers.hi = self.registers.read_gpr(instr.reg_src());
	}

	fn op_mtlo(&mut self, instr: Instruction) {
		self.registers.lo = self.registers.read_gpr(instr.reg_src());
	}

	// ? Logical Instructions
	fn op_ori(&mut self, instr: Instruction) {
		let result = self.registers.read_gpr(instr.reg_src()) | instr.imm16();
		
		self.registers.write_gpr(instr.reg_tgt(), result);
	}

	fn op_or(&mut self, instr: Instruction) {
		let result = self.registers.read_gpr(instr.reg_src()) | self.registers.read_gpr(instr.reg_tgt());

		self.registers.write_gpr(instr.reg_dst(), result);
	}

	fn op_xor(&mut self, instr: Instruction) {
		let result = self.registers.read_gpr(instr.reg_src()) ^ self.registers.read_gpr(instr.reg_tgt());

		self.registers.write_gpr(instr.reg_dst(), result);
	}

	fn op_xori(&mut self, instr: Instruction) {
		let result = self.registers.read_gpr(instr.reg_src()) ^ instr.imm16();

		self.registers.write_gpr(instr.reg_tgt(), result);
	}

	fn op_nor(&mut self, instr: Instruction) {
		let result = !(self.registers.read_gpr(instr.reg_src()) | self.registers.read_gpr(instr.reg_tgt()));

		self.registers.write_gpr(instr.reg_dst(), result);
	}

	fn op_andi(&mut self, instr: Instruction) {
		let result = self.registers.read_gpr(instr.reg_src()) & instr.imm16();

		self.registers.write_gpr(instr.reg_tgt(), result);
	}

	fn op_and(&mut self, instr: Instruction) {
		let result = self.registers.read_gpr(instr.reg_src()) & self.registers.read_gpr(instr.reg_tgt());

		self.registers.write_gpr(instr.reg_dst(), result);
	}

	// ? Arithmetic Instructions
	fn op_addiu(&mut self, instr: Instruction) {
		let result = self.registers.read_gpr(instr.reg_src()).wrapping_add(instr.imm16_se());

		self.registers.write_gpr(instr.reg_tgt(), result);
	}

	fn op_addu(&mut self, instr: Instruction) {
		let result = self.registers.read_gpr(instr.reg_src()).wrapping_add(self.registers.read_gpr(instr.reg_tgt()));

		self.registers.write_gpr(instr.reg_dst(), result);
	}

	fn op_addi(&mut self, instr: Instruction) {

		let src = self.registers.read_gpr(instr.reg_src()) as i32;

		let result = match src.checked_add(instr.imm16_se() as i32) {
			Some(result) => result as u32,
			None => { self.exception(Exception::ArithmeticOverflow); return }
		};

		self.registers.write_gpr(instr.reg_tgt(), result);

	}

	fn op_add(&mut self, instr: Instruction) {

		let src = self.registers.read_gpr(instr.reg_src()) as i32;

		let result = match src.checked_add(self.registers.read_gpr(instr.reg_tgt()) as i32) {
			Some(result) => result as u32,
			None => { self.exception(Exception::ArithmeticOverflow); return }
		};

		self.registers.write_gpr(instr.reg_dst(), result);

	}

	fn op_sub(&mut self, instr: Instruction) {
		let src = self.registers.read_gpr(instr.reg_src()) as i32;
		let tgt = self.registers.read_gpr(instr.reg_tgt()) as i32;

		match src.checked_sub(tgt) {
			Some(result) => self.registers.write_gpr(instr.reg_dst(), result as u32),
			None => self.exception(Exception::ArithmeticOverflow),
		}
	}

	fn op_subu(&mut self, instr: Instruction) {
		let result = self.registers.read_gpr(instr.reg_src()).wrapping_sub(self.registers.read_gpr(instr.reg_tgt()));

		self.registers.write_gpr(instr.reg_dst(), result);
	}

	fn op_slt(&mut self, instr: Instruction) {
		let src = self.registers.read_gpr(instr.reg_src()) as i32;
		let tgt = self.registers.read_gpr(instr.reg_tgt()) as i32;

		self.registers.write_gpr(instr.reg_dst(), (src < tgt) as u32);
	}

	fn op_sltu(&mut self, instr: Instruction) {
		let src = self.registers.read_gpr(instr.reg_src());
		let tgt = self.registers.read_gpr(instr.reg_tgt());

		self.registers.write_gpr(instr.reg_dst(), (src < tgt) as u32);
	}

	fn op_slti(&mut self, instr: Instruction) {
		let src = self.registers.read_gpr(instr.reg_src()) as i32;
		let imm = instr.imm16_se() as i32;

		self.registers.write_gpr(instr.reg_tgt(), (src < imm) as u32);
	}

	fn op_sltiu(&mut self, instr: Instruction) {
		let src = self.registers.read_gpr(instr.reg_src());
		let imm = instr.imm16_se();

		self.registers.write_gpr(instr.reg_tgt(), (src < imm) as u32);
	}

	fn op_div(&mut self, instr: Instruction) {
		let numerator = self.registers.read_gpr(instr.reg_src()) as i32;
		let denominator = self.registers.read_gpr(instr.reg_tgt()) as i32;

		// divide by zero has special values for HI/LO
		if denominator == 0 {
			self.registers.hi = numerator as u32;
			self.registers.lo = if numerator < 0 { 1 } else { 0xFFFFFFFF };
			
		} else if numerator as u32 == 0x80000000 && denominator == -1 {
			// result is outside of i32 range
			self.registers.hi = 0;
			self.registers.lo = 0x80000000;
		} else {
			// normal division
			self.registers.hi = (numerator % denominator) as u32;
			self.registers.lo = (numerator / denominator) as u32;
		}
	}

	fn op_divu(&mut self, instr: Instruction) {
		let numerator = self.registers.read_gpr(instr.reg_src());
		let denominator = self.registers.read_gpr(instr.reg_tgt());

		// divide by zero has special values for HI/LO
		if denominator == 0 {
			self.registers.hi = numerator as u32;
			self.registers.lo = 0xFFFFFFFF;
		} else {
			// normal division
			self.registers.hi = (numerator % denominator) as u32;
			self.registers.lo = (numerator / denominator) as u32;
		}
	}

	fn op_mult(&mut self, instr: Instruction) {
		let multiplicand = (self.registers.read_gpr(instr.reg_src()) as i32) as i64;
		let multiplier = (self.registers.read_gpr(instr.reg_tgt()) as i32) as i64;

		let product = (multiplicand * multiplier) as u64;

		self.registers.hi = (product >> 32) as u32;
		self.registers.lo = product as u32;
	}

	fn op_multu(&mut self, instr: Instruction) {
		let multiplicand = self.registers.read_gpr(instr.reg_src()) as u64;
		let multiplier = self.registers.read_gpr(instr.reg_tgt()) as u64;

		let product = multiplicand * multiplier;

		self.registers.hi = (product >> 32) as u32;
		self.registers.lo = product as u32;
	}

	// ? Shift Instructions
	fn op_sll(&mut self, instr: Instruction) {
		let new_val = self.registers.read_gpr(instr.reg_tgt()) << instr.shamt();

		self.registers.write_gpr(instr.reg_dst(), new_val);
	}

	fn op_sllv(&mut self, instr: Instruction) {
		let shamt = self.registers.read_gpr(instr.reg_src()) & 0x1F;
		let new_val = self.registers.read_gpr(instr.reg_tgt()) << shamt;
		
		self.registers.write_gpr(instr.reg_dst(), new_val);
	}

	fn op_srl(&mut self, instr: Instruction) {
		let new_val = self.registers.read_gpr(instr.reg_tgt()) >> instr.shamt();

		self.registers.write_gpr(instr.reg_dst(), new_val);
	}

	fn op_srlv(&mut self, instr: Instruction) {
		let shamt = self.registers.read_gpr(instr.reg_src()) & 0x1F;
		let new_val = self.registers.read_gpr(instr.reg_tgt()) >> shamt;

		self.registers.write_gpr(instr.reg_dst(), new_val);
	}

	fn op_sra(&mut self, instr: Instruction) {
		let result = (self.registers.read_gpr(instr.reg_tgt()) as i32) >> instr.shamt();

		self.registers.write_gpr(instr.reg_dst(), result as u32);
	}

	fn op_srav(&mut self, instr: Instruction) {
		let shamt = self.registers.read_gpr(instr.reg_src()) & 0x1F;
		let result = (self.registers.read_gpr(instr.reg_tgt()) as i32) >> shamt;

		self.registers.write_gpr(instr.reg_dst(), result as u32);
	}


	// ? Branch Instructions
	fn op_j(&mut self, instr: Instruction) {
		let jmp_addr = instr.imm26();

		self.delayed_branch = Some((self.pc & 0xF0000000) | (jmp_addr << 2));
	}

	fn op_jal(&mut self, instr: Instruction) {
		let jmp_addr = instr.imm26();

		self.delayed_branch = Some((self.pc & 0xF0000000) | (jmp_addr << 2));

		self.registers.write_gpr(31, self.pc.wrapping_add(8));
	}

	fn op_jalr(&mut self, instr: Instruction) {
		self.delayed_branch = Some(self.registers.read_gpr(instr.reg_src()));

		self.registers.write_gpr(instr.reg_dst(), self.pc.wrapping_add(8));
	}

	fn op_jr(&mut self, instr: Instruction) {
		let jmp_addr = self.registers.read_gpr(instr.reg_src());

		self.delayed_branch = Some(jmp_addr);
	}

	fn op_bne(&mut self, instr: Instruction) {

		if self.registers.read_gpr(instr.reg_src()) != self.registers.read_gpr(instr.reg_tgt()) {
			self.delayed_branch = Some(self.pc.wrapping_add(instr.imm16_se() << 2).wrapping_add(4));
		}
	}

	fn op_beq(&mut self, instr: Instruction) {
		if self.registers.read_gpr(instr.reg_src()) == self.registers.read_gpr(instr.reg_tgt()) {
			self.delayed_branch = Some(self.pc.wrapping_add(instr.imm16_se() << 2).wrapping_add(4));
		}
	}

	fn op_bgtz(&mut self, instr: Instruction) {
		if self.registers.read_gpr(instr.reg_src()) as i32 > 0 {
			self.delayed_branch = Some(self.pc.wrapping_add(instr.imm16_se() << 2).wrapping_add(4));
		}
	}

	fn op_blez(&mut self, instr: Instruction) {
		if self.registers.read_gpr(instr.reg_src()) as i32 <= 0 {
			self.delayed_branch = Some(self.pc.wrapping_add(instr.imm16_se() << 2).wrapping_add(4));
		}
	}

	// BLTZ, BLTZAL, BGEZ, BGEZAL instructions
	fn op_bcondz(&mut self, instr: Instruction) {

		let is_bgez = (instr.raw >> 16) & 0x1;
		let link = (instr.raw >> 17) & 0xF == 0x8;

		let reg_src = self.registers.read_gpr(instr.reg_src()) as i32;
		
		if ((reg_src < 0) as u32 ^ is_bgez) != 0 {
			self.delayed_branch = Some(self.pc.wrapping_add(instr.imm16_se() << 2).wrapping_add(4));
		}

		if link {
			self.registers.write_gpr(31, self.pc.wrapping_add(8));
		}

	}

	// ? Trap Instructions
	fn op_syscall(&mut self) {
		//println!("[0x{:X}] SYSCALL $4=0x{:X}", self.pc, self.registers.read_gpr(4));

		self.exception(Exception::Syscall);
	}

	fn op_break(&mut self) {
		self.exception(Exception::Breakpoint);
	}

	fn op_illegal(&mut self, instr: Instruction) {
		log::error!("Illegal instruction 0x{:X} (PC: 0x{:X}) (opcode: 0x{:X} funct: 0x{:X} cop0 opcode: 0x{:X})", instr.raw, self.pc, instr.opcode(), instr.funct(), instr.cop_opcode());

		self.exception(Exception::ReservedInstruction);
	}

	// ? Cop0 Instructions
	fn op_mfcn(&mut self, instr: Instruction) {
		let value = match instr.cop_num() {
			0 => self.cop0.read_reg(instr.reg_dst()),
			2 => 0,
			_ => todo!("MFC{} $r{}", instr.cop_num(), instr.reg_dst())
		};

		self.registers.write_gpr_delayed(instr.reg_tgt(), value);
	}
	
	fn op_mtcn(&mut self, instr: Instruction) {
		let write = self.registers.read_gpr(instr.reg_tgt());

		match instr.cop_num() {
			0 => self.cop0.write_reg(instr.reg_dst(), write),
			2 => {},
			_ => todo!("MTC{} $r{}", instr.cop_num(), instr.reg_dst()),
		};
	}

	fn op_rfe(&mut self, instr: Instruction) {

		if instr.raw & 0x3F != 0b010000 {
			panic!("invalid cop0 encoding: 0x{:X}", instr.raw);
		}

		self.cop0.reg_sr.pop_exception();

	}

	// ? Coprocessor Instructions
	fn op_copn(&mut self) {
		self.exception(Exception::CopUnusable);
	}

	fn op_gte(&mut self, instr: Instruction) {
		//error!("Unhandled GTE instruction: 0x{:X}", instr.raw);
	}

	fn op_lwcn(&mut self, instr: Instruction, bus: &mut Bus, scheduler: &mut Scheduler) {
		if self.cop0.read_reg(12) & 0x10000 != 0 {
			return;
		}

		let offset = self.registers.read_gpr(instr.reg_src());
		let addr = offset.wrapping_add(instr.imm16_se());

		if addr % 4 == 0 {
			match instr.cop_num() {
				2 => {},
				_ => self.exception(Exception::CopUnusable),
			}
		} else {
			self.exception(Exception::AddrLoadError);
			self.cop0.reg_badvaddr = addr;
		}
	}

	fn op_swcn(&mut self, instr: Instruction, bus: &mut Bus, scheduler: &mut Scheduler) {
		if self.cop0.read_reg(12) & 0x10000 != 0 {
			return;
		}

		let offset = self.registers.read_gpr(instr.reg_src());
		let addr = offset.wrapping_add(instr.imm16_se());

		let write = match instr.cop_num() {
			2 => 0,
			3 => { self.exception(Exception::ReservedInstruction); 0 },
			_ => { self.exception(Exception::CopUnusable); 0 }
		};

		self.store32(bus, addr, write, scheduler);
	}

}

// helper functions
impl R3000 {
	fn load32(bus: &mut Bus, addr: u32, scheduler: &mut Scheduler) -> u32 {
		if bus.read_breakpoints.contains(&addr) {
			bus.breakpoint_hit = (true, addr);
		}
		
		bus.read32(addr, scheduler)
	}
	
	fn load16(bus: &mut Bus, addr: u32, scheduler: &mut Scheduler) -> u16 {
		if bus.read_breakpoints.contains(&addr) {
			bus.breakpoint_hit = (true, addr);
		}
		
		bus.read16(addr, scheduler)
	}
	
	fn load8(bus: &mut Bus, addr: u32, scheduler: &mut Scheduler) -> u8 {
		if bus.read_breakpoints.contains(&addr) {
			bus.breakpoint_hit = (true, addr);
		}
		
		bus.read8(addr, scheduler)
	}
	
	fn store32(&mut self, bus: &mut Bus, addr: u32, write: u32, scheduler: &mut Scheduler) {
		if bus.write_breakpoints.contains(&addr) {
			bus.breakpoint_hit = (true, addr);
		}
		
		if addr % 4 == 0 {
			bus.write32(addr, write, scheduler);
		} else {
			self.exception(Exception::AddrStoreError);
			self.cop0.reg_badvaddr = addr;
		}
	}
	
	fn store16(&mut self, bus: &mut Bus, addr: u32, write: u16, scheduler: &mut Scheduler) {
		if bus.write_breakpoints.contains(&addr) {
			bus.breakpoint_hit = (true, addr);
		}
		
		if addr % 2 == 0 {
			bus.write16(addr, write, scheduler);
		} else {
			self.exception(Exception::AddrStoreError);
			self.cop0.reg_badvaddr = addr;
		}
	}
	
	fn store8(&mut self, bus: &mut Bus, addr: u32, write: u8, scheduler: &mut Scheduler) {
		if bus.write_breakpoints.contains(&addr) {
			bus.breakpoint_hit = (true, addr);
		}
		
		bus.write8(addr, write, scheduler);
	}
}
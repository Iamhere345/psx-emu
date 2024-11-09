use std::f32::consts::E;

use crate::bus::Bus;

use super::{Exception, R3000};

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

	pub fn cop0_opcode(&self) -> u32 {
		(self.raw >> 21) & 0x1F
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
}

impl R3000 {
	pub fn decode_and_exec(&mut self, instr: Instruction, bus: &mut Bus) {

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

			0x10 => match instr.cop0_opcode() {
				0x00 => self.op_mfc(instr),
				0x04 => self.op_mtc(instr),
				0x10 => self.op_rfe(instr),
				_ => self.op_illegal(instr),
			}

			0x11 => self.op_copn(),
			0x12 => self.op_gte(instr),
			0x13 => self.op_copn(),

			0x20 => self.op_lb(instr, bus),
			0x21 => self.op_lh(instr, bus),
			0x22 => self.op_lwl(instr, bus),
			0x23 => self.op_lw(instr, bus),
			0x24 => self.op_lbu(instr, bus),
			0x25 => self.op_lhu(instr, bus),
			0x26 => self.op_lwr(instr, bus),
			0x28 => self.op_sb(instr, bus),
			0x29 => self.op_sh(instr, bus),
			0x2A => self.op_swl(instr, bus),
			0x2B => self.op_sw(instr, bus),
			0x2E => self.op_swr(instr, bus),

			0x30 => self.op_lwcn(),
			0x31 => self.op_lwcn(),
			0x32 => self.op_lwc_gte(instr),
			0x33 => self.op_lwcn(),
			0x38 => self.op_swcn(),
			0x39 => self.op_swcn(),
			0x3A => self.op_swc_gte(instr),
			0x3B => self.op_swcn(),

			_ => self.op_illegal(instr),
		}

	}

	// ? Load/Store Instructions
	fn op_lui(&mut self, instr: Instruction) {
		let tgt = instr.reg_tgt();
		let imm = instr.imm16();

		self.registers.write_gpr(tgt, imm << 16);
	}

	fn op_sw(&mut self, instr: Instruction, bus: &mut Bus) {

		if self.cop0.read_reg(12) & 0x10000 != 0 {
			//println!("ignoring store while cache is isolated");
			return;
		}

		let offset = self.registers.read_gpr(instr.reg_src());

		let addr = offset.wrapping_add(instr.imm16_se());

		if addr % 4 == 0 {
			bus.write32(addr, self.registers.read_gpr(instr.reg_tgt()));
		} else {
			self.exception(Exception::AddrStoreError);
		}
	}

	fn op_lw(&mut self, instr: Instruction, bus: &mut Bus) {

		if self.cop0.read_reg(12) & 0x10000 != 0 {
			//println!("ignoring load while cache is isolated");
			return;
		}

		let offset = self.registers.read_gpr(instr.reg_src());
		let addr = offset.wrapping_add(instr.imm16_se());

		if addr % 4 == 0 {
			self.registers.write_gpr_delayed(instr.reg_tgt(), bus.read32(addr));
		} else {
			self.exception(Exception::AddrLoadError)
		}
	}

	fn op_sh(&mut self, instr: Instruction, bus: &mut Bus) {

		if self.cop0.read_reg(12) & 0x10000 != 0 {
			//println!("ignoring store while cache is isolated");
			return;
		}

		let offset = self.registers.read_gpr(instr.reg_src());
		let addr = offset.wrapping_add(instr.imm16_se());

		if addr % 2 == 0 {
			bus.write16(addr, self.registers.read_gpr(instr.reg_tgt()) as u16);
		} else {
			self.exception(Exception::AddrStoreError);
		}
	}

	fn op_lh(&mut self, instr: Instruction, bus: &mut Bus) {
		let addr = self.registers.read_gpr(instr.reg_src()).wrapping_add(instr.imm16_se());

		if addr % 2 == 0 {
			let new_val = bus.read16(addr) as i16;
			self.registers.write_gpr_delayed(instr.reg_tgt(), new_val as u32);
		} else {
			self.exception(Exception::AddrLoadError);
		}
	}

	fn op_lhu(&mut self, instr: Instruction, bus: &mut Bus) {
		let addr = self.registers.read_gpr(instr.reg_src()).wrapping_add(instr.imm16_se());

		if addr % 2 == 0 {
			self.registers.write_gpr_delayed(instr.reg_tgt(), bus.read16(addr) as u32);
		} else {
			self.exception(Exception::AddrLoadError);
		}
	}

	fn op_sb(&mut self, instr: Instruction, bus: &mut Bus) {

		if self.cop0.read_reg(12) & 0x10000 != 0 {
			//println!("ignoring store while cache is isolated");
			return;
		}

		let offset = self.registers.read_gpr(instr.reg_src());
		let addr = offset.wrapping_add(instr.imm16_se());

		bus.write8(addr, self.registers.read_gpr(instr.reg_tgt()) as u8);
	}

	fn op_lb(&mut self, instr: Instruction, bus: &mut Bus) {

		let offset = self.registers.read_gpr(instr.reg_src());
		let addr = offset.wrapping_add(instr.imm16_se());

		let value = bus.read8(addr) as i8; // cast to i8 to sign extend

		self.registers.write_gpr_delayed(instr.reg_tgt(), value as u32);
	}

	fn op_lbu(&mut self, instr: Instruction, bus: &mut Bus) {

		let offset = self.registers.read_gpr(instr.reg_src());
		let addr = offset.wrapping_add(instr.imm16_se());

		let value = bus.read8(addr);

		self.registers.write_gpr_delayed(instr.reg_tgt(), value as u32);
	}

	fn op_lwl(&mut self, instr: Instruction, bus: &mut Bus) {

		let addr = self.registers.read_gpr(instr.reg_src()).wrapping_add(instr.imm16_se());

		let current_val = self.registers.read_gpr_lwl_lwr(instr.reg_tgt());

		let aligned_addr = addr & !0x3;
		let aligned_word = bus.read32(aligned_addr);

		let value = match addr & 0x3 {
			0 => (current_val & 0x00FFFFFF) | (aligned_word << 24),
			1 => (current_val & 0x0000FFFF) | (aligned_word << 16),
			2 => (current_val & 0x000000FF) | (aligned_word << 8),
			3 => (current_val & 0x00000000) | (aligned_word << 0),
			_ => unreachable!()
		};

		self.registers.write_gpr_delayed(instr.reg_tgt(), value);
	}

	fn op_lwr(&mut self, instr: Instruction, bus: &mut Bus) {

		let addr = self.registers.read_gpr(instr.reg_src()).wrapping_add(instr.imm16_se());

		let current_val = self.registers.read_gpr_lwl_lwr(instr.reg_tgt());

		let aligned_addr = addr & !0x3;
		let aligned_word = bus.read32(aligned_addr);

		let value = match addr & 0x3 {
			0 => (current_val & 0x00000000) | (aligned_word >> 0),
			1 => (current_val & 0xFF000000) | (aligned_word >> 8),
			2 => (current_val & 0xFFFF0000) | (aligned_word >> 16),
			3 => (current_val & 0xFFFFFF00) | (aligned_word >> 24),
			_ => unreachable!()
		};

		self.registers.write_gpr_delayed(instr.reg_tgt(), value);
	}

	fn op_swl(&mut self, instr: Instruction, bus: &mut Bus) {

		let addr = self.registers.read_gpr(instr.reg_src()).wrapping_add(instr.imm16_se());

		let reg_val = self.registers.read_gpr(instr.reg_tgt());

		let aligned_addr = addr & !0x3;
		let current_mem = bus.read32(aligned_addr);

		let value = match addr & 0x3 {
			0 => (current_mem & 0xFFFFFF00) | (reg_val >> 24),
			1 => (current_mem & 0xFFFF0000) | (reg_val >> 16),
			2 => (current_mem & 0xFF000000) | (reg_val >> 8),
			3 => (current_mem & 0x00000000) | (reg_val >> 0),
			_ => unreachable!()
		};

		self.registers.write_gpr_delayed(instr.reg_tgt(), value);
	}

	fn op_swr(&mut self, instr: Instruction, bus: &mut Bus) {

		let addr = self.registers.read_gpr(instr.reg_src()).wrapping_add(instr.imm16_se());

		let reg_val = self.registers.read_gpr(instr.reg_tgt());

		let aligned_addr = addr & !0x3;
		let current_mem = bus.read32(aligned_addr);

		let value = match addr & 0x3 {
			0 => (current_mem & 0x00000000) | (reg_val << 0),
			1 => (current_mem & 0x000000FF) | (reg_val << 8),
			2 => (current_mem & 0x0000FFFF) | (reg_val << 16),
			3 => (current_mem & 0x00FFFFFF) | (reg_val << 24),
			_ => unreachable!()
		};

		self.registers.write_gpr_delayed(instr.reg_tgt(), value);
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

			if denominator >= 0 {
				self.registers.lo = 0xFFFFFFFF; // -1
			} else {
				self.registers.lo = 1;
			}
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
		let shamt = instr.reg_src() & 0x1F;
		let new_val = self.registers.read_gpr(instr.reg_tgt()) << shamt;
		
		self.registers.write_gpr(instr.reg_dst(), new_val);
	}

	fn op_srl(&mut self, instr: Instruction) {
		let new_val = self.registers.read_gpr(instr.reg_tgt()) >> instr.shamt();

		self.registers.write_gpr(instr.reg_dst(), new_val);
	}

	fn op_srlv(&mut self, instr: Instruction) {
		let shamt = instr.reg_src() & 0x1F;
		let new_val = self.registers.read_gpr(instr.reg_tgt()) >> shamt;

		self.registers.write_gpr(instr.reg_dst(), new_val);
	}

	fn op_sra(&mut self, instr: Instruction) {
		let result = (self.registers.read_gpr(instr.reg_tgt()) as i32) >> instr.shamt();

		self.registers.write_gpr(instr.reg_dst(), result as u32);
	}

	fn op_srav(&mut self, instr: Instruction) {
		let shamt = instr.reg_src() & 0x1F;
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
		println!("Illegal instruction 0x{:X} (PC: 0x{:X}) (opcode: 0x{:X} funct: 0x{:X} cop0 opcode: 0x{:X})", instr.raw, self.pc, instr.opcode(), instr.funct(), instr.cop0_opcode());

		self.exception(Exception::ReservedInstruction);
	}

	// ? Cop0 Instructions
	fn op_mfc(&mut self, instr: Instruction) {
		let value = self.cop0.read_reg(instr.reg_dst());

		self.registers.write_gpr_delayed(instr.reg_tgt(), value);
	}
	
	fn op_mtc(&mut self, instr: Instruction) {
		self.cop0.write_reg(instr.reg_dst(), self.registers.read_gpr(instr.reg_tgt()));
	}

	fn op_rfe(&mut self, instr: Instruction) {

		if instr.raw & 0x3F != 0b010000 {
			panic!("invalid cop0 encoding: 0x{:X}", instr.raw);
		}

		let mode = self.cop0.reg_sr & 0x3F;
		self.cop0.reg_sr &= !0x3F;
		self.cop0.reg_sr |= mode >> 2;

	}

	// ? Coprocessor Instructions
	fn op_copn(&mut self) {
		self.exception(Exception::CopUnusable);
	}

	fn op_gte(&mut self, instr: Instruction) {
		panic!("Unhandled GTE instruction: 0x{:X}", instr.raw);
	}

	fn op_lwcn(&mut self) {
		self.exception(Exception::CopUnusable);
	}

	fn op_lwc_gte(&mut self, instr: Instruction) {
		panic!("Unhandled GTE LWC: 0x{:X}", instr.raw);
	}

	fn op_swcn(&mut self) {
		self.exception(Exception::CopUnusable);
	}

	fn op_swc_gte(&mut self, instr: Instruction) {
		panic!("Unhandled GTE SWC: 0x{:X}", instr.raw);
	}

}
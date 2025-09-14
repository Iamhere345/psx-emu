const I44_MIN: i64 = -(1 << 43);
const I44_MAX: i64 = (1 << 43) - 1;

const MAC1_OVERFLOW: u32 = 30;
const MAC1_UNDERFLOW: u32 = 27;
const IR1_SATURATED: u32 = 24;
const COLOUR_R_SATURATED: u32 = 21;
const DIVIDE_OVERFLOW: u32 = 17;
const SX2_SATURATED: u32 = 14;
const IR0_SATURATED: u32 = 12;

const UNR_TABLE: [u32; 0x101] = [
    0xFF, 0xFD, 0xFB, 0xF9, 0xF7, 0xF5, 0xF3, 0xF1, 0xEF, 0xEE, 0xEC, 0xEA, 0xE8, 0xE6, 0xE4, 0xE3,
    0xE1, 0xDF, 0xDD, 0xDC, 0xDA, 0xD8, 0xD6, 0xD5, 0xD3, 0xD1, 0xD0, 0xCE, 0xCD, 0xCB, 0xC9, 0xC8,
    0xC6, 0xC5, 0xC3, 0xC1, 0xC0, 0xBE, 0xBD, 0xBB, 0xBA, 0xB8, 0xB7, 0xB5, 0xB4, 0xB2, 0xB1, 0xB0,
    0xAE, 0xAD, 0xAB, 0xAA, 0xA9, 0xA7, 0xA6, 0xA4, 0xA3, 0xA2, 0xA0, 0x9F, 0x9E, 0x9C, 0x9B, 0x9A,
    0x99, 0x97, 0x96, 0x95, 0x94, 0x92, 0x91, 0x90, 0x8F, 0x8D, 0x8C, 0x8B, 0x8A, 0x89, 0x87, 0x86,
    0x85, 0x84, 0x83, 0x82, 0x81, 0x7F, 0x7E, 0x7D, 0x7C, 0x7B, 0x7A, 0x79, 0x78, 0x77, 0x75, 0x74,
    0x73, 0x72, 0x71, 0x70, 0x6F, 0x6E, 0x6D, 0x6C, 0x6B, 0x6A, 0x69, 0x68, 0x67, 0x66, 0x65, 0x64,
    0x63, 0x62, 0x61, 0x60, 0x5F, 0x5E, 0x5D, 0x5D, 0x5C, 0x5B, 0x5A, 0x59, 0x58, 0x57, 0x56, 0x55,
    0x54, 0x53, 0x53, 0x52, 0x51, 0x50, 0x4F, 0x4E, 0x4D, 0x4D, 0x4C, 0x4B, 0x4A, 0x49, 0x48, 0x48,
    0x47, 0x46, 0x45, 0x44, 0x43, 0x43, 0x42, 0x41, 0x40, 0x3F, 0x3F, 0x3E, 0x3D, 0x3C, 0x3C, 0x3B,
    0x3A, 0x39, 0x39, 0x38, 0x37, 0x36, 0x36, 0x35, 0x34, 0x33, 0x33, 0x32, 0x31, 0x31, 0x30, 0x2F,
    0x2E, 0x2E, 0x2D, 0x2C, 0x2C, 0x2B, 0x2A, 0x2A, 0x29, 0x28, 0x28, 0x27, 0x26, 0x26, 0x25, 0x24,
    0x24, 0x23, 0x22, 0x22, 0x21, 0x20, 0x20, 0x1F, 0x1E, 0x1E, 0x1D, 0x1D, 0x1C, 0x1B, 0x1B, 0x1A,
    0x19, 0x19, 0x18, 0x18, 0x17, 0x16, 0x16, 0x15, 0x15, 0x14, 0x14, 0x13, 0x12, 0x12, 0x11, 0x11,
    0x10, 0x0F, 0x0F, 0x0E, 0x0E, 0x0D, 0x0D, 0x0C, 0x0C, 0x0B, 0x0A, 0x0A, 0x09, 0x09, 0x08, 0x08,
    0x07, 0x07, 0x06, 0x06, 0x05, 0x05, 0x04, 0x04, 0x03, 0x03, 0x02, 0x02, 0x01, 0x01, 0x00, 0x00,
    0x00,
];

#[derive(Default, Clone, Copy)]
struct Vector3 {
	x: i16,
	y: i16,
	z: i16
}

impl Vector3 {
	fn from_word(&mut self, word: u32) {
		self.x = (word & 0xFFFF) as i16;
		self.y = (word >> 16) as i16;
	}
}

#[derive(Default, Clone, Copy)]
struct Vector3_32 {
	x: i32,
	y: i32,
	z: i32
}

#[derive(Debug, Default, Clone, Copy)]
struct Vector2 {
	x: i16,
	y: i16,
}

impl Vector2 {
	fn as_word(&self) -> u32 {
		(self.x as u32 & 0xFFFF) | ((self.y as u32) << 16)
	}
	
	fn from_word(word: u32) -> Self {
		Self {
			x: word as i16,
			y: (word >> 16) as i16,
		}
	}
}

#[derive(Default)]
struct Vector2_32 {
	x: i32,
	y: i32,
}

#[derive(Default, Clone, Copy)]
struct Rgb {
	r: u8,
	g: u8,
	b: u8,
	c: u8,
}

impl Rgb {
	fn as_word(&self) -> u32 {
		(self.r as u32)
			| ((self.g as u32) << 8)
			| ((self.b as u32) << 16)
			| ((self.c as u32) << 24)
	}

	fn from_word(word: u32) -> Self {
		Self {
			r: (word & 0xFF) as u8,
			g: ((word >> 8) & 0xFF) as u8,
			b: ((word >> 16) & 0xFF) as u8,
			c: ((word >> 24) & 0xFF) as u8,
		}
	}
}

#[derive(Default, Clone, Copy)]
struct Matrix3x3 {
	m11: i16,
	m12: i16,
	m13: i16,
	m21: i16,
	m22: i16,
	m23: i16,
	m31: i16,
	m32: i16,
	m33: i16,
}

struct GteRegisters {
	// ? data registers (cop2r0-31)
	// vectors
	v: [Vector3; 3],
	// colour
	rgbc: Rgb,
	// Average Z value (for Ordering Table)
	otz: u16,
	// 16bit Accumulator (Interpolate)
	ir0: i16,
	// 16bit Accumulator (Vector)
	ir1: i16,
	ir2: i16,
	ir3: i16,
	// Screen XY-coordinate FIFO  (3 stages)
	sxy0: Vector2,
	sxy1: Vector2,
	sxy2: Vector2,
	// Screen Z-coordinate FIFO   (4 stages)
	sz0: u16,
	sz1: u16,
	sz2: u16,
	sz3: u16,
	// Color CRGB-code/color FIFO (3 stages)
	rgb: [Rgb; 3],
	// Prohibited
	res1: u32,
	// 32bit Maths Accumulators (Value)
	mac0: i32,
	// 32bit Maths Accumulators (Vector)
	mac1: i32,
	mac2: i32,
	mac3: i32,
	mac0_unclamped: i64, // used for RTP
	mac3_unclamped: i64,
	// Count Leading-Zeroes/Ones (sign bits) (src/result)
	lzcs: i32,
	// ? control registers (cop2r32-63)
	// Rotation matrix (3x3)
	rot_matrix: Matrix3x3,
	// Translation vector (X,Y,Z)
	translation_vec: Vector3_32,
	// Light source matrix (3x3)
	light_src_matrix: Matrix3x3,
	// Background color (R,G,B)
	bg_colour: Vector3_32,
	// Light color matrix source (3x3)
	light_colour_matrix: Matrix3x3,
	// Far color (R,G,B)
	far_colour: Vector3_32,
	// Screen offset (X,Y)
	screen_offset: Vector2_32,
	// Projection plane distance
	h: u16,
	// Depth queing parameter A (coeff)
	dqa: i16,
	// Depth queing parameter B (offset)
	dqb: i32,
	// Average Z scale factors
	zsf3: i16,
	zsf4: i16,
	// Returns any calculation errors
	flag: u32,
}

impl GteRegisters {
	pub fn new() -> Self {
		Self {
			// ? data registers (cop2r0-31)
			// vectors
			v: [Vector3::default(); 3],
			// colour
			rgbc: Rgb::default(),
			// Average Z value (for Ordering Table)
			otz: 0,
			// 16bit Accumulator (Interpolate)
			ir0: 0,
			// 16bit Accumulator (Vector)
			ir1: 0,
			ir2: 0,
			ir3: 0,
			// Screen XY-coordinate FIFO  (3 stages)
			sxy0: Vector2::default(),
			sxy1: Vector2::default(),
			sxy2: Vector2::default(),
			// Screen Z-coordinate FIFO   (4 stages)
			sz0: 0,
			sz1: 0,
			sz2: 0,
			sz3: 0,
			// Color CRGB-code/color FIFO (3 stages)
			rgb: [Rgb::default(); 3],
			// Prohibited
			res1: 0,
			// 32bit Maths Accumulators (Value)
			mac0: 0,
			// 32bit Maths Accumulators (Vector)
			mac1: 0,
			mac2: 0,
			mac3: 0,
			mac0_unclamped: 0,
			mac3_unclamped: 0,
			// Count Leading-Zeroes/Ones (sign bits) (src/result)
			lzcs: 0,
			// ? control registers (cop2r32-63)
			// Rotation matrix (3x3)
			rot_matrix: Matrix3x3::default(),
			// Translation vector (X,Y,Z)
			translation_vec: Vector3_32::default(),
			// Light source matrix (3x3)
			light_src_matrix: Matrix3x3::default(),
			// Background color (R,G,B)
			bg_colour: Vector3_32::default(),
			// Light color matrix source (3x3)
			light_colour_matrix: Matrix3x3::default(),
			// Far color (R,G,B)
			far_colour: Vector3_32::default(),
			// Screen offset (X,Y)
			screen_offset: Vector2_32::default(),
			// Projection plane distance
			h: 0,
			// Depth queing parameter A (coeff)
			dqa: 0,
			// Depth queing parameter B (offset)
			dqb: 0,
			// Average Z scale factors
			zsf3: 0,
			zsf4: 0,
			// Returns any calculation errors
			flag: 0,
		}
	}

	fn read_data_register(&self, reg_index: u32) -> u32 {
		let result = match reg_index {
			0 => (self.v[0].x as u16 as u32) | ((self.v[0].y as u16 as u32) << 16),
			1 => self.v[0].z as i32 as u32,
			2 => (self.v[1].x as u16 as u32) | ((self.v[1].y as u16 as u32) << 16),
			3 => self.v[1].z as u32,
			4 => (self.v[2].x as u16 as u32) | ((self.v[2].y as u16 as u32) << 16),
			5 => self.v[2].z as u32,
			6 => self.rgbc.as_word(),
			7 => self.otz as u32,
			8 => self.ir0 as u32,
			9 => self.ir1 as u32,
			10 => self.ir2 as u32,
			11 => self.ir3 as u32,
			12 => self.sxy0.as_word(),
			13 => self.sxy1.as_word(),
			14 => self.sxy2.as_word(),
			15 => self.sxy2.as_word(), // SXYP is a mirror os SXY2 when read
			16 => self.sz0 as u32,
			17 => self.sz1 as u32,
			18 => self.sz2 as u32,
			19 => self.sz3 as u32,
			20 => self.rgb[0].as_word(),
			21 => self.rgb[1].as_word(),
			22 => self.rgb[2].as_word(),
			23 => self.res1,
			24 => self.mac0 as u32,
			25 => self.mac1 as u32,
			26 => self.mac2 as u32,
			27 => self.mac3 as u32,
			28 => self.read_orgb(),
			29 => self.read_orgb(),
			30 => self.lzcs as u32,
			31 => self.read_lzcr(),

			_ => unreachable!(),
		};

		result
	}

	fn write_data_register(&mut self, reg_index: u32, write: u32) {

		match reg_index {
			0 => self.v[0].from_word(write),
			1 => self.v[0].z = write as i16,
			2 => self.v[1].from_word(write),
			3 => self.v[1].z = write as i16,
			4 => self.v[2].from_word(write),
			5 => self.v[2].z = write as i16,
			6 => self.rgbc = Rgb::from_word(write),
			7 => self.otz = write as u16,
			8 => self.ir0 = write as i16,
			9 => self.ir1 = write as i16,
			10 => self.ir2 = write as i16,
			11 => self.ir3 = write as i16,
			12 => self.sxy0 = Vector2::from_word(write),
			13 => self.sxy1 = Vector2::from_word(write),
			14 => self.sxy2 = Vector2::from_word(write),
			15 => self.push_sxy(write),
			16 => self.sz0 = write as u16,
			17 => self.sz1 = write as u16,
			18 => self.sz2 = write as u16,
			19 => self.sz3 = write as u16,
			20 => self.rgb[0] = Rgb::from_word(write),
			21 => self.rgb[1] = Rgb::from_word(write),
			22 => self.rgb[2] = Rgb::from_word(write),
			23 => self.res1 = write,
			24 => self.mac0 = write as i32,
			25 => self.mac1 = write as i32,
			26 => self.mac2 = write as i32,
			27 => self.mac3 = write as i32,
			28 => self.write_irgb(write),
			29 => {},
			30 => self.lzcs = write as i32,
			31 => {},

			_ => unreachable!(),
		}
	}

	fn read_control_register(&self, reg_index: u32) -> u32 {
		let result = match reg_index {
			0 => (self.rot_matrix.m11 as u16 as u32) | ((self.rot_matrix.m12 as u16 as u32) << 16),
			1 => (self.rot_matrix.m13 as u16 as u32) | ((self.rot_matrix.m21 as u16 as u32) << 16),
			2 => (self.rot_matrix.m22 as u16 as u32) | ((self.rot_matrix.m23 as u16 as u32) << 16),
			3 => (self.rot_matrix.m31 as u16 as u32) | ((self.rot_matrix.m32 as u16 as u32) << 16),
			4 => self.rot_matrix.m33 as u32,
			5 => self.translation_vec.x as u32,
			6 => self.translation_vec.y as u32,
			7 => self.translation_vec.z as u32,
			8 => (self.light_src_matrix.m11 as u16 as u32) | ((self.light_src_matrix.m12 as u16 as u32) << 16),
			9 => (self.light_src_matrix.m13 as u16 as u32) | ((self.light_src_matrix.m21 as u16 as u32) << 16),
			10 => (self.light_src_matrix.m22 as u16 as u32) | ((self.light_src_matrix.m23 as u16 as u32) << 16),
			11 => (self.light_src_matrix.m31 as u16 as u32) | ((self.light_src_matrix.m32 as u16 as u32) << 16),
			12 => self.light_src_matrix.m33 as u32,
			13 => self.bg_colour.x as u32,
			14 => self.bg_colour.y as u32,
			15 => self.bg_colour.z as u32,
			16 => (self.light_colour_matrix.m11 as u16 as u32) | ((self.light_colour_matrix.m12 as u16 as u32) << 16),
			17 => (self.light_colour_matrix.m13 as u16 as u32) | ((self.light_colour_matrix.m21 as u16 as u32) << 16),
			18 => (self.light_colour_matrix.m22 as u16 as u32) | ((self.light_colour_matrix.m23 as u16 as u32) << 16),
			19 => (self.light_colour_matrix.m31 as u16 as u32) | ((self.light_colour_matrix.m32 as u16 as u32) << 16),
			20 => self.light_colour_matrix.m33 as u32,
			21 => self.far_colour.x as u32,
			22 => self.far_colour.y as u32,
			23 => self.far_colour.z as u32,
			24 => self.screen_offset.x as u32,
			25 => self.screen_offset.y as u32,
			26 => self.h as i16 as i32 as u32,
			27 => self.dqa as u32,
			28 => self.dqb as u32,
			29 => self.zsf3 as u32,
			30 => self.zsf4 as u32,
			31 => self.flag | (u32::from(self.flag & 0x7F87E000 != 0) << 31),

			_ => unreachable!(),
		};

		result
	}

	fn write_control_register(&mut self, reg_index: u32, write: u32) {

		match reg_index {
			0 => { self.rot_matrix.m11 = write as i16; self.rot_matrix.m12 = (write >> 16) as i16; },
			1 => { self.rot_matrix.m13 = write as i16; self.rot_matrix.m21 = (write >> 16) as i16; },
			2 => { self.rot_matrix.m22 = write as i16; self.rot_matrix.m23 = (write >> 16) as i16; },
			3 => { self.rot_matrix.m31 = write as i16; self.rot_matrix.m32 = (write >> 16) as i16; },
			4 => self.rot_matrix.m33 = write as i16,
			5 => self.translation_vec.x = write as i32,
			6 => self.translation_vec.y = write as i32,
			7 => self.translation_vec.z = write as i32,
			8 => { self.light_src_matrix.m11 = write as i16; self.light_src_matrix.m12 = (write >> 16) as i16; },
			9 => { self.light_src_matrix.m13 = write as i16; self.light_src_matrix.m21 = (write >> 16) as i16; },
			10 => { self.light_src_matrix.m22 = write as i16; self.light_src_matrix.m23 = (write >> 16) as i16; },
			11 => { self.light_src_matrix.m31 = write as i16; self.light_src_matrix.m32 = (write >> 16) as i16; },
			12 => self.light_src_matrix.m33 = write as i16,
			13 => self.bg_colour.x = write as i32,
			14 => self.bg_colour.y = write as i32,
			15 => self.bg_colour.z = write as i32,
			16 => { self.light_colour_matrix.m11 = write as i16; self.light_colour_matrix.m12 = (write >> 16) as i16; },
			17 => { self.light_colour_matrix.m13 = write as i16; self.light_colour_matrix.m21 = (write >> 16) as i16; },
			18 => { self.light_colour_matrix.m22 = write as i16; self.light_colour_matrix.m23 = (write >> 16) as i16; },
			19 => { self.light_colour_matrix.m31 = write as i16; self.light_colour_matrix.m32 = (write >> 16) as i16; },
			20 => self.light_colour_matrix.m33 = write as i16,
			21 => self.far_colour.x = write as i32,
			22 => self.far_colour.y = write as i32,
			23 => self.far_colour.z = write as i32,
			24 => self.screen_offset.x = write as i32,
			25 => self.screen_offset.y = write as i32,
			26 => self.h = write as u16,
			27 => self.dqa = write as i16,
			28 => self.dqb = write as i32,
			29 => self.zsf3 = write as i16,
			30 => self.zsf4 = write as i16,
			31 => self.flag = write & 0x7FFFF000,

			_ => unreachable!(),
		}
	}

	// Expands 5:5:5 bit RGB (range 0..1Fh) to 16:16:16 bit RGB (range 0000h..0F80h)
	fn write_irgb(&mut self, write: u32) {
		self.ir1 = ((write & 0x1F) * 0x80) as i16;
		self.ir2 = (((write >> 5) & 0x1F) * 0x80) as i16;
		self.ir3 = (((write >> 10) & 0x1F) * 0x80) as i16;
	}

	// Collapses 16:16:16 bit RGB (range 0000h..0F80h) to 5:5:5 bit RGB (range 0..1Fh)
	fn read_orgb(&self) -> u32 {
		let r = self.ir1 / 0x80;
		let g = self.ir2 / 0x80;
		let b = self.ir3 / 0x80;

		(r.clamp(0, 0x1F)) as u32
			| (g.clamp(0, 0x1F) << 5) as u32
			| (b.clamp(0, 0x1F) << 10) as u32
	}

	fn read_lzcr(&self) -> u32 {
		if self.lzcs >= 0 {
			self.lzcs.leading_zeros()
		} else {
			self.lzcs.leading_ones()
		}
	}

	fn push_sxy(&mut self, word: u32) {
		self.sxy0 = self.sxy1;
		self.sxy1 = self.sxy2;
		self.sxy2 = Vector2::from_word(word);
	}
}

#[derive(Clone, Copy)]
struct GteInstruction {
	raw: u32
}

impl GteInstruction {
	fn from_raw(raw: u32) -> Self {
		Self { raw }
	}

	fn opcode(&self) -> u32 {
		self.raw & 0x3F
	}

	// shift fraction bit
	fn sf(&self) -> u32 {
		(self.raw >> 19) & 1
	}

	// saturate bit
	fn lm(&self) -> bool {
		(self.raw >> 10) & 1 != 0
	}

	// MVMVA multiply matrix
	fn mx(&self) -> u32 {
		(self.raw >> 17) & 3
	}

	// MVMVA multiply vector
	fn vx(&self) -> u32 {
		(self.raw >> 15) & 3
	}

	// MVMVA translation vector
	fn tx(&self) -> u32 {
		(self.raw >> 13) & 3
	}
}

pub struct Gte {
	regs: GteRegisters,
}

impl Gte {
	pub fn new() -> Self {
		Self {
			regs: GteRegisters::new(),
		}
	}

	pub fn decode_and_exec(&mut self, instr_raw: u32) {
		let instr = GteInstruction::from_raw(instr_raw);

		self.regs.flag = 0;

		match instr.opcode() {
			0x01 => self.op_rtps(instr),
			0x06 => self.op_nclip(),
			0x0C => self.op_op(instr),
			0x10 => self.op_dpcs(instr),
			0x11 => self.op_intpl(instr),
			0x12 => self.op_mvmva(instr),
			0x13 => self.op_ncds(instr),
			0x14 => self.op_cdp(instr),
			0x16 => self.op_ncdt(instr),
			0x1B => self.op_nccs(instr),
			0x1C => self.op_cc(instr),
			0x1E => self.op_ncs(instr),
			0x20 => self.op_nct(instr),
			0x28 => self.op_sqr(instr),
			0x29 => self.op_dcpl(instr),
			0x2A => self.op_dpct(instr),
			0x2D => self.op_avsz3(),
			0x2E => self.op_avsz4(),
			0x30 => self.op_rtpt(instr),
			0x3D => self.op_gpf(instr),
			0x3E => self.op_gpl(instr),
			0x3F => self.op_ncct(instr),

			_ => unimplemented!("GTE instruction 0x{:X}", instr.opcode())
		}

	}

	pub fn read_data_reg(&self, reg_index: u32) -> u32 {
		self.regs.read_data_register(reg_index)
	}

	pub fn write_data_reg(&mut self, reg_index: u32, write: u32) {
		self.regs.write_data_register(reg_index, write);
	}

	pub fn read_control_reg(&self, reg_index: u32) -> u32 {
		self.regs.read_control_register(reg_index)
	}

	pub fn write_control_reg(&mut self, reg_index: u32, write: u32) {
		self.regs.write_control_register(reg_index, write);
	}

	// mac overflows instead of saturating
	fn clamp_mac(&mut self, mac_num: u32, set: i64, sf: u32) -> i32 {
		if mac_num == 3 {
			self.regs.mac3_unclamped = (set << 20) >> 20;
		}

		if set > I44_MAX {
			self.regs.flag |= (1 << MAC1_OVERFLOW) >> (mac_num - 1);
		} else if set < I44_MIN {
			self.regs.flag |= (1 << MAC1_UNDERFLOW) >> (mac_num - 1);
		}

		(((set << 20) >> 20) >> (12 * sf as u64)) as i32
	}
	
	fn check_mac(&mut self, mac_num: u32, set: i64) -> i64 {
		if set > I44_MAX {
			self.regs.flag |= (1 << MAC1_OVERFLOW) >> (mac_num - 1);
		} else if set < I44_MIN {
			self.regs.flag |= (1 << MAC1_UNDERFLOW) >> (mac_num - 1);
		}

		(set << 20) >> 20
	}

	fn clamp_mac0(&mut self, set: i64) -> i32 {
		self.regs.mac0_unclamped = set;

		if set > (i32::MAX as i64) {
			self.regs.flag |= 1 << 16;
		} else if set < (i32::MIN as i64) {
			self.regs.flag |= 1 << 15;
		}

		set as i32
	}

	fn clamp_ir(&mut self, ir_num: u32, set: i32, lm: bool) -> i16 {
		if lm && set < 0 {
			self.regs.flag |= (1 << IR1_SATURATED) >> (ir_num - 1);

			return 0;
		} else if !lm && set < -0x8000 {
			self.regs.flag |= (1 << IR1_SATURATED) >> (ir_num - 1);

			return -0x8000;
		} else if set > 0x7FFF {
			self.regs.flag |= (1 << IR1_SATURATED) >> (ir_num - 1);

			return 0x7FFF;
		}

		set as i16
	}

	fn clamp_ir0(&mut self, set: i32) -> i16 {
		if set < 0 {
			self.regs.flag |= 1 << IR0_SATURATED;

			return 0;
		} else if set > 0x1000 {
			self.regs.flag |= 1 << IR0_SATURATED;

			return 0x1000;
		}

		return set as i16;
	}

	// emulates a hardware bug in RTPx which sets the IR3 flag incorrectly
	fn clamp_ir3_z(&mut self, set: i64, sf: u32, lm: bool) -> i16 {
		let mac_sf = (set >> sf * 12) as i32;
		let mac_12 = (set >> 12) as i32;

		let min = if lm { 0 } else { -0x8000 };

		if mac_12 < -0x8000 || mac_12 > 0x7FFF {
			self.regs.flag |= 1 << 22;
		}

		mac_sf.clamp(min, 0x7FFF) as i16
	}

	fn clamp_otz(&mut self, set: i64) -> u16 {
		if set > 0xFFFF {
			self.regs.flag |= 1 << 18;

			return 0xFFFF;
		} else if set < 0 {
			self.regs.flag |= 1 << 18;

			return 0;
		}

		set as u16
	}

	fn clamp_rgb_component(&mut self, comp_num: i32, component: i32) -> u8 {
		if component < 0 {
			self.regs.flag |= (1 << COLOUR_R_SATURATED) >> (comp_num - 1);

			return 0;
		} else if component > 0xFF {
			self.regs.flag |= (1 << COLOUR_R_SATURATED) >> (comp_num - 1);

			return 0xFF;
		}

		component as u8
	}

	fn clamp_sxy(&mut self, comp_num: i32, component: i64) -> i16 {
		if component < -0x400 {
			self.regs.flag |= (1 << SX2_SATURATED) >> (comp_num - 1);

			return -0x400;
		} else if component > 0x3FF {
			self.regs.flag |= (1 << SX2_SATURATED) >> (comp_num - 1);

			return 0x3FF;
		}

		component as i32 as i16
	}

	fn set_ir(&mut self, lm: bool) {
		self.regs.ir1 = self.clamp_ir(1, self.regs.mac1, lm);
		self.regs.ir2 = self.clamp_ir(2, self.regs.mac2, lm);
		self.regs.ir3 = self.clamp_ir(3, self.regs.mac3, lm);
	}

	fn push_colour_fifo(&mut self, instr: GteInstruction) {
		self.regs.rgb[0] = self.regs.rgb[1];
		self.regs.rgb[1] = self.regs.rgb[2];
		
		self.regs.rgb[2].r = self.clamp_rgb_component(1, self.regs.mac1 >> 4);
		self.regs.rgb[2].g = self.clamp_rgb_component(2, self.regs.mac2 >> 4);
		self.regs.rgb[2].b = self.clamp_rgb_component(3, self.regs.mac3 >> 4);
		self.regs.rgb[2].c = self.regs.rgbc.c;

		self.set_ir(instr.lm());
	}

	fn interp_far_colour(&mut self, instr: GteInstruction, m1: u64, m2: u64, m3: u64) {
		let mac1 = self.clamp_mac(1, (((self.regs.far_colour.x as u64) << 12) - m1) as i64, instr.sf());
		let mac2 = self.clamp_mac(2, (((self.regs.far_colour.y as u64) << 12) - m2) as i64, instr.sf());
		let mac3 = self.clamp_mac(3, (((self.regs.far_colour.z as u64) << 12) - m3) as i64, instr.sf());

		// saturation always behaves as if lm=0 for this step
		let ir1 = self.clamp_ir(1, mac1, false) as i64;
		let ir2 = self.clamp_ir(2, mac2, false) as i64;
		let ir3 = self.clamp_ir(3, mac3, false) as i64;

		self.regs.mac1 = self.clamp_mac(1, (m1 as i64) + ((self.regs.ir0 as i64) * ir1), instr.sf());
		self.regs.mac2 = self.clamp_mac(2, (m2 as i64) + ((self.regs.ir0 as i64) * ir2), instr.sf());
		self.regs.mac3 = self.clamp_mac(3, (m3 as i64) + ((self.regs.ir0 as i64) * ir3), instr.sf());
	}

	fn interp_light_colour(&mut self, instr: GteInstruction, vec_num: usize) {
		// [IR1,IR2,IR3] = [MAC1,MAC2,MAC3] = (LLM*V0) SAR (sf*12)
		self.regs.mac1 = self.clamp_mac(1, (self.regs.light_src_matrix.m11 as i64 * self.regs.v[vec_num].x as i64)
			+ (self.regs.light_src_matrix.m12 as i64 * self.regs.v[vec_num].y as i64)
			+ (self.regs.light_src_matrix.m13 as i64 * self.regs.v[vec_num].z as i64), 
			instr.sf());
		self.regs.mac2 = self.clamp_mac(2, (self.regs.light_src_matrix.m21 as i64 * self.regs.v[vec_num].x as i64)
			+ (self.regs.light_src_matrix.m22 as i64 * self.regs.v[vec_num].y as i64)
			+ (self.regs.light_src_matrix.m23 as i64 * self.regs.v[vec_num].z as i64), 
			instr.sf());
		self.regs.mac3 = self.clamp_mac(3, (self.regs.light_src_matrix.m31 as i64 * self.regs.v[vec_num].x as i64)
			+ (self.regs.light_src_matrix.m32 as i64 * self.regs.v[vec_num].y as i64)
			+ (self.regs.light_src_matrix.m33 as i64 * self.regs.v[vec_num].z as i64), 
			instr.sf());

		self.set_ir(instr.lm());

		// [IR1,IR2,IR3] = [MAC1,MAC2,MAC3] = (BK*1000h + LCM*IR) SAR (sf*12)
		self.regs.mac1 = {
			let mut mac = self.check_mac(1, ((self.regs.bg_colour.x as i64) << 12) + (self.regs.light_colour_matrix.m11 as i64 * self.regs.ir1 as i64));
			mac = self.check_mac(1, mac + self.regs.light_colour_matrix.m12 as i64 * self.regs.ir2 as i64);

			self.clamp_mac(1, mac + self.regs.light_colour_matrix.m13 as i64 * self.regs.ir3 as i64, instr.sf())
		};
		self.regs.mac2 = {
			let mut mac = self.check_mac(2, ((self.regs.bg_colour.y as i64) << 12) + (self.regs.light_colour_matrix.m21 as i64 * self.regs.ir1 as i64));
			mac = self.check_mac(2, mac + self.regs.light_colour_matrix.m22 as i64 * self.regs.ir2 as i64);

			self.clamp_mac(2, mac + self.regs.light_colour_matrix.m23 as i64 * self.regs.ir3 as i64, instr.sf())
		};
		self.regs.mac3 = {
			let mut mac = self.check_mac(3, ((self.regs.bg_colour.z as i64) << 12) + (self.regs.light_colour_matrix.m31 as i64 * self.regs.ir1 as i64));
			mac = self.check_mac(3, mac + self.regs.light_colour_matrix.m32 as i64 * self.regs.ir2 as i64);

			self.clamp_mac(3, mac + self.regs.light_colour_matrix.m33 as i64 * self.regs.ir3 as i64, instr.sf())
		};

		self.set_ir(instr.lm());
	}

	// copy of fogstations copy of duckstations unr dvivision impl
	fn divide(&mut self, lhs: u32, rhs: u32) -> u32 {
		if lhs < rhs * 2 {
			let shift = (rhs as u16).leading_zeros();
			let lhs_shift = lhs << shift;
			let rhs_shift = rhs << shift;

			let divisor = rhs_shift | 0x8000;

			let x: i32 = 0x101 + UNR_TABLE[(((divisor & 0x7FFF) + 0x40) >> 7) as usize] as i32;
			let d: i32 = ((divisor as i32 * -x) + 0x80) >> 8;

			let recip = ((x * (0x20000 + d) + 0x80) >> 8) as u32;

			let result = ((lhs_shift as u64 * recip as u64) + 0x8000) >> 16;

			return (result as u32).min(0x1FFFF);
		} else {
			self.regs.flag |= 1 << DIVIDE_OVERFLOW;

			return 0x1FFFF;
		}
	}

	fn do_rtp(&mut self, instr: GteInstruction, vec_num: usize, depth_queue: bool) {
		self.regs.mac1 = {
			let mut mac = self.check_mac(1,((self.regs.translation_vec.x as i64) << 12) + (self.regs.rot_matrix.m11 as i64 * self.regs.v[vec_num].x as i64));
			mac = self.check_mac(1, mac + (self.regs.rot_matrix.m12 as i64 * self.regs.v[vec_num].y as i64));

			self.clamp_mac(1, mac + (self.regs.rot_matrix.m13 as i64 * self.regs.v[vec_num].z as i64), instr.sf())
		};
		self.regs.mac2 = {
			let mut mac = self.check_mac(2,((self.regs.translation_vec.y as i64) << 12) + (self.regs.rot_matrix.m21 as i64 * self.regs.v[vec_num].x as i64));
			mac = self.check_mac(2, mac + (self.regs.rot_matrix.m22 as i64 * self.regs.v[vec_num].y as i64));

			self.clamp_mac(2, mac + (self.regs.rot_matrix.m23 as i64 * self.regs.v[vec_num].z as i64), instr.sf())
		};
		self.regs.mac3 = {
			let mut mac = self.check_mac(3,((self.regs.translation_vec.z as i64) << 12) + (self.regs.rot_matrix.m31 as i64 * self.regs.v[vec_num].x as i64));
			mac = self.check_mac(3, mac + (self.regs.rot_matrix.m32 as i64 * self.regs.v[vec_num].y as i64));

			self.clamp_mac(3, mac + (self.regs.rot_matrix.m33 as i64 * self.regs.v[vec_num].z as i64), instr.sf())
		};

		self.regs.ir1 = self.clamp_ir(1, self.regs.mac1, instr.lm());
		self.regs.ir2 = self.clamp_ir(2, self.regs.mac2, instr.lm());
		self.regs.ir3 = self.clamp_ir3_z(self.regs.mac3_unclamped as i64, instr.sf(), instr.lm());

		// push to SZ FIFO
		self.regs.sz0 = self.regs.sz1;
		self.regs.sz1 = self.regs.sz2;
		self.regs.sz2 = self.regs.sz3;
		self.regs.sz3 = self.clamp_otz(self.regs.mac3_unclamped >> 12); // OTZ and SZ3 have the same limiter

		let div = self.divide(self.regs.h as u32, self.regs.sz3 as u32) as i64;

		self.regs.sxy0 = self.regs.sxy1;
		self.regs.sxy1 = self.regs.sxy2;
		
		let sx = div * (self.regs.ir1 as i64) + (self.regs.screen_offset.x as i32 as i64);
		self.clamp_mac0(sx);

		let sy = div * (self.regs.ir2 as i64) + (self.regs.screen_offset.y as i32 as i64);
		self.clamp_mac0(sy);

		self.regs.sxy2.x = self.clamp_sxy(1, sx >> 16);
		self.regs.sxy2.y = self.clamp_sxy(2, sy >> 16);

		if depth_queue {
			self.regs.mac0 = self.clamp_mac0(self.regs.dqb as i64 + (self.regs.dqa as i64 * div));
			self.regs.ir0 = self.clamp_ir0((self.regs.mac0_unclamped >> 12) as i32);
		}
	}

	fn do_depth_queue(&mut self, instr: GteInstruction, rgb: Rgb) {
		self.interp_far_colour(
			instr, 
			(rgb.r as u64) << 16, 
			(rgb.g as u64) << 16, 
			(rgb.b as u64) << 16, 
		);

		self.push_colour_fifo(instr);
	}

	fn do_ncd(&mut self, instr: GteInstruction, vec_num: usize) {
		self.interp_light_colour(instr, vec_num);

		self.interp_far_colour(
			instr, 
			(((self.regs.rgbc.r as i64) << 4) * self.regs.ir1 as i64) as u64,
			(((self.regs.rgbc.g as i64) << 4) * self.regs.ir2 as i64) as u64,
			(((self.regs.rgbc.b as i64) << 4) * self.regs.ir3 as i64) as u64,
		);

		self.push_colour_fifo(instr);
	}

	fn do_nc(&mut self, instr: GteInstruction, vec_num: usize) {
		self.interp_light_colour(instr, vec_num);

		self.push_colour_fifo(instr);
	}

	fn do_ncc(&mut self, instr: GteInstruction, vec_num: usize) {
		self.interp_light_colour(instr, vec_num);

		self.regs.mac1 = self.clamp_mac(1, ((self.regs.rgbc.r as i64) << 4) * self.regs.ir1 as i64, instr.sf());
		self.regs.mac2 = self.clamp_mac(2, ((self.regs.rgbc.g as i64) << 4) * self.regs.ir2 as i64, instr.sf());
		self.regs.mac3 = self.clamp_mac(3, ((self.regs.rgbc.b as i64) << 4) * self.regs.ir3 as i64, instr.sf());

		self.push_colour_fifo(instr);
	}

	fn op_rtps(&mut self, instr: GteInstruction) {
		self.do_rtp(instr, 0, true);
	}

	fn op_rtpt(&mut self, instr: GteInstruction) {
		self.do_rtp(instr, 0, false);
		self.do_rtp(instr, 1, false);
		self.do_rtp(instr, 2, true);
	}

	fn op_sqr(&mut self, instr: GteInstruction) {
		self.regs.mac1 = self.clamp_mac(1, i64::from(self.regs.ir1).pow(2), instr.sf());
		self.regs.mac2 = self.clamp_mac(2, i64::from(self.regs.ir2).pow(2), instr.sf());
		self.regs.mac3 = self.clamp_mac(3, i64::from(self.regs.ir3).pow(2), instr.sf());

		self.set_ir(instr.lm());
	}

	fn op_nclip(&mut self) {
		let dot_product = (i64::from(self.regs.sxy0.x) * i64::from(self.regs.sxy1.y))
			+ (i64::from(self.regs.sxy1.x) * i64::from(self.regs.sxy2.y))
			+ (i64::from(self.regs.sxy2.x) * i64::from(self.regs.sxy0.y))
			- (i64::from(self.regs.sxy0.x) * i64::from(self.regs.sxy2.y))
			- (i64::from(self.regs.sxy1.x) * i64::from(self.regs.sxy0.y))
			- (i64::from(self.regs.sxy2.x) * i64::from(self.regs.sxy1.y));

		self.regs.mac0 = self.clamp_mac0(dot_product);
	}

	fn op_avsz3(&mut self) {
		let avg_z = i64::from(self.regs.zsf3) * (u32::from(self.regs.sz1) + u32::from(self.regs.sz2) + u32::from(self.regs.sz3)) as i64;

		self.regs.mac0 = self.clamp_mac0(avg_z);
		self.regs.otz = self.clamp_otz((avg_z >> 12) as i64);
	}

	fn op_avsz4(&mut self) {
		let avg_z = i64::from(self.regs.zsf4) * (u32::from(self.regs.sz0) + u32::from(self.regs.sz1) + u32::from(self.regs.sz2) + u32::from(self.regs.sz3)) as i64;

		self.regs.mac0 = self.clamp_mac0(avg_z);
		self.regs.otz = self.clamp_otz((avg_z >> 12) as i64);
	}

	// outer product is a mistranslation of cross product
	fn op_op(&mut self, instr: GteInstruction) {
		self.regs.mac1 = self.clamp_mac(1, (i64::from(self.regs.rot_matrix.m22) * i64::from(self.regs.ir3)) - (i64::from(self.regs.rot_matrix.m33) * i64::from(self.regs.ir2)), instr.sf());
		self.regs.mac2 = self.clamp_mac(2, (i64::from(self.regs.rot_matrix.m33) * i64::from(self.regs.ir1)) - (i64::from(self.regs.rot_matrix.m11) * i64::from(self.regs.ir3)), instr.sf());
		self.regs.mac3 = self.clamp_mac(3, (i64::from(self.regs.rot_matrix.m11) * i64::from(self.regs.ir2)) - (i64::from(self.regs.rot_matrix.m22) * i64::from(self.regs.ir1)), instr.sf());

		self.set_ir(instr.lm());
	}

	fn op_gpf(&mut self, instr: GteInstruction) {
		self.regs.mac1 = self.clamp_mac(1, i64::from(self.regs.ir0) * i64::from(self.regs.ir1), instr.sf());
		self.regs.mac2 = self.clamp_mac(2, i64::from(self.regs.ir0) * i64::from(self.regs.ir2), instr.sf());
		self.regs.mac3 = self.clamp_mac(3, i64::from(self.regs.ir0) * i64::from(self.regs.ir3), instr.sf());

		self.set_ir(instr.lm());

		self.push_colour_fifo(instr);
	}

	fn op_gpl(&mut self, instr: GteInstruction) {
		self.regs.mac1 = self.clamp_mac(1, (i64::from(self.regs.mac1) << (instr.sf() * 12)) + (i64::from(self.regs.ir0) * i64::from(self.regs.ir1)), instr.sf());
		self.regs.mac2 = self.clamp_mac(2, (i64::from(self.regs.mac2) << (instr.sf() * 12)) + (i64::from(self.regs.ir0) * i64::from(self.regs.ir2)), instr.sf());
		self.regs.mac3 = self.clamp_mac(3, (i64::from(self.regs.mac3) << (instr.sf() * 12)) + (i64::from(self.regs.ir0) * i64::from(self.regs.ir3)), instr.sf());

		self.set_ir(instr.lm());

		self.push_colour_fifo(instr);
	}

	fn op_dpcs(&mut self, instr: GteInstruction) {
		self.do_depth_queue(instr, self.regs.rgbc);
	}

	fn op_dpct(&mut self, instr: GteInstruction) {
		self.do_depth_queue(instr, self.regs.rgb[0]);
		self.do_depth_queue(instr, self.regs.rgb[0]);
		self.do_depth_queue(instr, self.regs.rgb[0]);
	}

	fn op_intpl(&mut self, instr: GteInstruction) {
		self.interp_far_colour(instr, 
			(self.regs.ir1 as u64) << 12, 
			(self.regs.ir2 as u64) << 12, 
			(self.regs.ir3 as u64) << 12, 
		);

		self.push_colour_fifo(instr);
	}

	fn op_mvmva(&mut self, instr: GteInstruction) {
		let matrix = match instr.mx() {
			0 => self.regs.rot_matrix,
			1 => self.regs.light_src_matrix,
			2 => self.regs.light_colour_matrix,
			// mx=3 is reserved (returns garbage matrix)
			3 => Matrix3x3 {
				m11: -(i16::from(self.regs.rgbc.r) << 4),
				m12: (i16::from(self.regs.rgbc.r) << 4),
				m13: self.regs.ir0,
				m21: self.regs.rot_matrix.m13,
				m22: self.regs.rot_matrix.m13,
				m23: self.regs.rot_matrix.m13,
				m31: self.regs.rot_matrix.m22,
				m32: self.regs.rot_matrix.m22,
				m33: self.regs.rot_matrix.m22,
			},

			_ => unreachable!(),
		};

		let vector = match instr.vx() {
			0 => self.regs.v[0],
			1 => self.regs.v[1],
			2 => self.regs.v[2],
			// vx=3 selects [IR1, IR2, IR3] as the vector
			3 => Vector3 { x: self.regs.ir1, y: self.regs.ir2, z: self.regs.ir3 },

			_ => unreachable!()
		};

		let tr_vector =  match instr.tx() {
			0 => self.regs.translation_vec,
			1 => self.regs.bg_colour,
			2 => self.regs.far_colour,
			// selecting tx=3 uses an empty vector (i think)
			3 => Vector3_32 { x: 0, y: 0, z: 0 },

			_ => unreachable!()
		};

		// selecting tx=2 has a hardware bug where the results aren't calculated correctly
		if instr.tx() == 2 {
			// bug: the first part of the equation is not included in the final result, however flags are still calculated as normal
			self.regs.mac1 = {
				let m_vy = self.check_mac(1, matrix.m12 as i64 * vector.y as i64);
				self.clamp_mac(1, m_vy + (matrix.m13 as i64 * vector.z as i64), instr.sf())
			};
			self.regs.mac2 = {
				let m_vy = self.check_mac(2, matrix.m22 as i64 * vector.y as i64);
				self.clamp_mac(2, m_vy + (matrix.m23 as i64 * vector.z as i64), instr.sf())
			};
			self.regs.mac3 = {
				let m_vy = self.check_mac(3, matrix.m32 as i64 * vector.y as i64);
				self.clamp_mac(3, m_vy + (matrix.m33 as i64 * vector.z as i64), instr.sf())
			};

			// set flags for missing part of calculation
			let mac1 = self.clamp_mac(1, ((tr_vector.x as i64) << 12) + (matrix.m11 as i64 * vector.x as i64), instr.sf());
			let mac2 = self.clamp_mac(2, ((tr_vector.y as i64) << 12) + (matrix.m21 as i64 * vector.x as i64), instr.sf());
			let mac3 = self.clamp_mac(3, ((tr_vector.z as i64) << 12) + (matrix.m31 as i64 * vector.x as i64), instr.sf());

			self.clamp_ir(1, mac1, false);
			self.clamp_ir(2, mac2, false);
			self.clamp_ir(3, mac3, false);

		} else {
			self.regs.mac1 = {
				let mut mac = self.check_mac(1,((tr_vector.x as i64) << 12) + (matrix.m11 as i64 * vector.x as i64));
				mac = self.check_mac(1, mac + (matrix.m12 as i64 * vector.y as i64));

				self.clamp_mac(1, mac + (matrix.m13 as i64 * vector.z as i64), instr.sf())
			};
			self.regs.mac2 = {
				let mut mac = self.check_mac(2,((tr_vector.y as i64) << 12) + (matrix.m21 as i64 * vector.x as i64));
				mac = self.check_mac(2, mac + (matrix.m22 as i64 * vector.y as i64));

				self.clamp_mac(2, mac + (matrix.m23 as i64 * vector.z as i64), instr.sf())
			};
			self.regs.mac3 = {
				let mut mac = self.check_mac(3,((tr_vector.z as i64) << 12) + (matrix.m31 as i64 * vector.x as i64));
				mac = self.check_mac(3, mac + (matrix.m32 as i64 * vector.y as i64));

				self.clamp_mac(3, mac + (matrix.m33 as i64 * vector.z as i64), instr.sf())
			};
		}

		self.set_ir(instr.lm());
	}

	fn op_ncds(&mut self, instr: GteInstruction) {
		self.do_ncd(instr, 0);
	}

	fn op_ncdt(&mut self, instr: GteInstruction) {
		self.do_ncd(instr, 0);
		self.do_ncd(instr, 1);
		self.do_ncd(instr, 2);
	}

	fn op_ncs(&mut self, instr: GteInstruction) {
		self.do_nc(instr, 0);
	}

	fn op_nct(&mut self, instr: GteInstruction) {
		self.do_nc(instr, 0);
		self.do_nc(instr, 1);
		self.do_nc(instr, 2);
	}

	fn op_nccs(&mut self, instr: GteInstruction) {
		self.do_ncc(instr, 0);
	}

	fn op_ncct(&mut self, instr: GteInstruction) {
		self.do_ncc(instr, 0);
		self.do_ncc(instr, 1);
		self.do_ncc(instr, 2);
	}

	fn op_cc(&mut self, instr: GteInstruction) {
		self.regs.mac1 = {
			let mut mac = self.check_mac(1, ((self.regs.bg_colour.x as i64) << 12) + (self.regs.light_colour_matrix.m11 as i64 * self.regs.ir1 as i64));
			mac = self.check_mac(1, mac + self.regs.light_colour_matrix.m12 as i64 * self.regs.ir2 as i64);

			self.clamp_mac(1, mac + self.regs.light_colour_matrix.m13 as i64 * self.regs.ir3 as i64, instr.sf())
		};
		self.regs.mac2 = {
			let mut mac = self.check_mac(2, ((self.regs.bg_colour.y as i64) << 12) + (self.regs.light_colour_matrix.m21 as i64 * self.regs.ir1 as i64));
			mac = self.check_mac(2, mac + self.regs.light_colour_matrix.m22 as i64 * self.regs.ir2 as i64);

			self.clamp_mac(2, mac + self.regs.light_colour_matrix.m23 as i64 * self.regs.ir3 as i64, instr.sf())
		};
		self.regs.mac3 = {
			let mut mac = self.check_mac(3, ((self.regs.bg_colour.z as i64) << 12) + (self.regs.light_colour_matrix.m31 as i64 * self.regs.ir1 as i64));
			mac = self.check_mac(3, mac + self.regs.light_colour_matrix.m32 as i64 * self.regs.ir2 as i64);

			self.clamp_mac(3, mac + self.regs.light_colour_matrix.m33 as i64 * self.regs.ir3 as i64, instr.sf())
		};

		self.set_ir(instr.lm());

		self.regs.mac1 = self.clamp_mac(1, ((self.regs.rgbc.r as i64) << 4) * self.regs.ir1 as i64, instr.sf());
		self.regs.mac2 = self.clamp_mac(2, ((self.regs.rgbc.g as i64) << 4) * self.regs.ir2 as i64, instr.sf());
		self.regs.mac3 = self.clamp_mac(3, ((self.regs.rgbc.b as i64) << 4) * self.regs.ir3 as i64, instr.sf());

		self.push_colour_fifo(instr);
	}

	fn op_cdp(&mut self, instr: GteInstruction) {
		self.regs.mac1 = {
			let mut mac = self.check_mac(1, ((self.regs.bg_colour.x as i64) << 12) + (self.regs.light_colour_matrix.m11 as i64 * self.regs.ir1 as i64));
			mac = self.check_mac(1, mac + self.regs.light_colour_matrix.m12 as i64 * self.regs.ir2 as i64);

			self.clamp_mac(1, mac + self.regs.light_colour_matrix.m13 as i64 * self.regs.ir3 as i64, instr.sf())
		};
		self.regs.mac2 = {
			let mut mac = self.check_mac(2, ((self.regs.bg_colour.y as i64) << 12) + (self.regs.light_colour_matrix.m21 as i64 * self.regs.ir1 as i64));
			mac = self.check_mac(2, mac + self.regs.light_colour_matrix.m22 as i64 * self.regs.ir2 as i64);

			self.clamp_mac(2, mac + self.regs.light_colour_matrix.m23 as i64 * self.regs.ir3 as i64, instr.sf())
		};
		self.regs.mac3 = {
			let mut mac = self.check_mac(3, ((self.regs.bg_colour.z as i64) << 12) + (self.regs.light_colour_matrix.m31 as i64 * self.regs.ir1 as i64));
			mac = self.check_mac(3, mac + self.regs.light_colour_matrix.m32 as i64 * self.regs.ir2 as i64);

			self.clamp_mac(3, mac + self.regs.light_colour_matrix.m33 as i64 * self.regs.ir3 as i64, instr.sf())
		};

		self.set_ir(instr.lm());

		self.interp_far_colour(
			instr, 
			(((self.regs.rgbc.r as i64) << 4) * self.regs.ir1 as i64) as u64,
			(((self.regs.rgbc.g as i64) << 4) * self.regs.ir2 as i64) as u64,
			(((self.regs.rgbc.b as i64) << 4) * self.regs.ir3 as i64) as u64,
		);

		self.push_colour_fifo(instr);
	}

	fn op_dcpl(&mut self, instr: GteInstruction) {
		self.interp_far_colour(
			instr, 
			(((self.regs.rgbc.r as i64) << 4) * self.regs.ir1 as i64) as u64,
			(((self.regs.rgbc.g as i64) << 4) * self.regs.ir2 as i64) as u64,
			(((self.regs.rgbc.b as i64) << 4) * self.regs.ir3 as i64) as u64,
		);

		self.push_colour_fifo(instr);
	}

}
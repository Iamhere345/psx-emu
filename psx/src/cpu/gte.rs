use log::*;

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
}

#[derive(Default)]
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

#[derive(Default)]
struct Vector3_32 {
	x: i32,
	y: i32,
	z: i32
}

#[derive(Default, Clone, Copy)]
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

#[derive(Default)]
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

#[derive(Default)]
struct Rgb32 {
	r: u32,
	g: u32,
	b: u32
}

#[derive(Default)]
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
	v0: Vector3,
	v1: Vector3,
	v2: Vector3,
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
	rgb0: Rgb,
	rgb1: Rgb,
	rgb2: Rgb,
	// Prohibited
	res1: u32,
	// 32bit Maths Accumulators (Value)
	mac0: i32,
	// 32bit Maths Accumulators (Vector)
	mac1: i32,
	mac2: i32,
	mac3: i32,
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
	bg_colour: Rgb32,
	// Light color matrix source (3x3)
	light_colour_matrix: Matrix3x3,
	// Far color (R,G,B)
	far_colour: Rgb32,
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
			v0: Vector3::default(),
			v1: Vector3::default(),
			v2: Vector3::default(),
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
			rgb0: Rgb::default(),
			rgb1: Rgb::default(),
			rgb2: Rgb::default(),
			// Prohibited
			res1: 0,
			// 32bit Maths Accumulators (Value)
			mac0: 0,
			// 32bit Maths Accumulators (Vector)
			mac1: 0,
			mac2: 0,
			mac3: 0,
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
			bg_colour: Rgb32::default(),
			// Light color matrix source (3x3)
			light_colour_matrix: Matrix3x3::default(),
			// Far color (R,G,B)
			far_colour: Rgb32::default(),
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
			0 => (self.v0.x as u16 as u32) | ((self.v0.y as u16 as u32) << 16),
			1 => self.v0.z as i32 as u32,
			2 => (self.v1.x as u16 as u32) | ((self.v1.y as u16 as u32) << 16),
			3 => self.v1.z as u32,
			4 => (self.v2.x as u16 as u32) | ((self.v2.y as u16 as u32) << 16),
			5 => self.v2.z as u32,
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
			20 => self.rgb0.as_word(),
			21 => self.rgb1.as_word(),
			22 => self.rgb2.as_word(),
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

		trace!("Read $cop2r{reg_index}, got 0x{result:X}");

		result
	}

	fn write_data_register(&mut self, reg_index: u32, write: u32) {

		trace!("[{reg_index}] write 0x{write:X}");

		match reg_index {
			0 => self.v0.from_word(write),
			1 => self.v0.z = write as i16,
			2 => self.v1.from_word(write),
			3 => self.v1.z = write as i16,
			4 => self.v2.from_word(write),
			5 => self.v2.z = write as i16,
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
			20 => self.rgb0 = Rgb::from_word(write),
			21 => self.rgb1 = Rgb::from_word(write),
			22 => self.rgb2 = Rgb::from_word(write),
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
			13 => self.bg_colour.r,
			14 => self.bg_colour.g,
			15 => self.bg_colour.b,
			16 => (self.light_colour_matrix.m11 as u16 as u32) | ((self.light_colour_matrix.m12 as u16 as u32) << 16),
			17 => (self.light_colour_matrix.m13 as u16 as u32) | ((self.light_colour_matrix.m21 as u16 as u32) << 16),
			18 => (self.light_colour_matrix.m22 as u16 as u32) | ((self.light_colour_matrix.m23 as u16 as u32) << 16),
			19 => (self.light_colour_matrix.m31 as u16 as u32) | ((self.light_colour_matrix.m32 as u16 as u32) << 16),
			20 => self.light_colour_matrix.m33 as u32,
			21 => self.far_colour.r,
			22 => self.far_colour.g,
			23 => self.far_colour.b,
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

		trace!("Read $cnt{reg_index}, got 0x{result:X}");

		result
	}

	fn write_control_register(&mut self, reg_index: u32, write: u32) {

		trace!("[ctrl{reg_index}] write 0x{write:X}");

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
			13 => self.bg_colour.r = write,
			14 => self.bg_colour.g = write,
			15 => self.bg_colour.b = write,
			16 => { self.light_colour_matrix.m11 = write as i16; self.light_colour_matrix.m12 = (write >> 16) as i16; },
			17 => { self.light_colour_matrix.m13 = write as i16; self.light_colour_matrix.m21 = (write >> 16) as i16; },
			18 => { self.light_colour_matrix.m22 = write as i16; self.light_colour_matrix.m23 = (write >> 16) as i16; },
			19 => { self.light_colour_matrix.m31 = write as i16; self.light_colour_matrix.m32 = (write >> 16) as i16; },
			20 => self.light_colour_matrix.m33 = write as i16,
			21 => self.far_colour.r = write,
			22 => self.far_colour.g = write,
			23 => self.far_colour.b = write,
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

		match instr.opcode() {
			_ => {},//unimplemented!("GTE instruction 0x{:X}", instr.opcode())
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
}
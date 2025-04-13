#![allow(dead_code)]
use std::cmp;
use log::*;

/* const DITHERING_TABLE: [[i8; 4]; 4] = [
	[-4,  0,  -3,  1],
	[2,  -2,  3,  -1],
	[-3,  1,  -4,  0],
	[3,  -1,  2,  -2],
]; */

const DITHERING_TABLE: &[[i8; 4]; 4] = &[[-4, 0, -3, 1], [2, -2, 3, -1], [-3, 1, -4, 0], [3, -1, 2, -2]];

#[derive(Debug, Clone, Copy)]
enum DrawCommand {
	CpuVramDma,
	VramCpuDma,
	VramVramDma,
	DrawRect(RectCmdParams),
	DrawPolygon(PolygonCmdParams),
	QuickFill(u32)
}

#[derive(Default)]
enum TexBitDepth {
	#[default]
	FourBit,
	EightBit,
	FiveteenBit,
}

impl TexBitDepth {
	fn from_bits(bits: u32) -> Self {
		match bits {
			0 => TexBitDepth::FourBit,
			1 => TexBitDepth::EightBit,
			2 | 3 => TexBitDepth::FiveteenBit,
			_ => unreachable!("value: {bits}")
		}
	}
}

#[derive(Debug, PartialEq, Clone, Copy)]
enum RectSize {
	Variable,
	Pixel,
	Sprite8x8,
	Sprite16x16,
}

impl RectSize {
	fn from_bits(bits: u32) -> Self {
		match bits {
			0 => Self::Variable,
			1 => Self::Pixel,
			2 => Self::Sprite8x8,
			3 => Self::Sprite16x16,
			_ => unimplemented!(),
		}
	}
}

#[derive(Debug, Clone, Copy)]
enum GP0State {
	WaitingForNextCmd,
	WaitingForParams { command: DrawCommand, index: u8, words_left: u8 },
	RecvData(VramDmaInfo),
	SendData(VramDmaInfo),
}

enum GP1State {
	WaitingForNextCmd,
}

#[derive(Debug, Clone, Copy)]
struct PolygonCmdParams {
	shaded: bool,			// true: gouraud false: flat
	vertices: u8,			// 3 / 4
	textured: bool,
	semi_transparent: bool,
	raw_texture: bool,		// true: raw texture false: modulated
	colour: Colour,			// colour of first point if shaded or all points if flat
	clut: Vertex,
}

#[derive(Debug, Clone, Copy)]
struct RectCmdParams {
	size_type: RectSize,
	textured: bool,
	semi_transparent: bool,
	raw_texture: bool,
	colour: Colour,
	position: Vertex,
	clut: Vertex,
	size: Vertex,
}

#[derive(Debug, Clone, Copy)]
struct VramDmaInfo {
	dest_x: u16,
	dest_y: u16,

	width: u16,
	height: u16,

	current_row: u16,
	current_col: u16,

	halfwords_left: u16
}

// set by GP0 $E1, some fields are set by textured cmds
#[derive(Default)]
struct TexturePage {
	x_base: u32,
	y_base: u32,
	bit_depth: TexBitDepth,
	dithering: bool,
	flip_x: bool,
	flip_y: bool,
}

#[derive(Debug, Clone, Copy, Default)]
struct Colour {
	r: u8,
	g: u8,
	b: u8,
}

impl Colour {
	fn from_rgb888(r: u8, g: u8, b: u8) -> Self {
		Self { r, g, b }
	}

	fn rgb888_to_rgb555(word: u32) -> Self {
		Self {
			r: ((word & 0xFF) >> 3) as u8,
			g: (((word >> 8) & 0xFF) >> 3) as u8,
			b: (((word >> 16) & 0xFF) >> 3) as u8
		}
	}

	fn truncate_to_15bit(&self) -> u16 {
		let r: u16 = (self.r >> 3).into();
        let g: u16 = (self.g >> 3).into();
        let b: u16 = (self.b >> 3).into();

        r | (g << 5) | (b << 10)
	}

	fn from_rgb555(word: u16) -> Self {
		Self {
			r: ((word & 0x1F)) as u8,
			g: (((word >> 5) & 0x1F)) as u8,
			b: (((word >> 10) & 0x1F)) as u8,
		}
	}

	fn from_packet(word: u32) -> Self {
		Self {
			r: ((word >> 0) & 0xFF) as u8,
			g: ((word >> 8) & 0xFF) as u8,
			b: ((word >> 16) & 0xFF) as u8,
		}
	}
}

#[derive(Debug, Clone, Copy, Default)]
struct Vertex {
	x: i32,
	y: i32,
	tex_x: i32,
	tex_y: i32,
	colour: Colour,
}

impl Vertex {
	fn new(x: i32, y: i32) -> Self {
		Self {
			x: x,
			y: y,
			..Default::default()
		}
	}
	fn from_packet(packet: u32) -> Self {
		Self {
			x: ((packet as i32) << 21) >> 21,
			y: (((packet >> 16) as i32) << 21) >> 21,
			..Default::default()
		}
	}
}

pub struct Gpu {
	pub vram: Box<[u16]>,

	gp0_state: GP0State,
	gp0_params: [u32; 16],

	gp1_state: GP1State,
	gp1_params: [u32; 16],

	reg_gpuread: u32,

	// misc draw settings
	draw_area_top_left: Vertex,
	draw_area_bottom_right: Vertex,
	force_mask_bit: bool,
	check_mask_bit: bool,

	tex_page: TexturePage,
}

impl Gpu {
	pub fn new() -> Self {
		Self {
			vram: vec![0; 512 * 1024].into_boxed_slice().try_into().unwrap(),

			gp0_state: GP0State::WaitingForNextCmd,
			gp0_params: [0; 16],

			gp1_state: GP1State::WaitingForNextCmd,
			gp1_params: [0; 16],

			reg_gpuread: 0,

			draw_area_top_left: Vertex::new(0, 0),
			draw_area_bottom_right: Vertex::new(0, 0),
			force_mask_bit: false,
			check_mask_bit: false,

			tex_page: TexturePage::default(),
		}
	}

	pub fn read32(&mut self, addr: u32) -> u32 {
		match addr {
			0x1F801810 => {
				if let GP0State::SendData(info) = self.gp0_state {
					self.reg_gpuread = self.process_vram_cpu_dma(info);
				}

				self.reg_gpuread
			},
			0x1F801814 => self.gpustat(),
			_ => unimplemented!("0x{addr:X}"),
		}
	}

	pub fn write32(&mut self, addr: u32, write: u32) {
		match addr {
			0x1F801810 => self.gp0_cmd(write),
			0x1F801814 => self.gp1_cmd(write),
			_ => unimplemented!("0x{addr:X} 0x{write:X}"),
		}
	}

	fn gpustat(&mut self) -> u32 {
		0x1C000000
	}

	pub fn gp0_cmd(&mut self, word: u32) {

		trace!("GP0: 0x{word:X} state: {:?}", self.gp0_state);

		self.gp0_state = match self.gp0_state {

			GP0State::WaitingForNextCmd => match word >> 29 {
				// 0/7 cmds need to be decoded with the highest 8 bits
				0 => match word >> 24 {
					0 | 0x3..=0x1E => {
						trace!("NOP / unused cmd: 0x{word:X} GP0(${:X})", word >> 29);

						GP0State::WaitingForNextCmd
					},
					0x01 => {
						// not emulating the texture cache, this does nothing
						trace!("clear texture cache");

						GP0State::WaitingForNextCmd
					},
					0x02 => {
						GP0State::WaitingForParams { command: DrawCommand::QuickFill(word), index: 0, words_left: 2 }
					},
					0x1F => {
						unimplemented!("GP0 IRQ")
					}

					_ => todo!("Misc cmd 0x{word:X}")
				}

				1 => {
					let gouraud_shading = (word >> 28) & 1 != 0;
					let vertices = if (word >> 27) & 1 != 0 { 4 } else { 3 };
					let textured = (word >> 26) & 1 != 0;

					let params = PolygonCmdParams {
						shaded: gouraud_shading,
						vertices: vertices,
						textured: textured,
						semi_transparent: (word >> 25) & 1 != 0,
						raw_texture: (word >> 24) & 1 != 0,
						colour: Colour::from_packet(word),
						clut: Vertex::default(),
					};

					let words_left = vertices * (1 + u8::from(textured))
						+ (vertices - 1) * u8::from(gouraud_shading);

					trace!("start polygon words left {words_left} verts: {vertices} textured: {textured} shaded: {gouraud_shading} colour: {:?}", params.colour);

					GP0State::WaitingForParams { command: DrawCommand::DrawPolygon(params), index: 0, words_left: words_left }
				},
				2 => todo!("draw line"),
				3 => {
					let rect_size = RectSize::from_bits((word >> 27) & 3);
					let textured = (word >> 26) & 1 != 0;

					let words_left = 1 + u8::from(textured) + u8::from(rect_size == RectSize::Variable);

					let params = RectCmdParams {
						size_type: rect_size,
						textured: textured,
						semi_transparent: (word >> 25) & 1 != 0,
						raw_texture: (word >> 24) & 1 != 0,
						colour: Colour::from_packet(word),
						position: Vertex::default(),
						clut: Vertex::default(),
						size: Vertex::default(),
					};

					GP0State::WaitingForParams { command: DrawCommand::DrawRect(params), index: 0, words_left: words_left }
				}
				//3 => GP0State::WaitingForParams { command: DrawCommand::DrawRect(word), index: 0, words_left: 1 },
				4 => GP0State::WaitingForParams { command: DrawCommand::VramVramDma, index: 0, words_left: 3 },
				5 => GP0State::WaitingForParams { command: DrawCommand::CpuVramDma, index: 0, words_left: 2 },
				6 => GP0State::WaitingForParams { command: DrawCommand::VramCpuDma, index: 0, words_left: 2 },

				7 => match word >> 24 {
					0xE1 => { 
						trace!("set draw mode");

						self.tex_page.x_base = 64 * (word & 0xF);
						self.tex_page.y_base = 256 * ((word >> 4) & 1);
						self.tex_page.bit_depth = TexBitDepth::from_bits((word >> 7) & 3);
						self.tex_page.dithering = (word >> 9) & 1 != 0;
						self.tex_page.flip_x = (word >> 12) & 1 != 0;
						self.tex_page.flip_y = (word >> 13) & 1 != 0;

						GP0State::WaitingForNextCmd
				 	},
					0xE2 => {
						//trace!("Texture window:\nMaskX: {} MaskY: {}\nOffsetX: {} OffsetY: {}", word & 0xF, (word >> 5) & 0x1F, (word >> 10) & 0x1F, (word >> 15) & 0x1F);

						GP0State::WaitingForNextCmd
					},
					//set drawing area top left
					0xE3 => {
						self.draw_area_top_left = Vertex::new(
							(word & 0x3FF) as i32,
							((word >> 10) & 0x1FF) as i32
						);

						trace!("draw area top left: {:?}", self.draw_area_top_left);

						GP0State::WaitingForNextCmd
					},
					// set drawing area bottom right
					0xE4 => {
						self.draw_area_bottom_right = Vertex::new(
							(word & 0x3FF) as i32,
							((word >> 10) & 0x1FF) as i32
						);

						trace!("draw area bottom right: {:?} word: 0x{word:X}", self.draw_area_bottom_right);

						GP0State::WaitingForNextCmd
					},
					0xE5 => { trace!("set drawing offset"); GP0State::WaitingForNextCmd },
					// mask bit settings
					0xE6 => {
						self.force_mask_bit = word & 1 != 0;
						self.check_mask_bit = (word >> 1) & 1 != 0;

						GP0State::WaitingForNextCmd
					}
					_ => { debug!("Enviroment cmd 0x{word:X} GP0(${:X})", word >> 24); GP0State::WaitingForNextCmd }
				}

				_ => unreachable!()
			},

			GP0State::WaitingForParams { command, index, words_left } => {

				self.gp0_params[index as usize] = word;
				trace!("(write 0x{word:X}) words left {}", words_left - 1);

				if words_left == 1 {
					self.exec_cmd(command)
				} else {
					GP0State::WaitingForParams { command: command, index: index + 1, words_left: words_left - 1 }
				}
			}

			GP0State::RecvData(vram_dma_info) => self.process_cpu_vram_dma(word, vram_dma_info),
			GP0State::SendData(_) => panic!("write 0x{word:X} to GP0 during VRAM to CPU DMA"),
		}
	}

	fn gp1_cmd(&mut self, word: u32) {
		self.gp1_state = match self.gp1_state {
			GP1State::WaitingForNextCmd => match word >> 29 {
				0 => {
					//println!("reset gpu");

					GP1State::WaitingForNextCmd
				},
				_ => unimplemented!("unimplemented GP1 command: 0x{:X}", word >> 29),
			}
		}
	}

	fn exec_cmd(&mut self, cmd: DrawCommand) -> GP0State {
		match cmd {
			DrawCommand::CpuVramDma => {
				let info = self.init_dma();

				GP0State::RecvData(info)
			},
			DrawCommand::VramCpuDma => {
				let info = self.init_dma();

				GP0State::SendData(info)
			},
			DrawCommand::VramVramDma => {
				self.vram_copy();

				GP0State::WaitingForNextCmd
			}
			DrawCommand::DrawRect(cmd) => {
				trace!("draw rect");
				self.draw_rect(cmd);

				GP0State::WaitingForNextCmd
			},
			DrawCommand::DrawPolygon(cmd) => {
				trace!("draw polygon");
				self.draw_polygon(cmd);

				GP0State::WaitingForNextCmd
			}
			DrawCommand::QuickFill(cmd) => {
				self.quick_fill(cmd);

				GP0State::WaitingForNextCmd
			}
		}
	}

	fn init_dma(&mut self) -> VramDmaInfo {

		let dest_x = (self.gp0_params[0] & 0x3FF) as u16;
		let dest_y = ((self.gp0_params[0] >> 16) & 0x1FF) as u16;

		let mut width = (self.gp0_params[1] & 0x3FF) as u16;
		if width == 0 {
			width = 1024;
		}

		let mut height = ((self.gp0_params[1] >> 16) & 0x1FF) as u16;
		if height == 0 {
			height = 512;
		}

		let extra_halfword = (width * height) % 2 != 0;

		VramDmaInfo {
			dest_x,
			dest_y,
			width,
			height,
			current_row: 0,
			current_col: 0,
			halfwords_left: (width * height) + u16::from(extra_halfword),
		}
	}

	fn process_cpu_vram_dma(&mut self, word: u32, mut info: VramDmaInfo) -> GP0State {
		for i in 0..2 {

			let halfword = (word >> (16 * i)) as u16;

			// wrap from 511 to 0
			let vram_row = ((info.dest_y + info.current_row) & 0x1FF) as u32;
			// wrap from 1023 to 0
			let vram_col = ((info.dest_x + info.current_col) & 0x3FF) as u32;

			self.draw_pixel_15bit(halfword, vram_col, vram_row);

			info.current_col += 1;
			info.halfwords_left -= 1;

			if info.current_col == info.width {
				info.current_col = 0;
				info.current_row += 1;
			}
		}

		if info.halfwords_left == 0 {
			return GP0State::WaitingForNextCmd;
		}

		GP0State::RecvData(info)
	}

	fn process_vram_cpu_dma(&mut self, mut info: VramDmaInfo) -> u32 {

		let mut result: [u16; 2] = [0; 2];

		for i in 0..2 {

			// wrap from 511 to 0
			let vram_row = ((info.dest_y + info.current_row) & 0x1FF) as u32;
			// wrap from 1023 to 0
			let vram_col = ((info.dest_x + info.current_col) & 0x3FF) as u32;

			let vram_addr = coord_to_vram_index(vram_col, vram_row) as usize;
			result[i] = self.vram[vram_addr];

			info.current_col += 1;
			info.halfwords_left -= 1;

			if info.current_col == info.width {
				info.current_col = 0;
				info.current_row += 1;
			}
			
		}

		if info.halfwords_left == 0 {
			self.gp0_state = GP0State::WaitingForNextCmd;
		} else {
			self.gp0_state = GP0State::SendData(info);
		}

		(u32::from(result[1]) << 16) | u32::from(result[0])
	}

	fn vram_copy(&mut self) {
		let src_x = (self.gp0_params[0] & 0x3FF) as u16;
		let src_y = ((self.gp0_params[0] >> 16) & 0x1FF) as u16;

		let dest_x = (self.gp0_params[1] & 0x3FF) as u16;
		let dest_y = ((self.gp0_params[1] >> 16) & 0x1FF) as u16;

		let mut width = (self.gp0_params[2] & 0x3FF) as u16;
		if width == 0 {
			width = 1024;
		}

		let mut height = ((self.gp0_params[2] >> 16) & 0x1FF) as u16;
		if height == 0 {
			height = 512;
		}

		for y_offset in 0..height {
			for x_offset in 0..width {
				let src_addr = coord_to_vram_index((src_x + x_offset) as u32, (src_y + y_offset) as u32) as usize;

				let src = self.vram[src_addr];
				self.draw_pixel_15bit(src, (dest_x + x_offset) as u32, (dest_y + y_offset) as u32);
			}
		}
	}

	fn draw_rect(&mut self, mut cmd: RectCmdParams) {
		cmd.position = Vertex::from_packet(self.gp0_params[0]);

		let (clut_packet, size_packet) = match (cmd.textured, cmd.size_type == RectSize::Variable) {
			(true, true) => (self.gp0_params[1], self.gp0_params[2]),
			(true, false) => (self.gp0_params[1], 0),
			(false, true) => (0, self.gp0_params[1]),
			(false, false) => (0, 0),
		};

		cmd.clut = Vertex::new((((clut_packet >> 16) & 0x3F) as u16 as i32) * 16, ((clut_packet >> 22) & 0x1FF) as u16 as i32);
		cmd.position.tex_x = (clut_packet & 0xFF) as i32;
		cmd.position.tex_y = ((clut_packet >> 8) & 0xFF) as i32;

		cmd.size = match cmd.size_type {
			RectSize::Variable => Vertex::from_packet(size_packet),
			RectSize::Pixel => Vertex::new(1, 1),
			RectSize::Sprite8x8 => Vertex::new(8, 8),
			RectSize::Sprite16x16 => Vertex::new(16, 16),
		};
		
		let mut min_x = cmd.position.x;
		let mut max_x = cmd.position.x + cmd.size.x - 1;
		let mut min_y = cmd.position.y;
		let mut max_y = cmd.position.y + cmd.size.y - 1;

		// constrain rect to drawing area
		min_x = cmp::max(min_x, self.draw_area_top_left.x);
		max_x = cmp::min(max_x, self.draw_area_bottom_right.x);
		min_y = cmp::max(min_y, self.draw_area_top_left.y);
		max_y = cmp::min(max_y, self.draw_area_bottom_right.y);

		if min_x > max_x || min_y > max_y {
			return;
		}

		for y in min_y..=max_y {
			for x in min_x..=max_x {

				let draw_colour = if cmd.textured {
					self.sample_texture(Vertex::new(cmd.position.tex_x + (x - min_x), cmd.position.tex_y + (y - min_y)), cmd.clut)
				} else {
					u16::from(cmd.colour.r) | (u16::from(cmd.colour.g) << 5) | (u16::from(cmd.colour.b) << 10)
				};

				self.draw_pixel_15bit(draw_colour, x as u32, y as u32);
			}
		}
	}

	fn quick_fill(&mut self, cmd: u32) {
		let r = ((cmd & 0xFF) >> 3) as u16;
		let g = (((cmd >> 8) & 0xFF) >> 3) as u16;
		let b = (((cmd >> 16) & 0xFF) >> 3) as u16;

		let colour = r | (g << 5) | (b << 10);

		let x = (self.gp0_params[0] & 0xFFFF) & 0x3F0;
		let y = (self.gp0_params[0] >> 16) & 0x1FF;
		
		// https://psx-spx.consoledev.net/graphicsprocessingunitgpu/#masking-and-rounding-for-fill-command-parameters
		let width = (((self.gp0_params[1] & 0xFFFF) & 0x3FF) + 0x0F) & !(0x0F);
		let height = (self.gp0_params[1] >> 16) & 0x1FF;

		//debug!("quick fill at ({x}, {y}) of size ({width}, {height}) cmd: ${cmd:X} r:{r} g:{g} b{b} colour: ${colour:X}");

		for y_offset in 0..height {
			for x_offset in 0..width {
				let index = coord_to_vram_index((x + x_offset) & 0x3FF, (y + y_offset) & 0x1FF);

				// quick fill doesn't check mask bit
				self.vram[index as usize] = colour;
			}
		}

	}

	fn draw_polygon(&mut self, mut cmd: PolygonCmdParams) {

		let mut v = [Vertex::default(); 4];

		let shaded = usize::from(cmd.shaded);
		let textured = usize::from(cmd.textured);

		let vertex_offset = 1 + (textured | shaded) + (textured & shaded);
		let colour_offset = shaded * (2 + textured);
		let texcoord_offset = textured * (2 + shaded);

		let clut_packet = self.gp0_params[1];
		cmd.clut = Vertex::new((((clut_packet >> 16) & 0x3F) as u16 as i32) * 16, ((clut_packet >> 22) & 0x1FF) as u16 as i32);

		let tex_page = self.gp0_params[textured * (3 + shaded)] >> 16;

		self.tex_page = TexturePage {
			x_base: 64 * (tex_page & 0xF),
			y_base: 256 * ((tex_page >> 4) & 1),
			bit_depth: TexBitDepth::from_bits((tex_page >> 7) & 3),
			..self.tex_page
		};

		v[0] = Vertex::from_packet(self.gp0_params[0]);
		v[0].colour = cmd.colour;
		v[0].tex_x = (self.gp0_params[1 + 0 * texcoord_offset] & 0xFF) as i32;
		v[0].tex_y = ((self.gp0_params[1 + 0 * texcoord_offset] >> 8) & 0xFF) as i32;

		v[1] = Vertex::from_packet(self.gp0_params[1 * vertex_offset]);
		v[1].colour = Colour::from_packet(self.gp0_params[1 + 0 * colour_offset]);
		v[1].tex_x = (self.gp0_params[1 + 1 * texcoord_offset] & 0xFF) as i32;
		v[1].tex_y = ((self.gp0_params[1 + 1 * texcoord_offset] >> 8) & 0xFF) as i32;

		v[2] = Vertex::from_packet(self.gp0_params[2 * vertex_offset]);
		v[2].colour = Colour::from_packet(self.gp0_params[1 + 1 * colour_offset]);
		v[2].tex_x = (self.gp0_params[1 + 2 * texcoord_offset] & 0xFF) as i32;
		v[2].tex_y = ((self.gp0_params[1 + 2 * texcoord_offset] >> 8) & 0xFF) as i32;

		ensure_vertex_order(&mut v);
		self.draw_triangle(v[0], v[1], v[2], cmd);

		if cmd.vertices == 4 {
			v[3] = Vertex::from_packet(self.gp0_params[3 * vertex_offset]);
			v[3].colour = Colour::from_packet(self.gp0_params[1 + 2 * colour_offset]);
			v[3].tex_x = (self.gp0_params[1 + 3 * texcoord_offset] & 0xFF) as i32;
			v[3].tex_y = ((self.gp0_params[1 + 3 * texcoord_offset] >> 8) & 0xFF) as i32;

			let mut v2 = [v[1], v[2], v[3]];
			ensure_vertex_order(&mut v2);

			self.draw_triangle(v2[0], v2[1], v2[2], cmd);
		}

	}

	fn draw_triangle(&mut self, v0: Vertex, v1: Vertex, v2: Vertex, cmd: PolygonCmdParams) {
		// compute polygon bounding box
		let mut min_x = cmp::min(v0.x, cmp::min(v1.x, v2.x));
		let mut max_x = cmp::max(v0.x, cmp::max(v1.x, v2.x));
		let mut min_y = cmp::min(v0.y, cmp::min(v1.y, v2.y));
		let mut max_y = cmp::max(v0.y, cmp::max(v1.y, v2.y));

		// constrain bounding box to drawing area
		min_x = cmp::max(min_x, self.draw_area_top_left.x);
		max_x = cmp::min(max_x, self.draw_area_bottom_right.x);
		min_y = cmp::max(min_y, self.draw_area_top_left.y);
		max_y = cmp::min(max_y, self.draw_area_bottom_right.y);

		if min_x > max_x || min_y > max_y {
			return;
		}

		for y in min_y..=max_y {
			for x in min_x..=max_x {

				let p = Vertex::new(x, y);

				if is_inside_triangle(p, v0, v1, v2) {
					let shaded_colour = if cmd.shaded {
						let coords = compute_barycentric_coords(p, v0, v1, v2);
						let mut colour = interpolate_colour(coords, [v0.colour, v1.colour, v2.colour]);

						if self.tex_page.dithering {
							colour = apply_dithering(colour, p);
						}

						colour
					} else {
						cmd.colour
					};

					let textured_colour = if cmd.textured {
						
						let coords = compute_barycentric_coords(p, v0, v1, v2);
						let interpolated_coords = interpolate_uv(coords, [v0, v1, v2]);

						let colour = self.sample_texture(interpolated_coords, cmd.clut);

						// black is transparent in textures
						if colour == 0 {
							continue;
						}

						colour

					} else {
						shaded_colour.truncate_to_15bit()
					};

					self.draw_pixel_15bit(textured_colour, x as u32, y as u32);
				}
			}
		}
	}

	fn sample_texture(&mut self, tex_coords: Vertex, clut: Vertex) -> u16 {

		match self.tex_page.bit_depth {
			TexBitDepth::FourBit => {
				let tex_x = self.tex_page.x_base + ((tex_coords.x as u32) / 4);
				let tex_y = self.tex_page.y_base + tex_coords.y as u32;

				let texel = self.vram[coord_to_vram_index(tex_x, tex_y) as usize];
				let clut_index = ((texel >> (tex_coords.x % 4) * 4) & 0xF) as u32;

				self.vram[coord_to_vram_index(clut.x as u32 + clut_index, clut.y as u32) as usize]
			},
			TexBitDepth::EightBit => {
				let tex_x = self.tex_page.x_base + ((tex_coords.x as u32) / 2);
				let tex_y = self.tex_page.y_base + tex_coords.y as u32;

				let texel = self.vram[coord_to_vram_index(tex_x, tex_y) as usize];
				let clut_index = ((texel >> (tex_coords.x % 2) * 8) & 0xFF) as u32;

				self.vram[coord_to_vram_index(clut.x as u32 + clut_index, clut.y as u32) as usize]
			}
			TexBitDepth::FiveteenBit => {
				let tex_x = self.tex_page.x_base + (tex_coords.x as u32);
				let tex_y = self.tex_page.y_base + (tex_coords.y as u32);
				let texel = self.vram[coord_to_vram_index(tex_x, tex_y) as usize];

				texel
			},
		}

	}

	fn draw_pixel_15bit(&mut self, colour: u16, x: u32, y: u32) {
		let mask = u16::from(self.force_mask_bit) << 15;

		let old_pixel = self.vram[coord_to_vram_index(x, y) as usize];

		if self.check_mask_bit && old_pixel & 0x8000 != 0 {
			return;
		}

		self.vram[coord_to_vram_index(x, y) as usize] = colour | mask;
	}
	
}

fn coord_to_vram_index(x: u32, y: u32) -> u32 {
	(1024 * y).wrapping_add(x)
}

fn cross_product_z(v0: Vertex, v1: Vertex, v2: Vertex) -> i32 {
	let result = (v1.x - v0.x) * (v2.y - v0.y) - (v1.y - v0.y) * (v2.x - v0.x);
	
	result
}

fn ensure_vertex_order(v: &mut [Vertex]) -> bool {
	let cross_product_z = cross_product_z(v[0], v[1], v[2]);

	if cross_product_z < 0 {
		v.swap(0, 1);
		return true;
	}

	false
}

fn is_inside_triangle(p: Vertex, v0: Vertex, v1: Vertex, v2: Vertex) -> bool {
	for (va, vb) in [(v0, v1), (v1, v2), (v2, v0)] {

		let cpz = cross_product_z(va, vb, p);

		if cpz < 0 {
			return false;
		}

		// P lies on an edge, only rasterize if its on a top/left edge
		if cpz == 0 {

			// right edge
			if vb.y > va.y {
				return false;
			}

			// bottom edge
			if va.y == vb.y && vb.x < va.x {
				return false;
			}

		}
	}

	true
}

fn compute_barycentric_coords(p: Vertex, v0: Vertex, v1: Vertex, v2: Vertex) -> [f64; 3] {
	let denominator = cross_product_z(v0, v1, v2);
    if denominator == 0 {
        return [1.0 / 3.0, 1.0 / 3.0, 1.0 / 3.0];
    }

    let denominator: f64 = denominator.into();

    let lambda0 = f64::from(cross_product_z(v1, v2, p)) / denominator;
    let lambda1 = f64::from(cross_product_z(v2, v0, p)) / denominator;

    let lambda2 = 1.0 - lambda0 - lambda1;

    [lambda0, lambda1, lambda2]
}

fn interpolate_colour(lambda: [f64; 3], point_colours: [Colour; 3]) -> Colour {
	let colours_r = point_colours.map(|colour| f64::from(colour.r));
	let colours_g = point_colours.map(|colour| f64::from(colour.g));
	let colours_b = point_colours.map(|colour| f64::from(colour.b));

	let r = (lambda[0] * colours_r[0] + lambda[1] * colours_r[1] + lambda[2] * colours_r[2]).round() as u8;
	let g = (lambda[0] * colours_g[0] + lambda[1] * colours_g[1] + lambda[2] * colours_g[2]).round() as u8;
	let b = (lambda[0] * colours_b[0] + lambda[1] * colours_b[1] + lambda[2] * colours_b[2]).round() as u8;

	Colour::from_rgb888(r, g, b)
}

fn interpolate_uv(lambda: [f64; 3], tex_coords: [Vertex; 3]) -> Vertex {
	let coords_x = tex_coords.map(|coord| coord.tex_x as f64);
	let coords_y = tex_coords.map(|coord| coord.tex_y as f64);

	// not rounding these fixes minor texcoord interpolation errors
	let u = (lambda[0] * coords_x[0] + lambda[1] * coords_x[1] + lambda[2] * coords_x[2]) as i32;
	let v = (lambda[0] * coords_y[0] + lambda[1] * coords_y[1] + lambda[2] * coords_y[2]) as i32;

	Vertex::new(u, v)
}

fn apply_dithering(colour: Colour, p: Vertex) -> Colour {
    let offset = DITHERING_TABLE[(p.y & 3) as usize][(p.x & 3) as usize];

    Colour {
        r: colour.r.saturating_add_signed(offset),
        g: colour.g.saturating_add_signed(offset),
        b: colour.b.saturating_add_signed(offset),
    }
}
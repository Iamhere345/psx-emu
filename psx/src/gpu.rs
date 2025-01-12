use std::cmp;
use log::*;

#[derive(Debug, Clone, Copy)]
enum DrawCommand {
	CpuVramDma,
	VramCpuDma,
	VramVramDma,
	DrawRect(u32),
	DrawPolygon(PolygonCmdParams),
	QuickFill(u32)
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
	colour: Colour,
}

#[derive(Debug, Clone, Copy)]
struct VramDmaInfo {
	dest_x: u16,
	dest_y: u16,

	width: u16,
	height: u16,

	current_row: u16,
	current_col: u16,
}

#[derive(Debug, Clone, Copy, Default)]
struct Colour {
	r: u8,
	g: u8,
	b: u8,
}

impl Colour {
	fn from_rgb(r: u8, g: u8, b: u8) -> Self {
		Self { r, g, b }
	}

	fn rgb888_to_rgb555(word: u32) -> Self {
		Self {
			r: ((word & 0xFF) >> 3) as u8,
			g: (((word >> 8) & 0xFF) >> 3) as u8,
			b: (((word >> 16) & 0xFF) >> 3) as u8
		}
	}
}

#[derive(Debug, Clone, Copy, Default)]
struct Vertex {
	x: i32,
	y: i32,
	colour: Colour,
}

impl Vertex {
	fn new(x: i32, y: i32) -> Self {
		Self {
			x: x,
			y: y,
			colour: Colour::default()
		}
	}
	fn from_packet(packet: u32) -> Self {
		Self {
			x: ((packet as i32) << 21) >> 21,
			y: (((packet >> 16) as i32) << 21) >> 21,
			colour: Colour::default()
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

	draw_area_top_left: Vertex,
	draw_area_bottom_right: Vertex,
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
						debug!("NOP / unused cmd: 0x{word:X} GP0(${:X})", word >> 29);

						GP0State::WaitingForNextCmd
					},
					0x01 => {
						// not emulating the texture cache, this does nothing
						debug!("clear texture cache");

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
						colour: Colour::from_rgb(
							((word & 0xFF) >> 3) as u8,
							(((word >> 8) & 0xFF) >> 3) as u8,
							(((word >> 16) & 0xFF) >> 3) as u8
						)
					};

					let words_left = vertices * (1 + u8::from(textured))
						+ (vertices - 1) * u8::from(gouraud_shading);

					trace!("start polygon words left {words_left} verts: {vertices} textured: {textured} shaded: {gouraud_shading} colour: {:?}", params.colour);

					GP0State::WaitingForParams { command: DrawCommand::DrawPolygon(params), index: 0, words_left: words_left }
				},
				2 => todo!("draw line"),
				3 => GP0State::WaitingForParams { command: DrawCommand::DrawRect(word), index: 0, words_left: 1 },
				4 => GP0State::WaitingForParams { command: DrawCommand::VramVramDma, index: 0, words_left: 3 },
				5 => GP0State::WaitingForParams { command: DrawCommand::CpuVramDma, index: 0, words_left: 2 },
				6 => GP0State::WaitingForParams { command: DrawCommand::VramCpuDma, index: 0, words_left: 2 },

				7 => match word >> 24 {
					0xE1 => { debug!("set draw mode"); GP0State::WaitingForNextCmd },
					//set drawing area top left
					0xE3 => {
						self.draw_area_top_left = Vertex::new(
							(word & 0x3FF) as i32,
							((word >> 10) & 0x1FF) as i32
						);

						debug!("draw area top left: {:?}", self.draw_area_top_left);

						GP0State::WaitingForNextCmd
					},
					// set drawing area bottom right
					0xE4 => {
						self.draw_area_bottom_right = Vertex::new(
							(word & 0x3FF) as i32,
							((word >> 10) & 0x1FF) as i32
						);

						debug!("draw area bottom right: {:?} word: 0x{word:X}", self.draw_area_bottom_right);

						GP0State::WaitingForNextCmd
					},
					0xE5 => { debug!("set drawing offset"); GP0State::WaitingForNextCmd },
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
			GP0State::SendData(info) => {panic!("write 0x{word:X} to GP0 during VRAM to CPU DMA"); GP0State::WaitingForNextCmd},
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
				self.draw_pixel(cmd, self.gp0_params[0]);

				GP0State::WaitingForNextCmd
			},
			DrawCommand::DrawPolygon(cmd) => {
				debug!("draw polygon");
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

		VramDmaInfo {
			dest_x,
			dest_y,
			width,
			height,
			current_row: 0,
			current_col: 0,
		}
	}

	fn process_cpu_vram_dma(&mut self, word: u32, mut info: VramDmaInfo) -> GP0State {
		for i in 0..2 {

			let halfword = (word >> (16 * i)) as u16;

			// wrap from 511 to 0
			let vram_row = ((info.dest_y + info.current_row) & 0x1FF) as u32;
			// wrap from 1023 to 0
			let vram_col = ((info.dest_x + info.current_col) & 0x3FF) as u32;

			let vram_addr = coord_to_vram_index(vram_col, vram_row) as usize;
			self.vram[vram_addr] = halfword;

			info.current_col += 1;

			if info.current_col == info.width {
				info.current_col = 0;
				info.current_row += 1;

				if info.current_row == info.height {
					return GP0State::WaitingForNextCmd;
				}
			}

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

			if info.current_col == info.width {
				info.current_col = 0;
				info.current_row += 1;

			}
			
		}

		if info.current_row == info.height {
			self.gp0_state = GP0State::WaitingForNextCmd;
		} else {
			self.gp0_state = GP0State::SendData(info);
		}

		(u32::from(result[0]) << 16) | u32::from(result[1])
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
				let dest_addr = coord_to_vram_index((dest_x + x_offset) as u32, (dest_y + y_offset) as u32) as  usize;

				let src = self.vram[src_addr];
				self.vram[dest_addr] = src;
			}
		}
	}

	fn draw_pixel(&mut self, cmd: u32, param: u32) {
		let r = (cmd & 0xFF) >> 3;
		let g = ((cmd >> 8) & 0xFF) >> 3;
		let b = ((cmd >> 16) & 0xFF) >> 3;

		let pixel = (r | (g << 5) | (b << 10)) as u16;

		let x = param & 0x3FF;
		let y = (param >> 16) & 0x1FF;

		if pixel != 0 {
			//println!("draw pixel ({r}, {g}, {b}) at ({x}, {y})");
		}

		let vram_addr = coord_to_vram_index(x, y) as usize;
		self.vram[vram_addr] = pixel;
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

		//println!("quick fill at ({x}, {y}) of size ({width}, {height}) cmd: ${cmd:X} r:{r} g:{g} b{b} colour: ${colour:X}");

		for y_offset in 0..height {
			for x_offset in 0..width {
				let index = coord_to_vram_index((x + x_offset) & 0x3FF, (y + y_offset) & 0x1FF);

				self.vram[index as usize] = colour;
			}
		}

	}

	fn draw_polygon(&mut self, cmd: PolygonCmdParams) {

		let mut v = [Vertex::default(); 4];

		let shaded = usize::from(cmd.shaded);
		let textured = usize::from(cmd.textured);

		let vertex_offset = 1 + (textured | shaded) + (textured & shaded);
		let colour_offset = shaded * (2 + textured);

		v[0] = Vertex::from_packet(self.gp0_params[0]);
		v[0].colour = cmd.colour;

		v[1] = Vertex::from_packet(self.gp0_params[1 * vertex_offset]);
		v[1].colour = Colour::rgb888_to_rgb555(self.gp0_params[1 + 0 * colour_offset]);

		v[2] = Vertex::from_packet(self.gp0_params[2 * vertex_offset]);
		v[2].colour = Colour::rgb888_to_rgb555(self.gp0_params[1 + 1 * colour_offset]);

		ensure_vertex_order(&mut v);
		self.draw_triangle(v[0], v[1], v[2], cmd);

		if cmd.vertices == 4 {
			v[3] = Vertex::from_packet(self.gp0_params[3 * vertex_offset]);
			v[3].colour = Colour::rgb888_to_rgb555(self.gp0_params[1 + 2 * colour_offset]);

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

				if is_inside_triangle(Vertex::new(x, y), v0, v1, v2) {
					let point_colour = if cmd.shaded {
						let coords = compute_barycentric_coords(Vertex::new(x, y), v0, v1, v2);
						interpolate_colour(coords, [v0.colour, v1.colour, v2.colour])
					} else {
						cmd.colour
					};

					let draw_colour = u16::from(point_colour.r) | (u16::from(point_colour.g) << 5) | (u16::from(point_colour.b) << 10);
					self.vram[coord_to_vram_index(x as u32, y as u32) as usize] = draw_colour;
				}
			}
		}
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

	Colour::from_rgb(r, g, b)
}
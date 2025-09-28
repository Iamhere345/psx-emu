#![allow(dead_code)]
use std::cmp;
use log::*;

const DITHERING_TABLE: &[[i8; 4]; 4] = &[[-4, 0, -3, 1], [2, -2, 3, -1], [-3, 1, -4, 0], [3, -1, 2, -2]];

#[derive(Debug, Clone, Copy)]
enum DrawCommand {
	CpuVramDma,
	VramCpuDma,
	VramVramDma,
	DrawRect(RectCmdParams),
	DrawPolygon(PolygonCmdParams),
	DrawLine(LineCmdParams),
	QuickFill(u32)
}

#[derive(Default, Clone, Copy)]
enum TexBitDepth {
	#[default]
	FourBit = 0,
	EightBit = 1,
	FiveteenBit = 2,
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

			_ => unreachable!(),
		}
	}
}

#[derive(Default, Clone, Copy, Debug)]
enum SemiTransparency {
	#[default]
	HalfBPlusHalfF = 0,
	BPlusF = 1,
	BMinusF = 2,
	BPlusFOver4 = 3,
}

impl SemiTransparency {
	fn from_bits(bits: u32) -> Self {
		match bits {
			0 => Self::HalfBPlusHalfF,
			1 => Self::BPlusF,
			2 => Self::BMinusF,
			3 => Self::BPlusFOver4,

			_ => unreachable!(),
		}
	}
}

#[derive(Debug, Clone, Copy)]
enum GP0State {
	WaitingForNextCmd,
	WaitingForParams { command: DrawCommand, index: u8, words_left: u8 },
	WaitingForPolyline(LineCmdParams),
	RecvData(VramDmaInfo),
	SendData(VramDmaInfo),
}

enum GP1State {
	WaitingForNextCmd,
}

#[derive(Debug, Clone, Copy)]
enum HorizontalRes {
	H256 = 0,
	H320 = 1,
	H512 = 2,
	H640 = 3,
}

impl HorizontalRes {
	fn from_bits(bits: u32) -> Self {
		match bits {
			0 => Self::H256,
			1 => Self::H320,
			2 => Self::H512,
			3 => Self::H640,

			_ => unreachable!()
		}
	}
}

#[derive(Debug, Clone, Copy)]
enum VerticalRes {
	V240 = 0,
	V480 = 1,
}

impl VerticalRes {
	fn from_bit(bit: bool) -> Self {
		match bit {
			true => Self::V480,
			false => Self::V240,
		}
	}
}

#[derive(Debug, Clone, Copy)]
enum VideoMode {
	Ntsc = 0,
	Pal = 1,
}

impl VideoMode {
	fn from_bit(bit: bool) -> Self {
		match bit {
			true => Self::Pal,
			false => Self::Ntsc,
		}
	}
}

#[derive(Debug, Clone, Copy)]
enum ColourDepth {
	FiveteenBit = 0,
	TwentyFourBit = 1,
}

impl ColourDepth {
	fn from_bit(bit: bool) -> Self {
		match bit {
			true => Self::TwentyFourBit,
			false => Self::FiveteenBit,
		}
	}
}

#[derive(Debug, Clone, Copy)]
enum DmaDirection {
	Off = 0,
	Fifo = 1,
	CpuToGp0 = 2,
	GpureadToCpu = 3
}

impl DmaDirection {
	fn from_bits(bits: u32) -> Self {
		match bits {
			0 => Self::Off,
			1 => Self::Fifo,
			2 => Self::CpuToGp0,
			3 => Self::GpureadToCpu,

			_ => unreachable!(),
		}
	}
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
struct LineCmdParams {
	shaded: bool,			// 1: gouraud / 0: flat
	polyline: bool,			// 1: polyline / 0: single line
	semi_transparent: bool,
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

	halfwords_left: u16
}

// set by GP0 $E1, some fields are set by textured cmds
#[derive(Default)]
struct TexturePage {
	x_base: u32,
	y_base: u32,
	transp_type: SemiTransparency,
	bit_depth: TexBitDepth,
	dithering: bool,
	allow_drawing_to_display_area: bool,
	flip_x: bool,
	flip_y: bool,
}

#[derive(Default)]
struct TextureWindow {
	mask: Vertex,
	offset: Vertex,
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

	fn rgb555_to_rgb888(word: u16) -> Self {
		Self {
			r: ((word & 0x1F) << 3) as u8,
			g: (((word >> 5) & 0x1F) << 3) as u8,
			b: (((word >> 10) & 0x1F) << 3) as u8,
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
	fn plus_offset(&self, offset: Vertex) -> Self {
		Self {
			x: self.x + offset.x,
			y: self.y + offset.y,
			..*self
		}
	}
}

pub struct Gpu {
	pub vram: Box<[u16]>,

	gp0_state: GP0State,
	gp0_params: Vec<u32>,

	gp1_state: GP1State,
	gp1_params: [u32; 16],

	reg_gpuread: u32,

	// misc draw settings
	draw_area_top_left: Vertex,
	draw_area_bottom_right: Vertex,
	drawing_offset: Vertex,
	force_mask_bit: bool,
	check_mask_bit: bool,

	tex_page: TexturePage,
	tex_window: TextureWindow,

	display_enabled: bool,

	irq: bool,

	horizontal_res: HorizontalRes,
	force_h368: bool,
	vertical_res: VerticalRes,
	video_mode: VideoMode,
	display_colour_depth: ColourDepth,
	vertical_interlace: bool,
	// invalid on v2 gpus, not being implemented
	flip_screen: bool,

	dma_direction: DmaDirection,

	display_start: Vertex,
	horizontal_display_range: (u32, u32),
	vertical_display_range: (u32, u32),

	internal_reg: Option<u32>,
}

impl Gpu {
	pub fn new() -> Self {
		Self {
			vram: vec![0; 512 * 1024].into_boxed_slice().try_into().unwrap(),

			gp0_state: GP0State::WaitingForNextCmd,
			gp0_params: vec![0; 16],

			gp1_state: GP1State::WaitingForNextCmd,
			gp1_params: [0; 16],

			reg_gpuread: 0,

			draw_area_top_left: Vertex::new(0, 0),
			draw_area_bottom_right: Vertex::new(0, 0),
			drawing_offset: Vertex::new(0, 0),
			force_mask_bit: false,
			check_mask_bit: false,

			tex_page: TexturePage::default(),
			tex_window: TextureWindow::default(),

			display_enabled: false,

			irq: false,

			horizontal_res: HorizontalRes::H256,
			force_h368: false,
			vertical_res: VerticalRes::V240,
			video_mode: VideoMode::Ntsc,
			display_colour_depth: ColourDepth::FiveteenBit,
			vertical_interlace: false,
			flip_screen: false,

			dma_direction: DmaDirection::Off,

			display_start: Vertex::new(0, 0),
			horizontal_display_range: (0, 0),
			vertical_display_range: (0, 0),

			internal_reg: None,
		}
	}

	pub fn read32(&mut self, addr: u32) -> u32 {
		match addr {
			0x1F801810 => {
				if let Some(reg) = self.internal_reg {
					self.reg_gpuread = reg;
					self.internal_reg = None;
				} else if let GP0State::SendData(info) = self.gp0_state {
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
		let ready_to_recv_cmd = matches!(self.gp0_state, GP0State::WaitingForNextCmd);
		let ready_to_recv_vram = matches!(self.gp0_state, GP0State::WaitingForNextCmd | GP0State::SendData(..));
		let ready_to_recv_dma = matches!(
			self.gp0_state, 
			GP0State::WaitingForNextCmd
				| GP0State::RecvData(..)
				| GP0State::SendData(..)
		);

		//debug!("cmd: {ready_to_recv_cmd} vram: {ready_to_recv_vram} dma: {ready_to_recv_dma} state: {:?}", self.gp0_state);

		let dma_request = match self.dma_direction {
			DmaDirection::Off => 0,
			DmaDirection::Fifo => 1,
			DmaDirection::CpuToGp0 => ready_to_recv_dma as u32,
			DmaDirection::GpureadToCpu => ready_to_recv_vram as u32,
		};

		let _result = (self.tex_page.x_base)
			| (self.tex_page.y_base) << 4
			| (self.tex_page.transp_type as u32) << 5
			| (self.tex_page.bit_depth as u32) << 7
			| (self.tex_page.dithering as u32) << 9
			| (self.tex_page.allow_drawing_to_display_area as u32) << 10
			| (self.force_mask_bit as u32) << 11
			| (self.check_mask_bit as u32) << 12
			| (0) << 13 // TODO interlace field
			| (self.flip_screen as u32) << 14
			| (self.force_h368 as u32) << 16
			| (self.horizontal_res as u32) << 17
			| (0) << 19
			| (self.video_mode as u32) << 20
			| (self.display_colour_depth as u32) << 21
			| (self.vertical_interlace as u32) << 22
			| (self.display_enabled as u32) << 23
			| (self.irq as u32) << 24
			| (dma_request) << 25
			| (ready_to_recv_cmd as u32) << 26
			| (ready_to_recv_vram as u32) << 27 // DMA via GPUREAD
			| (ready_to_recv_dma as u32) << 28  // DMA via GP0
			| (self.dma_direction as u32) << 29
			| (0) << 31
			| 0x1C000000; // TODO Drawing even/odd lines in interlace mode

		//debug!("gpustat: 0x{result:X}");

		// TODO using the non-stubbed value seems to break more things
		0x1C000000
		//_result
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
						trace!("GP0 IRQ");
						self.irq = true;
						
						GP0State::WaitingForNextCmd
					}

					_ => todo!("Misc cmd 0x{word:X}")
				}

				// draw polygon
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
				// draw line
				2 => {
					let params = LineCmdParams {
						shaded: (word >> 28) & 1 != 0,
						polyline: (word >> 27) & 1 != 0,
						semi_transparent: (word >> 25) & 1 != 0,
						colour: Colour::from_packet(word),
					};

					let words_left = 2 + u8::from(params.shaded);

					trace!("start line words left {words_left} shaded: {} polyline: {}", params.shaded, params.polyline);

					if params.polyline {
						self.gp0_params.clear();
						GP0State::WaitingForPolyline(params)
					} else {
						GP0State::WaitingForParams { command: DrawCommand::DrawLine(params), index: 0, words_left: words_left }
					}
				},
				// draw rect
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
						self.tex_page.transp_type = SemiTransparency::from_bits((word >> 5) & 3);
						self.tex_page.bit_depth = TexBitDepth::from_bits((word >> 7) & 3);
						self.tex_page.dithering = (word >> 9) & 1 != 0;
						self.tex_page.allow_drawing_to_display_area = (word >> 10) & 1 != 0;
						self.tex_page.flip_x = (word >> 12) & 1 != 0;
						self.tex_page.flip_y = (word >> 13) & 1 != 0;

						GP0State::WaitingForNextCmd
				 	},
					0xE2 => {
						trace!("Texture window");

						self.tex_window = TextureWindow {
							mask: Vertex::new((word as i32 & 0x1F) * 8, ((word as i32 >> 5) & 0x1F) * 8),
							offset: Vertex::new(((word as i32 >> 10) & 0x1F) * 8, ((word as i32 >> 15) & 0x1F) * 8),
						};

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
					0xE5 => { 
						self.drawing_offset = Vertex::new(
							(word & 0x7FF) as i32,
							((word >> 11) & 0x7FF) as i32
						);

						trace!("set drawing offset ({}, {})", self.drawing_offset.x, self.drawing_offset.y);

						GP0State::WaitingForNextCmd
					},
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

				if index == 0 {
					self.gp0_params = vec![0; 16];
				}

				self.gp0_params[index as usize] = word;
				trace!("(write 0x{word:X}) words left {}", words_left - 1);

				if words_left == 1 {
					self.exec_cmd(command)
				} else {
					GP0State::WaitingForParams { command: command, index: index + 1, words_left: words_left - 1 }
				}
			},
			GP0State::WaitingForPolyline(params) => {
				self.gp0_params.push(word);

				if word & 0xF000F000 == 0x50005000 {
					// exec polyline
					self.draw_polyline(params);
					GP0State::WaitingForNextCmd
				} else {
					GP0State::WaitingForPolyline(params)
				}
				
			}

			GP0State::RecvData(vram_dma_info) => self.process_cpu_vram_dma(word, vram_dma_info),
			GP0State::SendData(_) => { error!("write 0x{word:X} to GP0 during VRAM to CPU DMA"); GP0State::WaitingForNextCmd },
		}
	}

	fn gp1_cmd(&mut self, word: u32) {
		self.gp1_state = match self.gp1_state {
			GP1State::WaitingForNextCmd => match word >> 24 {
				// Reset GPU
				0x0 => {
					trace!("reset gpu");

					GP1State::WaitingForNextCmd
				},
				// Reset command buffer
				0x1 => {
					trace!("Reset command buffer");
					self.gp0_state = GP0State::WaitingForNextCmd;

					GP1State::WaitingForNextCmd
				},
				// Acknowledge GPU IRQ
				0x2 => {
					trace!("Ack GPU IRQ");
					self.irq = false;
					
					GP1State::WaitingForNextCmd
				},
				// Display Enable
				0x3 => {
					// 0=on, 1=off
					self.display_enabled = word & 1 == 0;

					trace!("Set display enabled: {}", self.display_enabled);

					GP1State::WaitingForNextCmd
				}
				// Dma Direction / Data Request
				0x4 => {
					self.dma_direction = DmaDirection::from_bits(word & 3);

					trace!("set DMA direction: {:?}", self.dma_direction);

					GP1State::WaitingForNextCmd
				},
				// Start of Display area (in VRAM)
				0x5 => {
					self.display_start = Vertex::new((word & 0x3FF) as i32, ((word >> 10) & 0x1FF) as i32);

					trace!("set display start: ({}, {})", self.display_start.x, self.display_start.y);

					GP1State::WaitingForNextCmd
				},
				// Horizontal Display range (on screen)
				0x6 => {
					self.horizontal_display_range = (word & 0xFFF, (word >> 12) & 0xFFF);

					trace!("set horizontal display range: {:X?}", self.horizontal_display_range);

					GP1State::WaitingForNextCmd
				},
				// Vertical Display range (on screen)
				0x7 => {
					self.vertical_display_range = (word & 0xFFF, (word >> 12) & 0xFFF);

					trace!("set horizontal display range: {:X?}", self.vertical_display_range);

					GP1State::WaitingForNextCmd
				},
				// Display mode
				0x8 => {
					self.horizontal_res = HorizontalRes::from_bits(word & 3);
					self.vertical_res = VerticalRes::from_bit((word >> 2) & 1 != 0);
					self.video_mode = VideoMode::from_bit((word >> 3) & 1 != 0);
					self.display_colour_depth = ColourDepth::from_bit((word >> 4) & 1 != 0);
					self.vertical_interlace = (word >> 5) & 1 != 0;
					self.force_h368 = (word >> 6) & 1 != 0;
					self.flip_screen = (word >> 7) & 1 != 0;

					trace!("set display mode");

					GP1State::WaitingForNextCmd
				},
				// Read GPU internel register
				0x10 | 0x11..=0x1F => {
					let index = word & 7;

					self.internal_reg = match index {
						// nothing
						0 ..= 1 => None,
						// TODO texture window
						2 => Some(0),
						// draw area top left
						3 => Some((self.draw_area_top_left.x as u32) | ((self.draw_area_top_left.y as u32) << 10)),
						// draw area bottom right
						4 => Some((self.draw_area_bottom_right.x as u32) | ((self.draw_area_bottom_right.y as u32) << 10)),
						// TODO drawing offset
						5 => Some(0),
						// nothing on v0 gpus
						6 ..= 7 => None,
						_ => unreachable!(),
					};

					GP1State::WaitingForNextCmd
				}
				_ => unimplemented!("unimplemented GP1 command: 0x{:X}", word >> 24),
			}
		}
	}

	pub fn get_display_res(&self) -> (usize, usize) {
		let width = if self.force_h368 {
			368
		} else {
			match self.horizontal_res {
				HorizontalRes::H256 => 256,
				HorizontalRes::H320 => 320,
				HorizontalRes::H512 => 512,
				HorizontalRes::H640 => 640
			}
		};

		let height = match self.vertical_res {
			VerticalRes::V240 => 240,
			VerticalRes::V480 => 480
		};

		(width, height)
	}

	pub fn get_display_start(&self) -> (usize, usize) {
		(self.display_start.x as usize, self.display_start.y as usize)
	}

	pub fn get_dotclock_divider(&self) -> u64 {
		if self.force_h368 {
			7
		} else {
			match self.horizontal_res {
				HorizontalRes::H256 => 10,
				HorizontalRes::H320 => 8,
				HorizontalRes::H512 => 5,
				HorizontalRes::H640 => 4,
			}
		}
	}

	pub fn get_dots_per_scanline(&self) -> u64 {
		let result_f64: f64 = if self.force_h368 {
			487.5714
		} else {
			match self.horizontal_res {
				HorizontalRes::H256 => 341.3,
				HorizontalRes::H320 => 426.625,
				HorizontalRes::H512 => 682.6,
				HorizontalRes::H640 => 853.25
			}
		};

		result_f64.floor() as u64
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
			DrawCommand::DrawLine(cmd) => {
				trace!("draw line (polyline: {})", cmd.polyline);

				let mut v0 = Vertex::from_packet(self.gp0_params[0]);
				let mut v1 = Vertex::from_packet(self.gp0_params[1 + usize::from(cmd.shaded)]);

				if cmd.shaded {
					v0.colour = cmd.colour;
					v1.colour = Colour::from_packet(self.gp0_params[1]);
				} else {
					v0.colour = cmd.colour;
					v1.colour = cmd.colour;
				}

				self.draw_line(v0, v1, cmd.semi_transparent);

				GP0State::WaitingForNextCmd
			}
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

			self.draw_pixel_15bit(halfword, vram_col, vram_row, false);

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

		let mut width = ((self.gp0_params[2].wrapping_sub(1) & 0x3FF) + 1) as u16;
		if width == 0 {
			width = 1024;
		}

		let mut height = (((self.gp0_params[2].wrapping_sub(1) >> 16) & 0x1FF) + 1) as u16;
		if height == 0 {
			height = 512;
		}

		trace!("[VRAM-VRAM DMA] src: (0x{src_x:X}, 0x{src_y:X}) dest: (0x{dest_x:X}, 0x{dest_y:X}) size: ({width}, {height})");

		for y_offset in 0..height {
			for x_offset in 0..width {
				let src_addr = coord_to_vram_index(((src_x + x_offset) & 0x3FF) as u32, ((src_y + y_offset) & 0x1FF) as u32) as usize;

				let src = self.vram[src_addr];
				//trace!("[VRAM-VRAM DMA] draw pixel 0x{src:X} at ({}, {})", (dest_x + x_offset) as u32, (dest_y + y_offset) as u32);
				self.draw_pixel_15bit(src, ((dest_x + x_offset) & 0x3FF) as u32, ((dest_y + y_offset) & 0x1FF) as u32, false);
			}
		}
	}
	
	fn draw_polyline(&mut self, params: LineCmdParams) {
		if params.shaded {
			let verts: Vec<Vertex> = self.gp0_params
				.chunks_exact(2)
				.map(|chunk| {
					let mut vertex = Vertex::from_packet(chunk[0]);
					vertex.colour = Colour::from_packet(chunk[1]);

					vertex
				})
				.collect();

			verts
				.windows(2)
				.map(|v| (&v[0], &v[1]))
				.for_each(|(v0, v1)| self.draw_line(*v0, *v1, params.semi_transparent));
		} else {
			let verts: Vec<Vertex> = self.gp0_params.drain(..).map(|v| {
				let mut vertex = Vertex::from_packet(v);
				vertex.colour = params.colour;

				vertex
			})
			.collect();

			verts
				.windows(2)
				.map(|v| (&v[0], &v[1]))
				.for_each(|(v0, v1)| self.draw_line(*v0, *v1, params.semi_transparent));
		}
	}

	fn draw_line(&mut self, v0: Vertex, v1: Vertex, semi_transparent: bool) {
		if !vertices_valid(v0, v1) {
			return;
		}

		let dx = v1.x - v0.x;
		let dy = v1.y - v0.y;

		let diff_r = i32::from(v1.colour.r) - i32::from(v0.colour.r);
		let diff_g = i32::from(v1.colour.g) - i32::from(v0.colour.g);
		let diff_b = i32::from(v1.colour.b) - i32::from(v0.colour.b);

		let (x_step, y_step, r_step, g_step, b_step) = if dx.abs() >= dy.abs() {
			let y_step = f64::from(dy) / f64::from(dx.abs());
			
			let r_step = f64::from(diff_r) / f64::from(dx.abs());
			let g_step = f64::from(diff_g) / f64::from(dx.abs());
			let b_step = f64::from(diff_b) / f64::from(dx.abs());

			(f64::from(dx.signum()), y_step, r_step, g_step, b_step)
		} else {
			let x_step = f64::from(dx) / f64::from(dy.abs());
			
			let r_step = f64::from(diff_r) / f64::from(dy.abs());
			let g_step = f64::from(diff_g) / f64::from(dy.abs());
			let b_step = f64::from(diff_b) / f64::from(dy.abs());

			(x_step, f64::from(dy.signum()), r_step, g_step, b_step)
		};

		let mut r = f64::from(v0.colour.r);
		let mut g = f64::from(v0.colour.g);
		let mut b = f64::from(v0.colour.b);

		let mut x = f64::from(v0.x);
		let mut y = f64::from(v0.y);

		while x.round() as i32 != v1.x || y.round() as i32 != v1.y {
			let vertex = Vertex::new(x.round() as i32, y.round() as i32);
			let mut colour = Colour::from_rgb888(r as u8, g as u8, b as u8);

			// ensure pixel is within the drawing area
			if (self.draw_area_top_left.x..=self.draw_area_bottom_right.x).contains(&vertex.x)
            	&& (self.draw_area_top_left.y..=self.draw_area_bottom_right.y).contains(&vertex.y) {

				if self.tex_page.dithering {
					colour = apply_dithering(colour, vertex)
				}

				self.draw_pixel_15bit(colour.truncate_to_15bit(), vertex.x as u32, vertex.y as u32, semi_transparent);

			}

			r += r_step;
			g += g_step;
			b += b_step;

			x += x_step;
			y += y_step;
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

		let min_x = cmd.position.x + self.drawing_offset.x;
		let max_x = cmd.position.x + self.drawing_offset.x + cmd.size.x - 1;
		let min_y = cmd.position.y + self.drawing_offset.y;
		let max_y = cmd.position.y + self.drawing_offset.y + cmd.size.y - 1;

		for y in min_y..=max_y {

			if y < self.draw_area_top_left.y || y > self.draw_area_bottom_right.y {
            	continue;
        	}

			for x in min_x..=max_x {

				if x < self.draw_area_top_left.x || x > self.draw_area_bottom_right.x {
                	continue;
            	}

				let (draw_colour, semi_transparent) = if cmd.textured {
					let tex_colour_u16 = self.sample_texture(Vertex::new(cmd.position.tex_x + (x - min_x), cmd.position.tex_y + (y - min_y)), cmd.clut);
					let tex_colour =  Colour::rgb555_to_rgb888(tex_colour_u16);

					if tex_colour_u16 == 0 {
						continue;
					}

					let semi_transparent = cmd.semi_transparent && tex_colour_u16 & 0x8000 != 0;

					if !cmd.raw_texture {
						(apply_modulation(tex_colour, cmd.colour), semi_transparent)
					} else {
						(tex_colour, semi_transparent)
					}
				} else {
					(cmd.colour, cmd.semi_transparent)
				};

				self.draw_pixel_15bit(u16::from(draw_colour.truncate_to_15bit()), x as u32, y as u32, semi_transparent);
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

		for i in 0..cmd.vertices as usize {
			if i != 0 && cmd.shaded {
				v[i].colour = Colour::from_packet(self.gp0_params.remove(0));
			}
			
			let vertex = Vertex::from_packet(self.gp0_params.remove(0)).plus_offset(self.drawing_offset);
			v[i].x = vertex.x;
			v[i].y = vertex.y;

			if cmd.textured {
				let tex_word = self.gp0_params.remove(0);

				match i {
					0 => cmd.clut = Vertex::new((((tex_word >> 16) & 0x3F) as u16 as i32) * 16, ((tex_word >> 22) & 0x1FF) as u16 as i32),
					1 => {
						let tex_page = tex_word >> 16;

						self.tex_page = TexturePage {
							x_base: 64 * (tex_page & 0xF),
							y_base: 256 * ((tex_page >> 4) & 1),
							bit_depth: TexBitDepth::from_bits((tex_page >> 7) & 3),
							..self.tex_page
						};
					},
					_ => {},
				}

				v[i].tex_x = (tex_word & 0xFF) as i32;
				v[i].tex_y = ((tex_word >> 8) & 0xFF) as i32;
			}

		}

		v[0].colour = cmd.colour;

		ensure_vertex_order(&mut v);
		self.draw_triangle(v[0], v[1], v[2], cmd);

		if cmd.vertices == 4 {
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
			//return;
		}

		if !vertices_valid(v0, v1) || !vertices_valid(v1, v2) || !vertices_valid(v2, v0) {
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

					let (textured_colour, semi_transparent) = if cmd.textured {
						
						let coords = compute_barycentric_coords(p, v0, v1, v2);
						let interpolated_coords = interpolate_uv(coords, [v0, v1, v2]);

						let tex_colour_u16 = self.sample_texture(interpolated_coords, cmd.clut);
						let tex_colour = Colour::rgb555_to_rgb888(tex_colour_u16);

						// black is transparent in textures
						if tex_colour_u16 == 0 {
							continue;
						}

						// bit 15 of the texture colour specifies if this pixel should be semi-transparent
						let semi_transparent = cmd.semi_transparent && tex_colour_u16 & 0x8000 != 0;

						if !cmd.raw_texture {
							(apply_modulation(tex_colour, shaded_colour), semi_transparent)
						} else {
							(tex_colour, semi_transparent)
						}

					} else {
						(shaded_colour, cmd.semi_transparent)
					};

					self.draw_pixel_15bit(textured_colour.truncate_to_15bit(), x as u32, y as u32, semi_transparent);
				}
			}
		}
	}

	fn sample_texture(&mut self, tex_coords: Vertex, clut: Vertex) -> u16 {

		let masked_coords = Vertex::new(
			tex_coords.x & (!(self.tex_window.mask.x)) | (self.tex_window.offset.x & self.tex_window.mask.x),
			tex_coords.y & (!(self.tex_window.mask.y)) | (self.tex_window.offset.y & self.tex_window.mask.y)
		);

		match self.tex_page.bit_depth {
			TexBitDepth::FourBit => {
				let tex_x = self.tex_page.x_base + ((masked_coords.x as u32) / 4);
				let tex_y = self.tex_page.y_base + masked_coords.y as u32;

				let texel = self.vram[coord_to_vram_index(tex_x, tex_y) as usize];
				let clut_index = ((texel >> (masked_coords.x % 4) * 4) & 0xF) as u32;

				self.vram[coord_to_vram_index(clut.x as u32 + clut_index, clut.y as u32) as usize]
			},
			TexBitDepth::EightBit => {
				let tex_x = self.tex_page.x_base + ((masked_coords.x as u32) / 2);
				let tex_y = self.tex_page.y_base + masked_coords.y as u32;

				let texel = self.vram[coord_to_vram_index(tex_x, tex_y) as usize];
				let clut_index = ((texel >> (masked_coords.x % 2) * 8) & 0xFF) as u32;

				self.vram[coord_to_vram_index(clut.x as u32 + clut_index, clut.y as u32) as usize]
			}
			TexBitDepth::FiveteenBit => {
				let tex_x = self.tex_page.x_base + (masked_coords.x as u32);
				let tex_y = self.tex_page.y_base + (masked_coords.y as u32);
				let texel = self.vram[coord_to_vram_index(tex_x, tex_y) as usize];

				texel
			},
		}

	}

	fn draw_pixel_15bit(&mut self, colour: u16, x: u32, y: u32, semi_transparent: bool) {
		let mask = u16::from(self.force_mask_bit) << 15;

		let old_pixel = self.vram[coord_to_vram_index(x, y) as usize];

		if self.check_mask_bit && old_pixel & 0x8000 != 0 {
			return;
		}

		let draw_pixel = if semi_transparent {
			self.apply_semi_transparency(old_pixel, colour)
		} else {
			colour
		};

		self.vram[coord_to_vram_index(x, y) as usize] = draw_pixel | mask;
	}

	fn apply_semi_transparency(&self, background: u16, foreground: u16) -> u16 {
		let b = Colour::from_rgb555(background);
		let f = Colour::from_rgb555(foreground);

		let result = match self.tex_page.transp_type {
			SemiTransparency::HalfBPlusHalfF => Colour {
				r: ((0.5 * b.r as f64) + (0.5 * f.r as f64)).clamp(0.0, 31.0) as u8,
				g: ((0.5 * b.g as f64) + (0.5 * f.g as f64)).clamp(0.0, 31.0) as u8,
				b: ((0.5 * b.b as f64) + (0.5 * f.b as f64)).clamp(0.0, 31.0) as u8,
			},
			SemiTransparency::BPlusF => Colour {
				r: (b.r as f64 + f.r as f64).clamp(0.0, 31.0) as u8,
				g: (b.g as f64 + f.g as f64).clamp(0.0, 31.0) as u8,
				b: (b.b as f64 + f.b as f64).clamp(0.0, 31.0) as u8,
			},
			SemiTransparency::BMinusF => Colour {
				r: (b.r as f64 - f.r as f64).clamp(0.0, 31.0) as u8,
				g: (b.g as f64 - f.g as f64).clamp(0.0, 31.0) as u8,
				b: (b.b as f64 - f.b as f64).clamp(0.0, 31.0) as u8,
			},
			SemiTransparency::BPlusFOver4 => Colour {
				r: (b.r as f64 + (0.25 * f.r as f64)).clamp(0.0, 31.0) as u8,
				g: (b.g as f64 + (0.25 * f.g as f64)).clamp(0.0, 31.0) as u8,
				b: (b.b as f64 + (0.25 * f.b as f64)).clamp(0.0, 31.0) as u8,
			}
		};

		(result.r as u16) | ((result.g as u16) << 5) | ((result.b as u16) << 10)
	}
	
}

fn coord_to_vram_index(x: u32, y: u32) -> u32 {
	(1024 * (y & 0x1FF)).wrapping_add(x & 0x3FF)
}

fn cross_product_z(v0: Vertex, v1: Vertex, v2: Vertex) -> i32 {
	let result = (v1.x - v0.x) * (v2.y - v0.y) - (v1.y - v0.y) * (v2.x - v0.x);
	
	result
}

fn ensure_vertex_order(v: &mut [Vertex]) -> bool {
	let cross_product_z = cross_product_z(v[0], v[1], v[2]);

	if cross_product_z < 0 {
		v.swap(1, 2);
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

	let u = (lambda[0] * coords_x[0] + lambda[1] * coords_x[1] + lambda[2] * coords_x[2]).round() as i32;
	let v = (lambda[0] * coords_y[0] + lambda[1] * coords_y[1] + lambda[2] * coords_y[2]).round() as i32;

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

fn apply_modulation(tex_colour: Colour, shaded_colour: Colour) -> Colour {
	Colour {
		r: (((tex_colour.r as i32) * (shaded_colour.r as i32)) >> 7).clamp(0, 255) as u8,
		g: (((tex_colour.g as i32) * (shaded_colour.g as i32)) >> 7).clamp(0, 255) as u8,
		b: (((tex_colour.b as i32) * (shaded_colour.b as i32)) >> 7).clamp(0, 255) as u8,
	}
}

fn vertices_valid(v0: Vertex, v1: Vertex) -> bool {
	(v0.x - v1.x).abs() < 1024 && (v0.y - v1.y).abs() < 512
}
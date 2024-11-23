#[derive(Debug, Clone, Copy)]
enum DrawCommand {
	CpuVramDma,
	VramCpuDma,
	VramVramDma,
	DrawRect(u32),
	QuickFill(u32)
}

enum GP0State {
	WaitingForNextCmd,
	WaitingForParams { command: DrawCommand, index: u8, max_index: u8 },
	RecvData(VramDmaInfo),
	SendData(VramDmaInfo),
}

enum GP1State {
	WaitingForNextCmd,
}



#[derive(Clone, Copy)]
struct VramDmaInfo {
	dest_x: u16,
	dest_y: u16,

	width: u16,
	height: u16,

	current_row: u16,
	current_col: u16,
}

pub struct Gpu {
	pub vram: Box<[u8]>,

	gp0_state: GP0State,
	gp0_params: [u32; 16],

	gp1_state: GP1State,
	gp1_params: [u32; 16],

	reg_gpustat: u32,
	reg_gpuread: u32,
}

impl Gpu {
	pub fn new() -> Self {
		Self {
			vram: vec![0; 512 * 2048].into_boxed_slice().try_into().unwrap(),

			gp0_state: GP0State::WaitingForNextCmd,
			gp0_params: [0; 16],

			gp1_state: GP1State::WaitingForNextCmd,
			gp1_params: [0; 16],

			reg_gpustat: 0b01011110100000000000000000000000,
			reg_gpuread: 0,
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
			0x1F801814 => self.reg_gpustat,
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

	fn gp0_cmd(&mut self, word: u32) {
		self.gp0_state = match self.gp0_state {

			GP0State::WaitingForNextCmd => match word >> 29 {
				// 0/7 cmds need to be decoded with the highest 8 bits
				0 => match word >> 24 {
					0 | 0x3..=0x1E => {
						println!("NOP / unused");

						GP0State::WaitingForNextCmd
					},
					0x01 => {
						// not emulating the texture cache, this does nothing
						println!("clear texture cache");

						GP0State::WaitingForNextCmd
					},
					0x02 => {

						GP0State::WaitingForParams { command: DrawCommand::QuickFill(word), index: 0, max_index: 1 }
					},
					0x1F => {
						unimplemented!("GP0 IRQ")
					}

					_ => todo!("Misc cmd 0x{word:X}")
				}

				1 => todo!("draw polygon"),
				2 => todo!("draw line"),
				3 => GP0State::WaitingForParams { command: DrawCommand::DrawRect(word), index: 0, max_index: 0 },
				4 => GP0State::WaitingForParams { command: DrawCommand::VramVramDma, index: 0, max_index: 2 },
				5 => GP0State::WaitingForParams { command: DrawCommand::CpuVramDma, index: 0, max_index: 1 },
				6 => GP0State::WaitingForParams { command: DrawCommand::VramCpuDma, index: 0, max_index: 1 },

				7 => match word >> 24 {
					_ => { println!("Enviroment cmd 0x{word:X}"); GP0State::WaitingForNextCmd }
				}

				_ => unreachable!()
			},

			GP0State::WaitingForParams { command, index, max_index } => {

				self.gp0_params[index as usize] = word;

				if index == max_index {
					self.exec_cmd(command)
				} else {
					GP0State::WaitingForParams { command: command, index: index + 1, max_index: max_index }
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
				self.draw_pixel(cmd, self.gp0_params[0]);

				GP0State::WaitingForNextCmd
			},
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

			let [lsb, msb] = halfword.to_le_bytes();

			let vram_addr = coord_to_vram_index(vram_col, vram_row) as usize;
			self.vram[vram_addr] = lsb;
			self.vram[vram_addr + 1] = msb;

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

		let mut result: [u8; 4] = [0; 4];

		for i in 0..2 {

			// wrap from 511 to 0
			let vram_row = ((info.dest_y + info.current_row) & 0x1FF) as u32;
			// wrap from 1023 to 0
			let vram_col = ((info.dest_x + info.current_col) & 0x3FF) as u32;

			let vram_addr = coord_to_vram_index(vram_col, vram_row) as usize;
			result[i + 0] = self.vram[vram_addr + 0];
			result[i + 1] = self.vram[vram_addr + 1];

			info.current_col += 1;

			if info.current_col == info.width {
				info.current_col = 0;
				info.current_row += 1;

				if info.current_row == info.height {
					self.gp0_state = GP0State::WaitingForNextCmd;
				} else {
					self.gp0_state = GP0State::SendData(info);
				}
			}

		}

		u32::from_le_bytes(result)
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

				let src_lsb = self.vram[src_addr];
				let src_msb = self.vram[src_addr + 1];

				self.vram[dest_addr] = src_lsb;
				self.vram[dest_addr + 1] = src_msb;
			}
		}
	}

	fn draw_pixel(&mut self, cmd: u32, param: u32) {
		let r = (cmd & 0xFF) >> 3;
		let g = ((cmd >> 8) & 0xFF) >> 3;
		let b = ((cmd >> 16) & 0xFF) >> 3;

		let pixel = (r | (g << 5) | (b << 10)) as u16;
		let [pixel_lsb, pixel_msb] = pixel.to_le_bytes();

		let x = param & 0x3FF;
		let y = (param >> 16) & 0x1FF;

		if pixel != 0 {
			//println!("draw pixel ({r}, {g}, {b}) at ({x}, {y})");
		}

		let vram_addr = coord_to_vram_index(x, y) as usize;
		self.vram[vram_addr] = pixel_lsb;
		self.vram[vram_addr + 1] = pixel_msb;
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

		println!("quick fill at ({x}, {y}) of size ({width}, {height}) cmd: ${cmd:X} r:{r} g:{g} b{b} colour: ${colour:X}");

		for y_offset in 0..height {
			for x_offset in 0..width {
				let [colour_lsb, colour_msb] = colour.to_le_bytes();

				let index = coord_to_vram_index((x + x_offset) & 0x3FF, (y + y_offset) & 0x1FF);

				self.vram[index as usize] = colour_lsb;
				self.vram[(index + 1) as usize] = colour_msb;
			}
		}

	}
}

fn coord_to_vram_index(x: u32, y: u32) -> u32 {
	2 * (1024 * y).wrapping_add(x)
}
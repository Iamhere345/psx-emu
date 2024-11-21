enum GP0State {
	WaitingForNextCmd,
	RecvCpuVramDmaParams { idx: u8 },
	RecvVramCpuDmaParams { idx: u8 },
	RecvDrawRectParams { idx: u8, cmd: u32 },
	RecvData(VramDmaInfo),
	SendData(VramDmaInfo),
}

enum GP1State {
	WaitingForNextCmd,
}

#[derive(Clone, Copy)]
struct VramDmaInfo {
	vram_x: u16,
	vram_y: u16,
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
				1 => todo!("draw polygon"),
				2 => todo!("draw line"),
				3 => GP0State::RecvDrawRectParams { idx: 0, cmd: word },
				4 => todo!("VRAM-VRAM DMA"),
				5 => GP0State::RecvCpuVramDmaParams { idx: 0 },
				6 => GP0State::RecvVramCpuDmaParams { idx: 0 },
				0 | 7 => GP0State::WaitingForNextCmd,//{ println!("Misc: 0x{word:X} 0b{:b}", word >> 29); GP0State::WaitingForNextCmd },
				_ => unreachable!()
			}

			GP0State::RecvCpuVramDmaParams { idx } => {
				self.gp0_params[idx as usize] = word;

				if idx == 1 {
					self.init_dma()
				} else {
					GP0State::RecvCpuVramDmaParams { idx: idx + 1 }
				}
			},

			GP0State::RecvVramCpuDmaParams { idx } => {
				self.gp0_params[idx as usize] = word;

				if idx == 1 {
					self.init_dma()
				} else {
					GP0State::RecvVramCpuDmaParams { idx: idx + 1 }
				}
			},

			GP0State::RecvDrawRectParams { idx, cmd } => {
				self.gp0_params[idx as usize] = word;
				
				self.draw_pixel(cmd, word);
				GP0State::WaitingForNextCmd

			}

			GP0State::RecvData(vram_dma_info) => self.process_cpu_vram_dma(word, vram_dma_info),
			GP0State::SendData(vram_dma_info) => GP0State::SendData(vram_dma_info),
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

	fn init_dma(&mut self) -> GP0State {

		let vram_x = (self.gp0_params[0] & 0x3FF) as u16;
		let vram_y = ((self.gp0_params[0] >> 16) & 0x1FF) as u16;

		let mut width = (self.gp0_params[1] & 0x3FF) as u16;
		if width == 0 {
			width = 1024;
		}

		let mut height = ((self.gp0_params[1] >> 16) & 0x1FF) as u16;
		if height == 0 {
			height = 512;
		}

		GP0State::RecvData(VramDmaInfo {
			vram_x,
			vram_y,
			width,
			height,
			current_row: 0,
			current_col: 0,
		})
	}

	fn process_cpu_vram_dma(&mut self, word: u32, mut info: VramDmaInfo) -> GP0State {
		for i in 0..2 {

			let halfword = (word >> (16 * i)) as u16;

			// wrap from 511 to 0
			let vram_row = ((info.vram_y + info.current_row) & 0x1FF) as usize;
			// wrap from 1023 to 0
			let vram_col = ((info.vram_x + info.current_col) & 0x3FF) as usize;

			let [lsb, msb] = halfword.to_le_bytes();

			let vram_addr = 2 * (vram_col + 1024 * vram_row);
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
			let vram_row = ((info.vram_y + info.current_row) & 0x1FF) as usize;
			// wrap from 1023 to 0
			let vram_col = ((info.vram_x + info.current_col) & 0x3FF) as usize;

			let vram_addr = 2 * (vram_col + 1024 * vram_row);
			result[i + 0] = self.vram[vram_addr + 0];
			result[i + 1] = self.vram[vram_addr + 1];

			info.current_col += 1;

			if info.current_col == info.width {
				info.current_col = 0;
				info.current_row += 1;

				if info.current_row == info.height {
					self.gp0_state = GP0State::WaitingForNextCmd;
				}
			}

		}

		self.gp0_state = GP0State::SendData(info);

		u32::from_le_bytes(result)
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

		let vram_addr = 2 * (x + 1024 * y) as usize;
		self.vram[vram_addr] = pixel_lsb;
		self.vram[vram_addr + 1] = pixel_msb;
	}
}
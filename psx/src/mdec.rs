use std::{collections::VecDeque, mem, usize};
use log::*;

const ZAGZIG: [usize; 64] = [
	00, 01, 08, 16, 09, 02, 03, 10,
    17, 24, 32, 25, 18, 11, 04, 05,
    12, 19, 26, 33, 40, 48, 41, 34,
    27, 20, 13, 06, 07, 14, 21, 28,
    35, 42, 49, 56, 57, 50, 43, 36, 
    29, 22, 15, 23, 30, 37, 44, 51,
    58, 59, 52, 45, 38, 31, 39, 46,
    53, 60, 61, 54, 47, 55, 62, 63
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CmdState {
	WaitingForNextCmd,
	WaitingForParams { cmd: MdecCmd, words_left: u16 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MdecCmd {
	Nop,
	DecodeMacroblock,
	SetQuant(bool),
	SetScale,
}

#[derive(Debug, Clone, Copy)]
enum OutputDepth {
	BPP4 	= 0,
	BPP8 	= 1,
	BPP24 	= 2,
	BPP15 	= 3,
}

impl OutputDepth {
	fn from_bits(bits: u32) -> Self {
		match bits {
			0 => Self::BPP4,
			1 => Self::BPP8,
			2 => Self::BPP24,
			3 => Self::BPP15,

			_ => unimplemented!()
		}
	}
}

pub struct Mdec {
	cmd_state: CmdState,

	input_fifo: VecDeque<u16>,
	output_fifo: VecDeque<u8>,

	output_depth: OutputDepth,
	output_signed: bool,
	output_bit15: bool,

	// reflected in stat but otherwise unused
	dma0_enable: bool,
	dma1_enable: bool,

	luminance_quant_table: [u8; 64],
	colour_quant_table: [u8; 64],
	scale_table: [i16; 64],

	cr_block: [i32; 64],
	cb_block: [i32; 64],
	y_block: [i32; 64],
}

impl Mdec {
	pub fn new() -> Self {
		Self {
			cmd_state: CmdState::WaitingForNextCmd,

			input_fifo: VecDeque::new(),
			output_fifo: VecDeque::new(),

			output_depth: OutputDepth::BPP4,
			output_signed: false,
			output_bit15: false,

			dma0_enable: false,
			dma1_enable: false,

			luminance_quant_table: [0; 64],
			colour_quant_table: [0; 64],
			scale_table: [0; 64],

			cr_block: [0; 64],
			cb_block: [0; 64],
			y_block: [0; 64],
		}
	}

	pub fn read32(&mut self, addr: u32) -> u32 {
		match addr {
			0x1F801820 => {
				let mut bytes = [0; 4];
				for byte in &mut bytes  {
					*byte = self.output_fifo.pop_front().unwrap_or(0xAA);
				}

				trace!("read output: 0x{:X}", u32::from_le_bytes(bytes));

				u32::from_le_bytes(bytes)
			},
			0x1F801824 => self.read_stat(),

			_ => unimplemented!("[0x{addr:X}] Invalid MDEC read32"),
		}
	}

	pub fn write32(&mut self, addr: u32, write: u32) {
		match addr {
			0x1F801820 => self.write_cmd(write),
			0x1F801824 => self.write_ctrl(write),

			_ => unimplemented!("[0x{addr:X}] Invalid MDEC read32"),
		}
	}

	fn read_stat(&self) -> u32 {
		let words_left: u32 = match self.cmd_state {
			CmdState::WaitingForNextCmd => 0xFFFF,
			CmdState::WaitingForParams { cmd, words_left } => {
				if cmd == MdecCmd::Nop {
					words_left as u32
				} else {
					words_left as u32 - 1
				}
			}
		};

		words_left 
			| (0) << 16 // Current Block (0..3=Y1..Y4, 4=Cr, 5=Cb) (or for mono: always 4=Y)
			| (self.output_bit15 as u32) << 23
			| (self.output_signed as u32) << 24
			| (self.output_depth as u32) << 25
			| (self.dma1_enable as u32) << 27 // Data-Out Request (set when DMA1 enabled and ready to send data)
			| (self.dma0_enable as u32) << 28 // Data-In Request  (set when DMA0 enabled and ready to receive data)
			| (matches!(self.cmd_state, CmdState::WaitingForParams { .. }) as u32) << 29 // Command Busy  (0=Ready, 1=Busy receiving or processing parameters)
			| (!self.output_fifo.is_empty() as u32) << 30 // Data-In Fifo Full (0=No, 1=Full, or Last word received)
			| (self.output_fifo.is_empty() as u32) << 31 // Data-Out Fifo Empty (0=No, 1=Empty)
	}

	fn write_ctrl(&mut self, write: u32) {
		self.dma0_enable = (write >> 30) & 1 != 0;
		self.dma1_enable = (write >> 29) & 1 != 0;

		// Reset MDEC
		if (write >> 31) & 1 != 0 {
			self.cmd_state = CmdState::WaitingForNextCmd;

			self.output_bit15 = false;
			self.output_signed = false;
			self.output_depth = OutputDepth::BPP4;

			self.input_fifo.clear();
		}
	}

	fn write_cmd(&mut self, write: u32) {
		self.cmd_state = match self.cmd_state {
			CmdState::WaitingForNextCmd => {
				match write >> 29 {
					// Decode Macroblock
					1 => {
						self.output_depth = OutputDepth::from_bits((write >> 27) & 3);
						self.output_signed = ((write >> 26) & 1) != 0;
						self.output_bit15 = ((write >> 25) & 1) != 0;

						debug!("DecodeMacroblock depth: {:?} signed: {} bit15: {} len: {} halfwords", self.output_depth, self.output_signed, self.output_bit15, (write & 0xFFFF) * 2);

						CmdState::WaitingForParams { cmd: MdecCmd::DecodeMacroblock, words_left: (write & 0xFFFF) as u16 }
					},
					// Set Quant Table
					2 => {
						// These arent part of the command but are still copied to the status register
						self.output_depth = OutputDepth::from_bits((write >> 27) & 3);
						self.output_signed = ((write >> 26) & 1) != 0;
						self.output_bit15 = ((write >> 25) & 1) != 0;

						let recv_colour_table = (write & 1) != 0;

						// 64 bytes for luminance, 64 bytes for colour (if enabled)
						let words_left = (64 / 4) * (1 + (1 * u16::from(recv_colour_table)));

						debug!("SetQuant (colour table: {recv_colour_table} words left: {words_left})");

						CmdState::WaitingForParams { cmd: MdecCmd::SetQuant(recv_colour_table), words_left: words_left }
					},
					// Set Scale Table
					3 => {
						debug!("SetScale");

						// These arent part of the command but are still copied to the status register
						self.output_depth = OutputDepth::from_bits((write >> 27) & 3);
						self.output_signed = ((write >> 26) & 1) != 0;
						self.output_bit15 = ((write >> 25) & 1) != 0;

						CmdState::WaitingForParams { cmd: MdecCmd::SetScale, words_left: 64 / 2 }
					},
					_ => unimplemented!("MDEC cmd {}", write >> 29)
				}
			},
			CmdState::WaitingForParams { cmd, words_left } => {

				self.input_fifo.push_back(write as u16);
				self.input_fifo.push_back((write >> 16) as u16);

				trace!("[{cmd:?}] write param 0x{write:X} (words left: {words_left}");

				if words_left == 1 {
					trace!("Exec cmd {cmd:?}");

					self.exec_cmd(cmd);
					self.input_fifo.clear();

					CmdState::WaitingForNextCmd
				} else {
					CmdState::WaitingForParams { cmd: cmd, words_left: words_left - 1 }
				}
			}
		}
	}

	fn exec_cmd(&mut self, cmd: MdecCmd) {
		match cmd {
			MdecCmd::Nop => {},
			MdecCmd::DecodeMacroblock => self.decode_macroblock(),
			MdecCmd::SetQuant(recv_colour_table) => self.set_quant_table(recv_colour_table),
			MdecCmd::SetScale => self.set_scale_table(),
		}
	}

	
	fn set_quant_table(&mut self, recv_colour_table: bool) {
		for i in 0..64 / 2 {
			let halfword = self.input_fifo.pop_front().expect("luminance table halfwords");
			self.luminance_quant_table[2 * i..2 * (i + 1)].copy_from_slice(&halfword.to_le_bytes());
		}
		
		if recv_colour_table {
			for i in 0..64 / 2 {
				let halfword = self.input_fifo.pop_front().expect("colour table halfwords");
				self.colour_quant_table[2 * i..2 * (i + 1)].copy_from_slice(&halfword.to_le_bytes());
			}
		}

		trace!("set luminance table: {:X?}", self.luminance_quant_table);
		trace!("set colour table: {:X?}", self.colour_quant_table);
	}
	
	fn set_scale_table(&mut self) {
		for (i, &halfword) in self.input_fifo.iter().enumerate() {
			self.scale_table[i] =  halfword as i16;
		}

		trace!("set scale table: {:X?}", self.scale_table)
	}

	fn decode_macroblock(&mut self) {
		match self.output_depth {
			OutputDepth::BPP15 | OutputDepth::BPP24 => self.decode_colour_macroblock(),
			OutputDepth::BPP4 | OutputDepth::BPP8 => self.decode_monochrome_macroblock(),
		}
	}

	fn decode_monochrome_macroblock(&mut self) {
		self.output_fifo.clear();

		decode_block(&mut self.input_fifo, &mut self.y_block, &self.luminance_quant_table, &self.scale_table);

		// y_to_mono
		let mut mono_out = [0; 64];
		for (i, &y) in self.y_block.iter().enumerate() {
			// clip to signed 9bit range
			let y = (y << (23)) >> (23);

			// Clamp to signed 8-bit
			let mut y = y.clamp(-128, 127);

			if !self.output_signed {
				y += 128;
			}

			mono_out[i] = y as u8;
		}

		match self.output_depth {
			OutputDepth::BPP4 => {
				for chunk in mono_out.chunks_exact(2) {
					let byte = (chunk[0] >> 4) | (chunk[1] & 0xF0);
					self.output_fifo.push_back(byte);
				}
			},
			OutputDepth::BPP8 => {
				for byte in mono_out.iter() {
					self.output_fifo.push_back(*byte);
				}
			},
			_ => unimplemented!(),
		}

		trace!("pushed {} bytes to fifo", self.output_fifo.len());
	}

	fn decode_colour_macroblock(&mut self) {
		let mut count = 0;

		loop {
			// Cr
			if !decode_block(&mut self.input_fifo, &mut self.cr_block, &self.colour_quant_table, &self.scale_table) {
				break;
			}

			// Cb
			decode_block(&mut self.input_fifo, &mut self.cb_block, &self.colour_quant_table, &self.scale_table);

			let mut colour_out = [0; 0x300];

			// Y1
			decode_block(&mut self.input_fifo, &mut self.y_block, &self.luminance_quant_table, &self.scale_table);
			self.yuv_to_rgb(0, 0, &mut colour_out);
			// Y2
			decode_block(&mut self.input_fifo, &mut self.y_block, &self.luminance_quant_table, &self.scale_table);
			self.yuv_to_rgb(8, 0, &mut colour_out);
			// Y3
			decode_block(&mut self.input_fifo, &mut self.y_block, &self.luminance_quant_table, &self.scale_table);
			self.yuv_to_rgb(0, 8, &mut colour_out);
			// Y4
			decode_block(&mut self.input_fifo, &mut self.y_block, &self.luminance_quant_table, &self.scale_table);
			self.yuv_to_rgb(8, 8, &mut colour_out);
			
			// push 16x16 output to fifo
			match self.output_depth {
				OutputDepth::BPP15 => {
					for y in 0..16 {
						for x in 0..16 {
							let rgb555_addr = (16 * y + x) * 2;

							for i in rgb555_addr ..= rgb555_addr + 1 {
								self.output_fifo.push_back(colour_out[i]);
							}
						}
					}
				},
				OutputDepth::BPP24 => {
					for y in 0..16 {
						for x in 0..16 {
							let rgb888_addr = (16 * y + x) * 3;

							for i in rgb888_addr ..= rgb888_addr + 2 {
								self.output_fifo.push_back(colour_out[i]);
							}
						}
					}
				},
				_ => unreachable!(),
			}

			count += 1;
		}

		debug!("finished decode macroblocks: {count}");
	}

	fn yuv_to_rgb(&mut self, xx: usize, yy: usize, colour_out: &mut [u8; 0x300]) {
		for y in 0..8 {
			for x in 0..8 {
				let mut r = self.cr_block[(((x + xx) / 2) + ((y + yy) / 2) * 8) as usize];
				let mut b = self.cb_block[(((x + xx) / 2) + ((y + yy) / 2) * 8) as usize];

				let g = (-0.3437 * f64::from(b) - 0.7143 * f64::from(r)).round() as i32;
				r = (1.402 * f64::from(r)).round() as i32;
				b = (1.772 * f64::from(b)).round() as i32;

				let l = self.y_block[(x + y * 8) as usize];
				let mut r = (l + r).clamp(-128, 127) as i16;
				let mut g = (l + g).clamp(-128, 127) as i16;
				let mut b = (l + b).clamp(-128, 127) as i16;

				if !self.output_signed {
					r += 128;
					g += 128;
					b += 128;
				}

				match self.output_depth {
					OutputDepth::BPP24 => {
						colour_out[(0 + ((x + xx) + (y + yy) * 16) * 3) as usize] = r as u8;
						colour_out[(1 + ((x + xx) + (y + yy) * 16) * 3) as usize] = g as u8;
						colour_out[(2 + ((x + xx) + (y + yy) * 16) * 3) as usize] = b as u8;
					},
					OutputDepth::BPP15 => {
						let r5 = (r as u8) >> 3;
						let g5 = (g as u8) >> 3;
						let b5 = (b as u8) >> 3;

						let mut rgb = ((b5 as u16) << 10) | ((g5 as u16) << 5) | (r5 as u16);
						if self.output_bit15 {
							rgb |= 0x8000;
						}

						colour_out[(0 + ((x + xx) + (y + yy) * 16) * 2) as usize] = rgb as u8;
						colour_out[(1 + ((x + xx) + (y + yy) * 16) * 2) as usize] = (rgb >> 8) as u8;
					},
					_ => unreachable!(),
				}
			}
		}
	}

}

fn decode_block(src: &mut VecDeque<u16>, block: &mut [i32; 64], quant_table: &[u8; 64], scale_table: &[i16; 64]) -> bool {
	block.fill(0);

	while src.front().copied() == Some(0xFE00) {
		debug!("Skip padding");
		src.pop_front();
	}

	let Some(mut n) = src.pop_front() else { return false };

	let quant_scale = n >> 10;
	
	let mut value = i10(n & 0x3FF) * i32::from(quant_table[0]);
	let mut k = 0;

	while k < 64 {
		if quant_scale == 0 {
			value = i10(n & 0x3FF) * 2;
		}

		value = value.clamp(-0x400, 0x3FF);

		if quant_scale > 0 {
			block[ZAGZIG[k as usize]] = value;
		} else {
			block[k as usize] = value;
		}

		// avoids off-by-one error for index of next block
		if k == 63 {
			break;
		}

		n = src.pop_front().unwrap_or(0xFE00);

		k += (n >> 10) + 1;

		if k >= 64 {
			break;
		}

		value = (i10(n & 0x3FF) * i32::from(quant_table[k as usize]) * i32::from(quant_scale) + 4) / 8;
	}

	idct_core(block, scale_table);

	return true;
}

fn idct_core(block: &mut [i32; 64], scale_table: &[i16; 64]) {
	let mut buf: [i32; 64] = [0; 64];

	for _ in 0..2 {
		for x in 0..8 {
			for y in 0..8 {
				let mut sum = 0;

				for z in 0..8 {
					sum += block[y + z * 8] as i32 * (scale_table[x + z * 8] / 8) as i32;
				}

				buf[x + y * 8] = (sum + 0xFFF) / 0x2000;
			}
		}
		mem::swap(&mut buf, block);
	}
}

fn i10(value: u16) -> i32 {
	(((value as i16) << 6) >> 6) as i32
}
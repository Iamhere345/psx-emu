use log::*;

#[derive(Default, Clone, Copy)]
struct Voice {
	key_on: bool,

	current_addr: usize,
	start_addr: usize,
	repeat_addr: usize,

	sample_rate: u16,
	pitch_counter: u16,
	decode_buf_index: usize,
	current_sample: i16,

	decode_buf: [i16; 28],
	old_sample: i16,
	older_sample: i16,
}

impl Voice {
	fn tick(&mut self, sram: &[u8]) {
		// TODO pitch modulation
		self.pitch_counter += self.sample_rate.min(0x4000);

		// every 0x1000 steps (44100hz) increment index of sample to play
		// i.e Counter.Bit12 and up indicates the current sample (within a ADPCM block).
		while self.pitch_counter >= 0x1000 {
			self.pitch_counter -= 0x1000;
			self.decode_buf_index += 1;

			// decode new block if the end of the current block is reached
			if self.decode_buf_index == 28 {
				self.decode_buf_index = 0;
				self.decode_next_block(sram);
			}
		}

		// update current sample
		// TODO sample interpolation
		self.current_sample = self.decode_buf[self.decode_buf_index];
	}

	fn key_on(&mut self, sram: &[u8]) {
		// TODO reset envelope
		self.current_addr = self.start_addr;
		self.pitch_counter = 0;
		self.decode_buf_index = 0;

		self.decode_next_block(sram);

		self.key_on = true;
	}

	fn key_off(&mut self) {
		// TODO
		self.key_on =  false;
	}

	fn decode_next_block(&mut self, sram: &[u8]) {
		let block = &sram[self.current_addr..self.current_addr + 16];

		// decode shift/filter from header
		// shift can be 0-12; >12 = 9
		let shift = block[0] & 0xF;
		let shift = if shift > 12 { 9 } else { shift };

		// 0-4 different filter values
		let filter = ((block[0] >> 4) & 0x7).min(4);

		for sample_i in 0..28 {
			let sample_byte = block[2 + sample_i / 2];
			let sample_nibble = (sample_byte >> (4 * sample_i % 2)) & 0xF;

			// sign-extend to i32
			let raw_sample = (((sample_nibble as i8) << 4) >> 4) as i32;
			// apply shift from header (calulated as 12 - shift)
			let shifted_sample = raw_sample << (12 - shift);

			let old = self.old_sample as i32;
			let older = self.older_sample as i32;

			let filtered_sample = match filter {
				// no filter
				0 => shifted_sample,
				// filter using old sample
				1 => shifted_sample + (60 * old + 32) / 64,
				// filter using old and older sample
				2 => shifted_sample + (115 * old - 52 * older + 32) / 64,
    			3 => shifted_sample + (98 * old - 55 * older + 32) / 64,
				4 => shifted_sample + (122 * old - 60 * older + 32) / 64,

				_ => unreachable!(),
			};

			let clamped_sample = filtered_sample.clamp(-0x8000, 0x7FFF) as i16;
			self.decode_buf[sample_i] = clamped_sample;

			// update old and older samples
			self.older_sample = self.old_sample;
			self.old_sample = clamped_sample;
		}

		// handle loop flags
		let loop_end 		= (block[1] >> 0) & 1 != 0;
		let loop_repeat	= (block[1] >> 1) & 1 != 0;
		let loop_start	= (block[1] >> 2) & 1 != 0;

		if loop_start {
			self.repeat_addr = self.current_addr;
		}

		if loop_end {
			self.current_addr = self.repeat_addr;

			if !loop_repeat {
				// TODO mute when loop repeat = 0
			}
		} else {
			self.current_addr += 16;
		}

	}

	fn read(&self, addr: u32) -> u16 {
		match addr & 0xF {
			// Volume L
			0x0 => 0,
			// Volume R
			0x2 => 0,
			// ADPCM Sample Rate
			0x4 => self.sample_rate,
			// ADPCM Start Address
			0x6 => self.start_addr as u16,
			// ADSR low
			0x8 => 0,
			// ADSR high
			0xA => 0,
			// ADSR current volume
			0xC => 0,
			// ADPCM Repeat Address
			0xE => self.repeat_addr as u16,
			_ => unimplemented!("SPU Voice read 0x{:X}", addr & 0xF),
		}
	}

	fn write(&mut self, addr: u32, write: u16) {
		match addr & 0xF {
			// Volume L
			0x0 => {},
			// Volume R
			0x2 => {},
			// ADPCM Sample Rate
			0x4 => self.sample_rate = write,
			// ADPCM Start Address
			0x6 => self.start_addr = write as usize,
			// ADSR low
			0x8 => {},
			// ADSR high
			0xA => {},
			// ADSR current volume
			0xC => {},
			// ADPCM Repeat Address
			0xE => self.repeat_addr = write as usize,
			_ => unimplemented!("[0x{:X}] SPU voice write 0x{write:X}", addr & 0xF),
		}
	}
}

struct SpuControlRegister {
	spu_enable: bool,			// doesnt apply to CD audio
	mute_spu: bool,				// doesnt apply to CD audio
	noise_freq_shift: u8,		// 0..0Fh = low-high frequency
	noise_freq_step: u8,		// 0..03h = Step "4,5,6,7"
	reverb_master_enable: bool,
	irq_enable: bool,			// 0=Disabled/Acknowledge, 1=Enabled; only when Bit15=1
	sram_transfer_mode: u8,		// 0=Stop, 1=ManualWrite, 2=DMAwrite, 3=DMAread
	ext_audio_reverb: bool,
	cd_audio_reverb: bool,
	ext_audio_enable: bool,
	cd_audio_enable: bool,
}

impl SpuControlRegister {
	fn new() -> Self {
		Self {
			spu_enable: false,
			mute_spu: true,
			noise_freq_shift: 0,
			noise_freq_step: 0,
			reverb_master_enable: false,
			irq_enable: false,
			sram_transfer_mode: 0,
			ext_audio_reverb: false,
			cd_audio_reverb: false,
			ext_audio_enable: false,
			cd_audio_enable: false,
		}
	}

	fn read(&self) -> u16 {
		u16::from(self.cd_audio_enable)
			| (u16::from(self.ext_audio_enable) << 1)
			| (u16::from(self.cd_audio_reverb) << 2)
			| (u16::from(self.ext_audio_reverb) << 3)
			| (u16::from(self.sram_transfer_mode) << 5)
			| (u16::from(self.irq_enable) << 6)
			| (u16::from(self.reverb_master_enable) << 7)
			| (u16::from(self.noise_freq_step) << 9)
			| (u16::from(self.noise_freq_shift) << 13)
			| (u16::from(self.mute_spu) << 14)
			| (u16::from(self.spu_enable) << 15)
	}

	fn write(&mut self, write: u16) {
		self.cd_audio_enable = write & 1 != 0;
		self.ext_audio_enable = (write >> 1) & 1 != 0;
		self.cd_audio_reverb = (write >> 2) & 1 != 0;
		self.ext_audio_reverb = (write >> 3) & 1 != 0;

		self.sram_transfer_mode = ((write >> 5) & 3) as u8;

		self.irq_enable = (write >> 6) & 1 != 0;
		self.reverb_master_enable = (write >> 7) & 1 != 0;

		self.noise_freq_step = ((write >> 9) & 3) as u8;
		self.noise_freq_shift = ((write >> 13) & 0xF) as u8;

		self.mute_spu = (write >> 14) & 1 != 0;
		self.spu_enable = (write >> 15) & 1 != 0;
	}

}

// stubbed for now
pub struct Spu {
	control: SpuControlRegister,
	voices: [Voice; 24],

	sram: Vec<u8>, 
	start_sram_addr: u16,
	current_sram_addr: usize
}

impl Spu {
	pub fn new() -> Self {
		Self {
			control: SpuControlRegister::new(),
			voices: [Voice::default(); 24],

			sram: vec![0; 512 * 1024], // 512K of sound ram
			start_sram_addr: 0,
			current_sram_addr: 0,
		}
	}

	pub fn tick(&mut self) -> i16 {
		// update all voices
		for voice in &mut self.voices {
			voice.tick(&self.sram);
		}

		let mut mixed_sample: i32 = 0;

		for voice in &self.voices {
			if !voice.key_on {
				continue;
			}

			mixed_sample += i32::from(voice.current_sample / 4);
		}

		mixed_sample.clamp(-0x8000, 0x7FFF) as i16
	}

	pub fn read16(&self, addr: u32) -> u16 {
		match addr {
			// voice regs
			0x1F801C00 		..= 0x1F801D7F => {
				let voice_num = (addr >> 4) & 0x1F;

				self.voices[voice_num as usize].read(addr)
			},
			// volume regs
			0x1F801D80	 	..= 0x1F801D87 => 0,
			// voice flags
			0x1F801D88		..= 0x1F801D9F => 0,
			// Sound RAM Data Transfer Address
			0x1F801DA6 => self.start_sram_addr,
			// Control Register (SPUCNT)
			0x1F801DAA => self.control.read(),
			// Sound RAM Data Transfer Control (should be 0004h)
			0x1F801DAC => 4,
			// Status Register (SPUSTAT)
			0x1F801DAE => self.read_stat(),
			// unused?
			0x1F801E80 		..= 0x1F801FFF => 0,

			_ => { warn!("[0x{addr:08X}] Unknown SPU register read"); 0}
		}
	}

	pub fn read32(&self, addr: u32) -> u32 {
		(u32::from(self.read16(addr)) << 16) | u32::from(self.read16(addr + 2))
	}

	pub fn write16(&mut self, addr: u32, write: u16) {
		match addr {
			// voice regs
			0x1F801C00 		..= 0x1F801D7F => {
				let voice_num = (addr >> 4) & 0x1F;

				self.voices[voice_num as usize].write(addr, write);
			},
			// volume regs
			0x1F801D80	 	..= 0x1F801D87 => {},
			0x1F801DB0		..= 0x1F801DB4 => {},
			// voice flags
			0x1F801D88 => self.write_keyon(write, false),
			0x1F801D8A => self.write_keyon(write, true),
			0x1F801D8C => self.write_keyoff(write, false),
			0x1F801D8E => self.write_keyoff(write, true),
			0x1F801D90		..= 0x1F801D9F => {},
			// Sound RAM Data Transfer Address
			0x1F801DA6 => {
				self.start_sram_addr = write;
				self.current_sram_addr = self.start_sram_addr as usize;
			},
			// Control Register (SPUCNT)
			0x1F801DAA => self.control.write(write),
			// Sound RAM Data Transfer Fifo
			0x1F801DA8 => self.write_sram(write),
			// Sound RAM Data Transfer Control (stubbed)
			0x1F801DAC => {},
			// Status Register (SPUSTAT)
			0x1F801DAE => {}, // SPUSTAT is technically writeable but written bits are cleared shortly after being written
			// unused?
			0x1F801E80 		..= 0x1F801FFF => {},

			_ => warn!("[0x{addr:08X}] Unknown SPU register write 0x{write:X}")
		}
	}

	// not sure if i have this in the right order
	pub fn write32(&mut self, addr: u32, write: u32) {
		self.write16(addr, (write >> 16) as u16);
		self.write16(addr + 2, write as u16);
	}

	pub fn write_sram(&mut self, write: u16) {
		let bytes = u16::to_le_bytes(write);

		self.sram[self.current_sram_addr] = bytes[0];
		self.sram[self.current_sram_addr + 1] = bytes[1];

		self.current_sram_addr += 2;
	}
	
	fn write_keyon(&mut self, write: u16, is_high: bool) {
		let (start, end) = match is_high {
			true => (16, 23),
			false => (0, 16),
		};

		for voice in start..end {
			if (write >> (voice - start)) & 1 != 0 {
				self.voices[voice].key_on(&self.sram);
			}
		}
	}

	fn write_keyoff(&mut self, write: u16, is_high: bool) {
		let (start, end) = match is_high {
			true => (16, 23),
			false => (0, 16),
		};

		for voice in start..end {
			if (write >> (voice - start)) & 1 != 0 {
				self.voices[voice].key_off();
			}
		}
	}


	pub fn read_stat(&self) -> u16 {
		(self.control.read() & 0x3F)
			| (0 << 6) // IRQ flag
			// data transfer DMA read/write request
			| (u16::from(self.control.sram_transfer_mode & 2) << 7)
			| (0 << 8) // data transfer DMA write request
			| (0 << 9) // data transfer dma read request
			| (0 << 10) // data transfer busy flag
			| (0 << 11) // writing to first/second half of capture buffers
	}
}
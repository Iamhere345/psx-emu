use log::*;

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

	sram_transfer_addr: u16,
}

impl Spu {
	pub fn new() -> Self {
		Self {
			control: SpuControlRegister::new(),

			sram_transfer_addr: 0,
		}
	}

	pub fn read16(&self, addr: u32) -> u16 {
		match addr {
			// voice regs
			0x1F801C00 		..= 0x1F801D7F => 0,
			// volume regs
			0x1F801D80	 	..= 0x1F801D87 => 0,
			// voice flags
			0x1F801D88		..= 0x1F801D9F => 0,
			// Sound RAM Data Transfer Address
			0x1F801DA6 => self.sram_transfer_addr,
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
			0x1F801C00 		..= 0x1F801D7F => {},
			// volume regs
			0x1F801D80	 	..= 0x1F801D87 => {},
			0x1F801DB0		..= 0x1F801DB4 => {},
			// voice flags
			0x1F801D88		..= 0x1F801D9F => {},
			// Sound RAM Data Transfer Address
			0x1F801DA6 => self.sram_transfer_addr = write,
			// Control Register (SPUCNT)
			0x1F801DAA => self.control.write(write),
			// Sound RAM Data Transfer Fifo
			0x1F801DA8 => {},
			// Sound RAM Data Transfer Control
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
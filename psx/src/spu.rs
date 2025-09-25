use log::*;

// Table for 4-Point Gaussian Interpolation
const GAUSS_TABLE: &[i16; 512] = &[
    -0x001, -0x001, -0x001, -0x001, -0x001, -0x001, -0x001, -0x001, -0x001, -0x001, -0x001, -0x001,
    -0x001, -0x001, -0x001, -0x001, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0001,
    0x0001, 0x0001, 0x0001, 0x0002, 0x0002, 0x0002, 0x0003, 0x0003, 0x0003, 0x0004, 0x0004, 0x0005,
    0x0005, 0x0006, 0x0007, 0x0007, 0x0008, 0x0009, 0x0009, 0x000A, 0x000B, 0x000C, 0x000D, 0x000E,
    0x000F, 0x0010, 0x0011, 0x0012, 0x0013, 0x0015, 0x0016, 0x0018, 0x0019, 0x001B, 0x001C, 0x001E,
    0x0020, 0x0021, 0x0023, 0x0025, 0x0027, 0x0029, 0x002C, 0x002E, 0x0030, 0x0033, 0x0035, 0x0038,
    0x003A, 0x003D, 0x0040, 0x0043, 0x0046, 0x0049, 0x004D, 0x0050, 0x0054, 0x0057, 0x005B, 0x005F,
    0x0063, 0x0067, 0x006B, 0x006F, 0x0074, 0x0078, 0x007D, 0x0082, 0x0087, 0x008C, 0x0091, 0x0096,
    0x009C, 0x00A1, 0x00A7, 0x00AD, 0x00B3, 0x00BA, 0x00C0, 0x00C7, 0x00CD, 0x00D4, 0x00DB, 0x00E3,
    0x00EA, 0x00F2, 0x00FA, 0x0101, 0x010A, 0x0112, 0x011B, 0x0123, 0x012C, 0x0135, 0x013F, 0x0148,
    0x0152, 0x015C, 0x0166, 0x0171, 0x017B, 0x0186, 0x0191, 0x019C, 0x01A8, 0x01B4, 0x01C0, 0x01CC,
    0x01D9, 0x01E5, 0x01F2, 0x0200, 0x020D, 0x021B, 0x0229, 0x0237, 0x0246, 0x0255, 0x0264, 0x0273,
    0x0283, 0x0293, 0x02A3, 0x02B4, 0x02C4, 0x02D6, 0x02E7, 0x02F9, 0x030B, 0x031D, 0x0330, 0x0343,
    0x0356, 0x036A, 0x037E, 0x0392, 0x03A7, 0x03BC, 0x03D1, 0x03E7, 0x03FC, 0x0413, 0x042A, 0x0441,
    0x0458, 0x0470, 0x0488, 0x04A0, 0x04B9, 0x04D2, 0x04EC, 0x0506, 0x0520, 0x053B, 0x0556, 0x0572,
    0x058E, 0x05AA, 0x05C7, 0x05E4, 0x0601, 0x061F, 0x063E, 0x065C, 0x067C, 0x069B, 0x06BB, 0x06DC,
    0x06FD, 0x071E, 0x0740, 0x0762, 0x0784, 0x07A7, 0x07CB, 0x07EF, 0x0813, 0x0838, 0x085D, 0x0883,
    0x08A9, 0x08D0, 0x08F7, 0x091E, 0x0946, 0x096F, 0x0998, 0x09C1, 0x09EB, 0x0A16, 0x0A40, 0x0A6C,
    0x0A98, 0x0AC4, 0x0AF1, 0x0B1E, 0x0B4C, 0x0B7A, 0x0BA9, 0x0BD8, 0x0C07, 0x0C38, 0x0C68, 0x0C99,
    0x0CCB, 0x0CFD, 0x0D30, 0x0D63, 0x0D97, 0x0DCB, 0x0E00, 0x0E35, 0x0E6B, 0x0EA1, 0x0ED7, 0x0F0F,
    0x0F46, 0x0F7F, 0x0FB7, 0x0FF1, 0x102A, 0x1065, 0x109F, 0x10DB, 0x1116, 0x1153, 0x118F, 0x11CD,
    0x120B, 0x1249, 0x1288, 0x12C7, 0x1307, 0x1347, 0x1388, 0x13C9, 0x140B, 0x144D, 0x1490, 0x14D4,
    0x1517, 0x155C, 0x15A0, 0x15E6, 0x162C, 0x1672, 0x16B9, 0x1700, 0x1747, 0x1790, 0x17D8, 0x1821,
    0x186B, 0x18B5, 0x1900, 0x194B, 0x1996, 0x19E2, 0x1A2E, 0x1A7B, 0x1AC8, 0x1B16, 0x1B64, 0x1BB3,
    0x1C02, 0x1C51, 0x1CA1, 0x1CF1, 0x1D42, 0x1D93, 0x1DE5, 0x1E37, 0x1E89, 0x1EDC, 0x1F2F, 0x1F82,
    0x1FD6, 0x202A, 0x207F, 0x20D4, 0x2129, 0x217F, 0x21D5, 0x222C, 0x2282, 0x22DA, 0x2331, 0x2389,
    0x23E1, 0x2439, 0x2492, 0x24EB, 0x2545, 0x259E, 0x25F8, 0x2653, 0x26AD, 0x2708, 0x2763, 0x27BE,
    0x281A, 0x2876, 0x28D2, 0x292E, 0x298B, 0x29E7, 0x2A44, 0x2AA1, 0x2AFF, 0x2B5C, 0x2BBA, 0x2C18,
    0x2C76, 0x2CD4, 0x2D33, 0x2D91, 0x2DF0, 0x2E4F, 0x2EAE, 0x2F0D, 0x2F6C, 0x2FCC, 0x302B, 0x308B,
    0x30EA, 0x314A, 0x31AA, 0x3209, 0x3269, 0x32C9, 0x3329, 0x3389, 0x33E9, 0x3449, 0x34A9, 0x3509,
    0x3569, 0x35C9, 0x3629, 0x3689, 0x36E8, 0x3748, 0x37A8, 0x3807, 0x3867, 0x38C6, 0x3926, 0x3985,
    0x39E4, 0x3A43, 0x3AA2, 0x3B00, 0x3B5F, 0x3BBD, 0x3C1B, 0x3C79, 0x3CD7, 0x3D35, 0x3D92, 0x3DEF,
    0x3E4C, 0x3EA9, 0x3F05, 0x3F62, 0x3FBD, 0x4019, 0x4074, 0x40D0, 0x412A, 0x4185, 0x41DF, 0x4239,
    0x4292, 0x42EB, 0x4344, 0x439C, 0x43F4, 0x444C, 0x44A3, 0x44FA, 0x4550, 0x45A6, 0x45FC, 0x4651,
    0x46A6, 0x46FA, 0x474E, 0x47A1, 0x47F4, 0x4846, 0x4898, 0x48E9, 0x493A, 0x498A, 0x49D9, 0x4A29,
    0x4A77, 0x4AC5, 0x4B13, 0x4B5F, 0x4BAC, 0x4BF7, 0x4C42, 0x4C8D, 0x4CD7, 0x4D20, 0x4D68, 0x4DB0,
    0x4DF7, 0x4E3E, 0x4E84, 0x4EC9, 0x4F0E, 0x4F52, 0x4F95, 0x4FD7, 0x5019, 0x505A, 0x509A, 0x50DA,
    0x5118, 0x5156, 0x5194, 0x51D0, 0x520C, 0x5247, 0x5281, 0x52BA, 0x52F3, 0x532A, 0x5361, 0x5397,
    0x53CC, 0x5401, 0x5434, 0x5467, 0x5499, 0x54CA, 0x54FA, 0x5529, 0x5558, 0x5585, 0x55B2, 0x55DE,
    0x5609, 0x5632, 0x565B, 0x5684, 0x56AB, 0x56D1, 0x56F6, 0x571B, 0x573E, 0x5761, 0x5782, 0x57A3,
    0x57C3, 0x57E2, 0x57FF, 0x581C, 0x5838, 0x5853, 0x586D, 0x5886, 0x589E, 0x58B5, 0x58CB, 0x58E0,
    0x58F4, 0x5907, 0x5919, 0x592A, 0x593A, 0x5949, 0x5958, 0x5965, 0x5971, 0x597C, 0x5986, 0x598F,
    0x5997, 0x599E, 0x59A4, 0x59A9, 0x59AD, 0x59B0, 0x59B2, 0x59B3,
];

const ADPCM_POS_FILTER: [i32; 5] = [0, 60, 115, 98, 122];
const ADPCM_NEG_FILTER: [i32; 5] = [0, 0, -52, -55, -60];

const SRAM_MASK: usize = (512 * 1024) - 1;
const ENVELOPE_COUNTER_MAX: u32 = 1 << (33 - 11);

#[derive(Clone, Copy, PartialEq, Eq)]
enum TransferMode {
	Stop = 0,
	ManualWrite = 1,
	DmaWrite = 2,
	DmaRead = 3,
}

impl TransferMode {
	fn from_bits(bits: u16) -> Self {
		match bits {
			0 => Self::Stop,
			1 => Self::ManualWrite,
			2 => Self::DmaWrite,
			3 => Self::DmaRead,

			_ => unreachable!(),
		}
	}
}

#[derive(Default, Clone, Copy, PartialEq, Eq)]
enum EnvelopeMode {
	#[default]
	Linear = 0,
	Exponential = 1
}

impl EnvelopeMode {
	fn from_bit(bit: bool) -> Self {
		match bit {
			false => Self::Linear,
			true => Self::Exponential
		}
	}
}

#[derive(Default, Clone, Copy, PartialEq, Eq)]
enum EnvelopeDir {
	#[default]
	Increase = 0,
	Decrease = 1,
}

impl EnvelopeDir {
	fn from_bit(bit: bool) -> Self {
		match bit {
			false => Self::Increase,
			true => Self::Decrease,
		}
	}
}

#[derive(Default, Clone, Copy, PartialEq, Eq)]
enum SweepPhase {
	#[default]
	Positive = 0,
	Negative = 1,
}

impl SweepPhase {
	fn from_bit(bit: bool) -> Self {
		match bit {
			true => Self::Negative,
			false => Self::Positive
		}
	}
}

#[derive(Default, Clone, Copy)]
struct SweepEnvelope {
	level: i16,
	counter: u32,
	envelope_enabled: bool,

	dir: EnvelopeDir,
	mode: EnvelopeMode,
	phase: SweepPhase,
	shift: u8,
	step: u8,
}

impl SweepEnvelope {
	fn tick(&mut self) {
		if !self.envelope_enabled {
			return;
		}

		let mut counter_dec = ENVELOPE_COUNTER_MAX >> self.shift.saturating_sub(11);

		if self.dir == EnvelopeDir::Increase && self.mode == EnvelopeMode::Exponential && self.level > 0x6000 {
			counter_dec >>= 2;
		}

		self.counter = self.counter.saturating_sub(counter_dec);
		if self.counter == 0 {
			self.counter = ENVELOPE_COUNTER_MAX;

			let mut step = i32::from(7 - self.step);
			if (self.dir == EnvelopeDir::Decrease) ^ (self.phase == SweepPhase::Negative) {
				step = !step;
			}

			step <<= 11_u8.saturating_sub(self.shift);

			if self.dir == EnvelopeDir::Decrease && self.mode == EnvelopeMode::Exponential {
				step = (step * i32::from(self.level)) >> 15;
			}

			let new_level = i32::from(self.level) + step;
			self.level = if self.dir != EnvelopeDir::Decrease {
				new_level.clamp(-0x8000, 0x7FFF) as i16
			} else if self.phase == SweepPhase::Negative {
				new_level.clamp(-0x8000, 0) as i16
			} else {
				new_level.clamp(0, 0x7FFF) as i16
			}
		}

	}

	fn read(&self) -> u16 {
		if self.envelope_enabled {
			(self.step as u16)
				| (self.shift as u16) << 2
				| (self.phase as u16) << 12
				| (self.dir as u16) << 13
				| (self.mode as u16) << 14
				| (self.envelope_enabled as u16) << 15
		} else {
			(self.level >> 1) as u16
		}
	}

	fn write(&mut self, write: u16) {
		self.envelope_enabled = (write >> 15) & 1 != 0;

		if self.envelope_enabled {
			self.step = (write & 3) as u8;
			self.shift = ((write >> 2) & 0x1F) as u8;
			self.phase = SweepPhase::from_bit((write >> 12) & 1 != 0);
			self.dir = EnvelopeDir::from_bit((write >> 13) & 1 != 0);
			self.mode = EnvelopeMode::from_bit((write >> 14) & 1 != 0);
		} else {
			self.level = (write << 1) as i16;
		}
	}
}

#[derive(Default, Clone, Copy, PartialEq, Eq)]
enum AdsrPhase {
	Attack,
	Decay,
	Sustain,
	#[default]
	Release
}

#[derive(Default, Clone, Copy)]
struct AdsrEnvelope {
	level: i16,
	phase: AdsrPhase,
	counter: u32,

	// attack settingss
	attack_dir: EnvelopeDir,
	attack_mode: EnvelopeMode,
	attack_shift: u8,
	attack_step: u8,

	// decay settings
	decay_dir: EnvelopeDir,
	decay_mode: EnvelopeMode,
	decay_shift: u8,
	decay_step: u8,

	// sustain settings
	sustain_dir: EnvelopeDir,
	sustain_mode: EnvelopeMode,
	sustain_shift: u8,
	sustain_step: u8,
	sustain_level: u8,

	// release settings
	release_dir: EnvelopeDir,
	release_mode: EnvelopeMode,
	release_shift: u8,
	release_step: u8,
}

impl AdsrEnvelope {
	fn new() -> Self {
		Self {
			level: 0,
			phase: AdsrPhase::Release,
			counter: 0,

			attack_dir: EnvelopeDir::Increase,		// fixed
			attack_mode: EnvelopeMode::Linear,		// configurable
			attack_shift: 11,						// configurable (0-31)
			attack_step: 0,							// configurable (0-3, interpreted as 7-N)
			
			decay_dir: EnvelopeDir::Decrease,		// fixed
			decay_mode: EnvelopeMode::Exponential,	// fixed
			decay_shift: 11,						// configurable (0-15)
			decay_step: 0,							// fixed (interpreted as -8)

			sustain_dir: EnvelopeDir::Decrease,		// configurable
			sustain_mode: EnvelopeMode::Linear,		// configurable
			sustain_shift: 11,						// configurable (0-31)
			sustain_step: 0,						// configurable (0-3, interpreted as 7-N or -(8-N) depending on direction)
			sustain_level: 0,

			release_dir: EnvelopeDir::Decrease,		// fixed
			release_mode: EnvelopeMode::Linear,		// configurable
			release_shift: 11,						// configurable (0-31)
			release_step: 0,						// fixed (interpreted as -8)
		}
	}

	fn key_on(&mut self) {
		self.level = 0;
		self.phase = AdsrPhase::Attack;
	}

	fn key_off(&mut self) {
		self.phase = AdsrPhase::Release;
	}

	fn tick(&mut self) {
		self.check_for_phase_transition();

		let (dir, mode, shift, step) = match self.phase {
			AdsrPhase::Attack => (self.attack_dir, self.attack_mode, self.attack_shift, self.attack_step),
			AdsrPhase::Decay => (self.decay_dir, self.decay_mode, self.decay_shift, self.decay_step),
			AdsrPhase::Sustain => (self.sustain_dir, self.sustain_mode, self.sustain_shift, self.sustain_step),
			AdsrPhase::Release => (self.release_dir, self.release_mode, self.release_shift, self.release_step),
		};

		let mut counter_dec = ENVELOPE_COUNTER_MAX >> shift.saturating_sub(11);

		if dir == EnvelopeDir::Increase && mode == EnvelopeMode::Exponential && self.level > 0x6000 {
			counter_dec >>= 2;
		}

		self.counter = self.counter.saturating_sub(counter_dec);
		if self.counter == 0 {
			self.counter = ENVELOPE_COUNTER_MAX;

			let mut step = i32::from(7 - step);
			if dir == EnvelopeDir::Decrease {
				step = !step;
			}

			step <<= 11_u8.saturating_sub(shift);

			if dir == EnvelopeDir::Decrease && mode == EnvelopeMode::Exponential {
				step = (step * i32::from(self.level)) >> 15;
			}

			self.level = (i32::from(self.level) + step).clamp(0, 0x7FFF) as i16;
		}

	}

	fn check_for_phase_transition(&mut self) {
		if self.phase == AdsrPhase::Attack && self.level == 0x7FFF {
			self.phase = AdsrPhase::Decay;
		}

		if self.phase == AdsrPhase::Decay && (self.level as u16) <= ((u16::from(self.sustain_level & 0xF) + 1) << 11) {
			self.phase = AdsrPhase::Sustain;
		}
	}

	fn read_low(&self) -> u16 {
		// sustain level?
		(self.sustain_level as u16)
			| (self.decay_shift as u16) << 4
			| (self.attack_step as u16) << 8
			| (self.attack_shift as u16) << 10
			| (self.attack_mode as u16) << 15
	}

	fn write_low(&mut self, write: u16) {
		self.sustain_level = (write & 0xF) as u8;
		self.decay_shift = ((write >> 4) & 0xF) as u8;
		self.attack_step = ((write >> 8) & 3) as u8;
		self.attack_shift = ((write >> 10) & 0x1F) as u8;
		self.attack_mode = EnvelopeMode::from_bit((write >> 15) & 1 != 0);
	}

	fn read_high(&self) -> u16 {
		(self.release_shift as u16) << 0
			| (self.release_mode as u16) << 5
			| (self.sustain_step as u16) << 6
			| (self.sustain_shift as u16) << 8
			| (self.sustain_dir as u16) << 14
			| (self.sustain_mode as u16) << 15
	}

	fn write_high(&mut self, write: u16) {
		self.release_shift = (write & 0x1F) as u8;
		self.release_mode = EnvelopeMode::from_bit((write >> 5) & 1 != 0);
		self.sustain_step = ((write >> 6) & 3) as u8;
		self.sustain_shift = ((write >> 8) & 0x1F) as u8;
		self.sustain_dir = EnvelopeDir::from_bit((write >> 14) & 1 != 0);
		self.sustain_mode = EnvelopeMode::from_bit((write >> 15) & 1 != 0);
	}
}

#[derive(Default, Clone, Copy)]
struct Voice {
	adsr: AdsrEnvelope,

	end_x: bool,

	current_addr: usize,
	start_addr: usize,
	repeat_addr: usize,

	sample_rate: u16,
	pitch_counter: u16,
	decode_buf_index: usize,
	current_sample: (i16, i16),

	// 28 samples
	decode_buf: [i16; 28],
	// 4 last decoded samples for interpolation
	old_samples: [i16; 4],

	old_sample: i16,
	older_sample: i16,

	volume_l: SweepEnvelope,
	volume_r: SweepEnvelope,
}

impl Voice {
	fn new() -> Self {
		Self {
			adsr: AdsrEnvelope::new(),

			decode_buf_index: 0,

			..Default::default()
		}
	}

	fn tick(&mut self, sram: &[u8]) {
		// TODO pitch modulation
		self.adsr.tick();

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

			self.old_samples[3] = self.old_samples[2];
			self.old_samples[2] = self.old_samples[1];
			self.old_samples[1] = self.old_samples[0];
			self.old_samples[0] = self.decode_buf[self.decode_buf_index];
		}

		// gaussian interpolation
		// index into gauss table uses bits 4-11 of pitch ounter
		let interp_index = ((self.pitch_counter >> 4) & 0xFF) as usize;

		let mut interp_value = apply_volume(GAUSS_TABLE[0xFF - interp_index], self.old_samples[3]);
		interp_value += apply_volume(GAUSS_TABLE[0x1FF - interp_index], self.old_samples[2]);
		interp_value += apply_volume(GAUSS_TABLE[0x100 + interp_index], self.old_samples[1]);
		interp_value += apply_volume(GAUSS_TABLE[interp_index], self.old_samples[0]);

		self.current_sample = self.apply_volume(interp_value);

	}

	fn apply_volume(&mut self, sample: i16) -> (i16, i16) {
		let adsr_sample = apply_volume(sample, self.adsr.level);

		self.volume_l.tick();
		self.volume_r.tick();

		// TODO volume envelope
		(apply_volume(adsr_sample, self.volume_l.level), apply_volume(adsr_sample, self.volume_r.level))
	}

	fn key_on(&mut self, sram: &[u8]) {
		self.adsr.key_on();

		self.end_x = false;

		trace!("keyon - start: 0x{:X} repeat 0x{:X}", self.start_addr, self.repeat_addr);

		self.current_addr = self.start_addr;
		self.pitch_counter = 0;
		self.decode_buf_index = 3;

		self.decode_next_block(sram);
	}

	fn key_off(&mut self) {
		self.adsr.key_off();
	}

	fn decode_next_block(&mut self, sram: &[u8]) {
		let block = &sram[self.current_addr..self.current_addr + 16];

		// decode shift/filter from header
		// shift can be 0-12; >12 = 9
		let shift = block[0] & 0xF;
		let shift = if shift > 12 { 9 } else { shift };

		// 0-4 different filter values
		let filter = ((block[0] >> 4) & 0x7).min(4);

		let filter_0 = ADPCM_POS_FILTER[filter as usize];
		let filter_1 = ADPCM_NEG_FILTER[filter as usize];

		for sample_i in 0..28 {
			let sample_byte = block[2 + sample_i / 2];
			let sample_nibble = (sample_byte >> (4 * (sample_i % 2))) & 0xF;

			// sign-extend to i32
			let raw_sample = (((sample_nibble as i8) << 4) >> 4) as i32;
			// apply shift from header (calulated as 12 - shift)
			let shifted_sample = raw_sample << (12 - shift);

			let old = self.old_sample as i32;
			let older = self.older_sample as i32;

			let filtered_sample = shifted_sample + (filter_0 * old + filter_1 * older + 32) / 64;

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
			self.end_x = true;

			if !loop_repeat {
				self.key_off();
				self.adsr.level = 0;
			}
		} else {
			self.current_addr = (self.current_addr + 16) & SRAM_MASK;
		}

	}

	fn read(&self, addr: u32) -> u16 {
		match addr & 0xF {
			// Volume L
			0x0 => self.volume_l.read(),
			// Volume R
			0x2 => self.volume_r.read(),
			// ADPCM Sample Rate
			0x4 => self.sample_rate,
			// ADPCM Start Address
			0x6 => (self.start_addr >> 3) as u16,
			// ADSR low
			0x8 => self.adsr.read_low(),
			// ADSR high
			0xA => self.adsr.read_high(),
			// ADSR current volume
			0xC => self.adsr.level as u16,
			// ADPCM Repeat Address
			0xE => (self.repeat_addr >> 3) as u16,
			_ => unimplemented!("SPU Voice read 0x{:X}", addr & 0xF),
		}
	}

	fn write(&mut self, addr: u32, write: u16) {
		match addr & 0xF {
			// Volume L
			0x0 => self.volume_l.write(write),
			// Volume R
			0x2 => self.volume_r.write(write),
			// ADPCM Sample Rate
			0x4 => self.sample_rate = write,
			// ADPCM Start Address
			0x6 => self.start_addr = (write as usize) << 3,
			// ADSR low
			0x8 => self.adsr.write_low(write),
			// ADSR high
			0xA => self.adsr.write_high(write),
			// ADSR current volume
			0xC => self.adsr.level = write as i16,
			// ADPCM Repeat Address
			0xE => self.repeat_addr = (write as usize) << 3,
			_ => unimplemented!("[0x{:X}] SPU voice write 0x{write:X}", addr & 0xF),
		}
	}
}

struct SpuControlRegister {
	spu_enable: bool,					// doesnt apply to CD audio
	mute_spu: bool,						// doesnt apply to CD audio
	noise_freq_shift: u8,				// 0..0Fh = low-high frequency
	noise_freq_step: u8,				// 0..03h = Step "4,5,6,7"
	reverb_master_enable: bool,
	irq_enable: bool,					// 0=Disabled/Acknowledge, 1=Enabled; only when Bit15=1
	transfer_mode: TransferMode,	// 0=Stop, 1=ManualWrite, 2=DMAwrite, 3=DMAread
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
			transfer_mode: TransferMode::Stop,
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
			| (u16::from(self.transfer_mode as u16) << 4)
			| (u16::from(self.irq_enable) << 6)
			| (u16::from(self.reverb_master_enable) << 7)
			| (u16::from(self.noise_freq_step) << 8)
			| (u16::from(self.noise_freq_shift) << 10)
			| (u16::from(self.mute_spu) << 14)
			| (u16::from(self.spu_enable) << 15)
	}

	fn write(&mut self, write: u16) {
		self.cd_audio_enable = write & 1 != 0;
		self.ext_audio_enable = (write >> 1) & 1 != 0;
		self.cd_audio_reverb = (write >> 2) & 1 != 0;
		self.ext_audio_reverb = (write >> 3) & 1 != 0;

		self.transfer_mode = TransferMode::from_bits((write >> 4) & 3);

		self.irq_enable = (write >> 6) & 1 != 0;
		self.reverb_master_enable = (write >> 7) & 1 != 0;

		self.noise_freq_step = ((write >> 8) & 3) as u8;
		self.noise_freq_shift = ((write >> 10) & 0xF) as u8;

		self.mute_spu = (write >> 14) & 1 != 0;
		self.spu_enable = (write >> 15) & 1 != 0;
	}

}

// stubbed for now
pub struct Spu {
	control: SpuControlRegister,
	voices: [Voice; 24],

	transfer_control: u16,

	sram: Vec<u8>, 
	start_sram_addr: u16,
	current_sram_addr: usize,

	volume_l: SweepEnvelope,
	volume_r: SweepEnvelope,

	pub emu_mute: bool,
}

impl Spu {
	pub fn new() -> Self {
		Self {
			control: SpuControlRegister::new(),
			voices: [Voice::new(); 24],

			transfer_control: 0x4, // should always be 0x4

			sram: vec![0; 512 * 1024], // 512K of sound ram
			start_sram_addr: 0,
			current_sram_addr: 0,

			volume_l: SweepEnvelope::default(),
			volume_r: SweepEnvelope::default(),

			emu_mute: false,
		}
	}

	pub fn tick(&mut self) -> (i16, i16) {
		// update all voices
		for voice in &mut self.voices {
			voice.tick(&self.sram);
		}

		// update sweep envelopes
		self.volume_l.tick();
		self.volume_r.tick();

		let mut mixed_l: i32 = 0;
		let mut mixed_r: i32 = 0;

		for voice in &self.voices {
			let (sample_l, sample_r) = voice.current_sample;

			mixed_l += i32::from(sample_l);
			mixed_r += i32::from(sample_r);
		}

		if !self.emu_mute {
			let clamped_l = mixed_l.clamp(-0x8000, 0x7FFF) as i16;
			let clamped_r = mixed_r.clamp(-0x8000, 0x7FFF) as i16;

			(apply_volume(clamped_l, self.volume_l.level), apply_volume(clamped_r, self.volume_r.level))
		} else {
			(0, 0)
		}
	}

	pub fn read16(&self, addr: u32) -> u16 {
		match addr {
			// voice regs
			0x1F801C00 		..= 0x1F801D7F => {
				let voice_num = (addr >> 4) & 0x1F;

				self.voices[voice_num as usize].read(addr)
			},
			// volume regs
			0x1F801D80 => self.volume_l.read(),
			0x1F801D82 => self.volume_r.read(),
			0x1F801D80	 	..= 0x1F801D87 => 0,
			// voice flags
			0x1F801D9C => self.read_endx(false),
			0x1F801D9E => self.read_endx(true),
			0x1F801D88		..= 0x1F801D9F => 0,
			// Sound RAM Data Transfer Address
			0x1F801DA6 => self.start_sram_addr,
			// Control Register (SPUCNT)
			0x1F801DAA => self.control.read(),
			// Sound RAM Data Transfer Control (should be 0004h)
			0x1F801DAC => self.transfer_control,
			// Status Register (SPUSTAT)
			0x1F801DAE => self.read_stat(),
			// unused?
			0x1F801E80 		..= 0x1F801FFF => 0,

			_ => { /* warn!("[0x{addr:08X}] Unknown SPU register read"); */ 0}
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

				trace!("[0x{addr:X}] write voice{voice_num} 0x{write:X}");

				self.voices[voice_num as usize].write(addr, write);
			},
			// volume regs
			0x1F801D80 => self.volume_l.write(write),
			0x1F801D82 => self.volume_r.write(write),
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
				self.current_sram_addr = (self.start_sram_addr as usize) << 3;
			},
			// Control Register (SPUCNT)
			0x1F801DAA => self.control.write(write),
			// Sound RAM Data Transfer Fifo
			0x1F801DA8 => self.write_sram(write),
			// Sound RAM Data Transfer Control
			0x1F801DAC => self.transfer_control = write,
			// Status Register (SPUSTAT)
			0x1F801DAE => {}, // SPUSTAT is technically writeable but written bits are cleared shortly after being written
			// unused?
			0x1F801E80 		..= 0x1F801FFF => {},

			_ => {}//warn!("[0x{addr:08X}] Unknown SPU register write 0x{write:X}")
		}
	}

	pub fn write32(&mut self, addr: u32, write: u32) {
		self.write16(addr, write as u16);
		self.write16(addr + 2, (write >> 16) as u16);
	}

	pub fn write_sram(&mut self, write: u16) {
		let bytes = u16::to_le_bytes(write);

		self.sram[self.current_sram_addr] = bytes[0];
		self.sram[self.current_sram_addr + 1] = bytes[1];

		self.current_sram_addr = (self.current_sram_addr + 2) & SRAM_MASK;
	}

	pub fn read_sram(&mut self) -> u16 {
		let low = self.sram[self.current_sram_addr];
		let high = self.sram[self.current_sram_addr + 1];

		self.current_sram_addr = (self.current_sram_addr + 2) & SRAM_MASK;

		u16::from_le_bytes([low, high])
	}

	fn read_endx(&self, is_high: bool) -> u16 {
		let mut result = 0;

		let (start, end) = match is_high {
			true => (16, 23),
			false => (0, 16),
		};

		for voice in start..end {
			result |= u16::from(self.voices[voice].end_x) << (voice - start);
		}

		result
	}
	
	fn write_keyon(&mut self, write: u16, is_high: bool) {
		let (start, end) = match is_high {
			true => (16, 23),
			false => (0, 16),
		};

		trace!("keyon {is_high} 0b{write:b}");

		for voice in start..end {
			if (write >> (voice - start)) & 1 != 0 {
				trace!("voice {voice} key on");
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
				trace!("voice {voice} key off");
				self.voices[voice].key_off();
			}
		}
	}


	pub fn read_stat(&self) -> u16 {
		(self.control.read() & 0x3F)
			| (0 << 6) // IRQ flag
			// data transfer DMA read/write request
			| ((self.control.transfer_mode as u16 & 2) << 7)
			| (u16::from(self.control.transfer_mode == TransferMode::DmaWrite) << 8) // data transfer DMA write request
			| (u16::from(self.control.transfer_mode == TransferMode::DmaRead) << 9) // data transfer dma read request
			| (0 << 10) // data transfer busy flag
			| (0 << 11) // writing to first/second half of capture buffers
	}
}

fn apply_volume(sample: i16, volume: i16) -> i16 {
	((i32::from(sample) * i32::from(volume)) >> 15) as i16
}
use log::*;

use crate::cdrom::disc::Sector;

const POS_XA_ADPCM_TABLE: [i32; 5] = [0, 60, 115, 98, 122];
const NEG_XA_ADPCM_TABLE: [i32; 5] = [0, 0, -52, -55, -60];

const ADPCM_BUF_CAPACITY:  usize = 18 * 8 * 28;
const OUTPUT_BUF_CAPACITY: usize = ADPCM_BUF_CAPACITY * 14 / 6;

const ZIGZAG_TABLE: [[i32; 29]; 7] = [
	[
		0x0000, 0x0000, 0x0000, 0x0000, 0x0000, -0x0002, 0x000A, -0x0022, 0x0041, -0x0054, 0x0034,
		0x0009, -0x010A, 0x0400, -0x0A78, 0x234C, 0x6794, -0x1780, 0x0BCD, -0x0623, 0x0350,
		-0x016D, 0x006B, 0x000A, -0x0010, 0x0011, -0x0008, 0x0003, -0x0001,
	],
	[
		0x0000, 0x0000, 0x0000, -0x0002, 0x0000, 0x0003, -0x0013, 0x003C, -0x004B, 0x00A2, -0x00E3,
		0x0132, -0x0043, -0x0267, 0x0C9D, 0x74BB, -0x11B4, 0x09B8, -0x05BF, 0x0372, -0x01A8,
		0x00A6, -0x001B, 0x0005, 0x0006, -0x0008, 0x0003, -0x0001, 0x0000,
	],
	[
		0x0000, 0x0000, -0x0001, 0x0003, -0x0002, -0x0005, 0x001F, -0x004A, 0x00B3, -0x0192,
		0x02B1, -0x039E, 0x04F8, -0x05A6, 0x7939, -0x05A6, 0x04F8, -0x039E, 0x02B1, -0x0192,
		0x00B3, -0x004A, 0x001F, -0x0005, -0x0002, 0x0003, -0x0001, 0x0000, 0x0000,
	],
	[
		0x0000, -0x0001, 0x0003, -0x0008, 0x0006, 0x0005, -0x001B, 0x00A6, -0x01A8, 0x0372,
		-0x05BF, 0x09B8, -0x11B4, 0x74BB, 0x0C9D, -0x0267, -0x0043, 0x0132, -0x00E3, 0x00A2,
		-0x004B, 0x003C, -0x001B, 0x0003, 0x0000, -0x0002, 0x0000, 0x0000, 0x0000,
	],
	[
		-0x0001, 0x0003, -0x0008, 0x0011, -0x0010, 0x000A, 0x006B, -0x016D, 0x0350, -0x0623,
		0x0BCD, -0x1780, 0x6794, 0x234C, -0x0A78, 0x0400, -0x010A, 0x0009, 0x0034, -0x0054, 0x0041,
		-0x0022, 0x000A, -0x0001, 0x0000, 0x0001, 0x0000, 0x0000, 0x0000,
	],
	[
		0x0002, -0x0008, 0x0010, -0x0023, 0x002B, 0x001A, -0x00EB, 0x027B, -0x0548, 0x0AFA,
		-0x16FA, 0x53E0, 0x3C07, -0x1249, 0x080E, -0x0347, 0x015B, -0x0044, -0x0017, 0x0046,
		-0x0023, 0x0011, -0x0005, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000,
	],
	[
		-0x0005, 0x0011, -0x0023, 0x0046, -0x0017, -0x0044, 0x015B, -0x0347, 0x080E, -0x1249,
		0x3C07, 0x53E0, -0x16FA, 0x0AFA, -0x0548, 0x027B, -0x00EB, 0x001A, 0x002B, -0x0023, 0x0010,
		-0x0008, 0x0002, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000, 0x0000,
	],
];

pub struct XaAdpcmState {
	output_l: Vec<i16>,
	output_r: Vec<i16>,
	output_index: usize,

	adpcm_samples_l: Vec<i16>,
	adpcm_samples_r: Vec<i16>,
	prev_samples_l: [i16; 2],
	prev_samples_r: [i16; 2],

	// L, R
	ringbuf: [[i16; 32]; 2],
	ringbuf_index: [usize; 2],
}
impl XaAdpcmState {
	pub fn new() -> Self {
		Self {
			output_l: Vec::with_capacity(OUTPUT_BUF_CAPACITY),
			output_r: Vec::with_capacity(OUTPUT_BUF_CAPACITY),
			output_index: 0,

			adpcm_samples_l: Vec::with_capacity(ADPCM_BUF_CAPACITY),
			adpcm_samples_r: Vec::with_capacity(ADPCM_BUF_CAPACITY),
			prev_samples_l: [0; 2],
			prev_samples_r: [0; 2],

			ringbuf: [[0; 32]; 2],
			ringbuf_index: [0; 2],
		}
	}

	pub fn get_sample(&mut self) -> Option<(i16, i16)> {
		//debug!("new sample index: {} len: {}", self.output_index + 1, self.output_l.len());

		if self.output_index >= self.output_l.len() {
			return None;
		}

		let sample_l = self.output_l[self.output_index];
		let sample_r = self.output_r[self.output_index];
		self.output_index += 1;

		Some((sample_l, sample_r))
	}

	pub fn decode_xa_sector(&mut self, sector: &Sector) {
		self.output_l.clear();
		self.output_r.clear();
		self.adpcm_samples_l.clear();
		self.adpcm_samples_r.clear();
		self.output_index = 0;

		let coding_info = sector.audio_sector().get(0x13).unwrap();
		let is_stereo = if (coding_info & 3) == 1 { true } else { false };
		let is_18900hz = if ((coding_info >> 2) & 1) == 0 { false } else { true };

		for data_block in sector.xa_audio().chunks_exact(128) {
			for audio_block in 0..4 {
				if is_stereo {
					Self::deocde_block(
						data_block,
						audio_block,
						0,
						&mut self.adpcm_samples_l,
						&mut self.prev_samples_l
					);
					Self::deocde_block(
						data_block,
						audio_block,
						1,
						&mut self.adpcm_samples_r,
						&mut self.prev_samples_r
					);
				} else {
					Self::deocde_block(
						data_block,
						audio_block,
						0, 
						&mut self.adpcm_samples_l,
						&mut self.prev_samples_l
					);
					Self::deocde_block(
						data_block,
						audio_block,
						1,
						&mut self.adpcm_samples_l,
						&mut self.prev_samples_l
					);
				}
			}
		}

		// stero/mono both need to resample left channel
		resample_to_44100hz(&mut self.ringbuf[0], &mut self.ringbuf_index[0], &self.adpcm_samples_l, is_18900hz, &mut self.output_l);
		
		if is_stereo {
			resample_to_44100hz(&mut self.ringbuf[1], &mut self.ringbuf_index[1], &self.adpcm_samples_r, is_18900hz, &mut self.output_r);
		} else {
			resample_to_44100hz(&mut self.ringbuf[1], &mut self.ringbuf_index[1], &self.adpcm_samples_l, is_18900hz, &mut self.output_r);
		}
	}

	fn deocde_block(data_block: &[u8], audio_block_index: usize, nibble: usize, out_buf: &mut Vec<i16>, prev_samples: &mut [i16; 2]) {
		let header = data_block[4 + audio_block_index * 2 + nibble];
		let shift = if (header & 0xF) > 12 { 9 } else { 12 - (header & 0xF) };
		let filter = (header >> 4) & 3;

		let filter_0 = POS_XA_ADPCM_TABLE[filter as usize];
		let filter_1 = NEG_XA_ADPCM_TABLE[filter as usize];

		for i in 0..28 {
			let sample_byte = (data_block[16 + audio_block_index + i * 4] >> (nibble * 4)) & 0xF;
			let sample_halfword = (((sample_byte as i8) << 4) >> 4) as i16;
			let filtered_sample = (i32::from(sample_halfword << shift) + (((i32::from(prev_samples[0]) * filter_0) + (i32::from(prev_samples[1]) * filter_1) + 32) / 64)).clamp(-0x8000, 0x7FFF) as i16;

			prev_samples[1] = prev_samples[0];
			prev_samples[0] = filtered_sample;
			out_buf.push(filtered_sample);
		}
	}

}

fn resample_to_44100hz(ringbuf: &mut [i16; 32], ringbuf_index: &mut usize, samples: &Vec<i16>, is_18900hz: bool, output: &mut Vec<i16>) {
	let pushes_per_sample = if is_18900hz { 2 } else { 1 };

	let mut six_step = 6;

	for sample in samples {
		for _ in 0..pushes_per_sample {
			ringbuf[*ringbuf_index & 0x1F] = *sample;
			*ringbuf_index = ringbuf_index.wrapping_add(1);
			six_step -= 1;

			if six_step == 0 {
				six_step = 6;

				for i in 0..7 {
					output.push(zigzag_interpolate(ringbuf, ringbuf_index, i));
				}
			}
		}
	}
}

fn zigzag_interpolate(ringbuf: &mut [i16; 32], ringbuf_index: &mut usize, table: usize) -> i16 {
	let mut sum: i32 = 0;

	for i in 0..29 {
		sum += (i32::from(ringbuf[ringbuf_index.wrapping_sub(i) & 0x1F]) * ZIGZAG_TABLE[table][i]) / 0x8000
	}

	sum.clamp(-0x8000, 0x7FFF) as i16
}

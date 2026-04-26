use crate::cdrom::disc::Sector;

const POS_XA_ADPCM_TABLE: [i16; 5] = [0, 60, 115, 98, 122];
const NEG_XA_ADPCM_TABLE: [i16; 5] = [0, 0, -52, -55, -60];

pub struct XaAdpcmState {
	output_l: Vec<i16>,
	output_r: Vec<i16>,

	prev_samples_l: [i16; 2],
	prev_samples_r: [i16; 2],
}
impl XaAdpcmState {
	pub fn decode_xa_sector(&mut self, sector: &Sector) {
		let coding_info = sector.audio_sector().get(0x13).unwrap();
		let is_stereo = if (coding_info & 3) == 1 { true } else { false };
		let is_37800hz = if ((coding_info >> 2) & 1) == 0 { true } else { false };

		for data_block in sector.audio_sector().chunks_exact(128) {
			for audio_block in 0..4 {
				if is_stereo {
					Self::deocde_block(
						data_block, 
						audio_block, 
						0, 
						&mut self.output_l, 
						&mut self.prev_samples_l
					);
					Self::deocde_block(
						data_block, 
						audio_block, 
						1, 
						&mut self.output_r, 
						&mut self.prev_samples_r
					);
				} else {
					Self::deocde_block(
						data_block, 
						audio_block, 
						0, 
						&mut self.output_l, 
						&mut self.prev_samples_l
					);
					Self::deocde_block(
						data_block, 
						audio_block, 
						1, 
						&mut self.output_l, 
						&mut self.prev_samples_l
					);
				}
			}
		}
	}

	fn deocde_block(data_block: &[u8], audio_block_index: usize, nibble: usize, out_buf: &mut Vec<i16>, prev_samples: &mut [i16; 2]) {
		let shift = 12 - (data_block[4 + audio_block_index * 2 + nibble] & 0xF);
		let filter = (data_block[4 + audio_block_index * 2 + nibble] & 0x30) >> 4;

		let filter_0 = POS_XA_ADPCM_TABLE[filter as usize];
		let filter_1 = NEG_XA_ADPCM_TABLE[filter as usize];

		for nibble in 0..28 {
			let sample_byte = (data_block[16 + audio_block_index + nibble * 4] << (nibble * 4) & 0xF);
			let sample_halfword = (i16::from(sample_byte) << 12) >> 12;
			let filtered_sample = (sample_halfword << shift) + ((prev_samples[0] * filter_0 + prev_samples[1] * filter_1 + 32) / 64).clamp(-0x8000, 0x7FFF);

			prev_samples[1] = prev_samples[0];
			prev_samples[0] = filtered_sample;
			out_buf.push(filtered_sample);
		}
	}
}
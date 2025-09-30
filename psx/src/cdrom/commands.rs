use crate::cdrom::disc::*;

use super::*;

pub const AVG_CYCLES: u64 = 0xC4E1;
pub const DELAY_1MS: u64 = 0x844D;

pub const READ_CYCLES: [u64; 2] = [0x6E1CD, 0x36CD2]; // single speed, double speed
//pub const READ_CYCLES: [u64; 2] = [0x100, 0x200];
pub const PAUSE_CYCLES: [u64; 2] = [0x21181C, 0x10BD93];

const ERROR_INVALID_SUBCMD: u8 = 0x10;
const ERROR_INVALID_PARAMS: u8 = 0x20;
const ERROR_INVALID_CMD: 	u8 = 0x40;
const ERROR_CANNOT_RESPOND:	u8 = 0x80;	// also if the disc is not inserted at all

impl Cdrom {
	pub fn nop(&self) -> (CmdResponse, u64) {
		debug!("Nop");

		(CmdResponse::int3_status(self), AVG_CYCLES)
	}

	pub fn test(&mut self) -> (CmdResponse, u64) {
		if let Some(sub_cmd) = self.params_fifo.pop_front() {
			return match sub_cmd {
				0x20 => {
					(CmdResponse {
						int_level: 3,
						result: vec![0x94, 0x09, 0x19, 0xC0],

						second_response: None,
						on_complete: None,
					}, AVG_CYCLES)
				},
				_ => todo!("subcommand 0x{sub_cmd:X}")
			}
		} else {
			return (CmdResponse::error(self, ERROR_INVALID_PARAMS), AVG_CYCLES);
		}
	}

	pub fn get_id(&self) -> (CmdResponse, u64) {
		debug!("GetID");

		let mut first_response = CmdResponse::int3_status(self);

		let stat = self.get_stat();
		let mut flags = 0;
		let disk_type = 0x20;
		let atip = 0;

		let mut second_response = CmdResponse {
			int_level: 2,
			result: vec![stat, flags, disk_type, atip, b'S', b'C', b'E', b'A'],

			second_response: None,
			on_complete: None,
		};

		if self.disc.is_none() {
			second_response.int_level = 5;
			second_response.result[1] |= (1 << 6);
		}

		first_response.second_response = Some((Box::new(second_response), 0x4A00));

		(first_response, AVG_CYCLES)
	}

	pub fn get_tn(&self) -> (CmdResponse, u64) {
		let (first, last) = match &self.disc {
			Some(disc) => (1u8, disc.tracks.len() as u8),
			None => (1, 1)
		};

		debug!("GetTN ({first}, {last})");

		(CmdResponse {
			int_level: 3,
			result: vec![self.get_stat(), first, last],
			second_response: None,
			on_complete: None
		}, AVG_CYCLES)
	}

	// TODO
	pub fn get_td(&mut self) -> (CmdResponse, u64) {
		if self.params_fifo.len() < 1 {
			return (CmdResponse::error(&self, ERROR_INVALID_PARAMS), AVG_CYCLES);
		}

		let track = bcd_to_binary(self.params_fifo.pop_front().unwrap());
		self.params_fifo.clear();

		debug!("GetTD {track}");

		if let Some(disc) = &self.disc {
			if track as usize > disc.tracks.len() {
				debug!("GetTD error {track} >= {}", disc.tracks.len());
				return (CmdResponse::error(&self, ERROR_INVALID_SUBCMD), AVG_CYCLES);
			}

			let track_index = if track == 0 {
				disc.get_disc_end()
			} else {
				disc.get_track_start(track as usize)
			};

			debug!("track index: {}:{}", track_index.minutes, track_index.seconds);

			(CmdResponse {
				int_level: 3,
				result: vec![self.get_stat(), binary_to_bcd(track_index.minutes), binary_to_bcd(track_index.seconds)],
				second_response: None,
				on_complete: None
			}, AVG_CYCLES)
		} else {
			(CmdResponse::error(&self, ERROR_CANNOT_RESPOND), AVG_CYCLES)
		}
	}

	pub fn set_loc(&mut self) -> (CmdResponse, u64) {
		if self.params_fifo.len() < 3 {
			return (CmdResponse::error(&self, ERROR_INVALID_PARAMS), AVG_CYCLES);
		}

		let minutes = self.params_fifo.pop_front().unwrap();
		let seconds = self.params_fifo.pop_front().unwrap();
		let sectors = self.params_fifo.pop_front().unwrap();

		self.params_fifo.clear();

		self.seek_target = CdIndex::from_bcd(minutes, seconds, sectors);
		self.seek_complete = false;

		debug!("setloc {}", self.seek_target);

		(CmdResponse::int3_status(self), AVG_CYCLES)
	}

	pub fn get_loc_p(&mut self) -> (CmdResponse, u64) {
		if let Some(disc) = &self.disc {
			let current_sector = match self.drive_state {
				DriveState::Idle => self.current_seek,
				DriveState::Seek => self.seek_target,
				DriveState::Read => self.current_seek + self.read_offset,
				DriveState::Play => self.current_seek,
			};
	
			let (track, track_addr) = disc.get_track_number(current_sector.to_lba());

			let relative_time = CdIndex::from_lba(current_sector.to_lba() - track_addr);

			let index: u8 = 1;
			
			let mut response = CmdResponse::int3_status(&self);
			response.result = vec![
				track as u8,
				index,

				relative_time.minutes,
				relative_time.seconds,
				relative_time.sectors,

				current_sector.minutes,
				current_sector.seconds,
				current_sector.seconds,
			];

			(response, AVG_CYCLES)
		} else {
			(CmdResponse::error(&self, ERROR_CANNOT_RESPOND), AVG_CYCLES)
		}
	}

	pub fn seek_l(&mut self) -> (CmdResponse, u64) {
		debug!("SeekL");

		let mut first_response = CmdResponse::int3_status(&self);

		let second_response = CmdResponse {
			int_level: 2,
			result: vec![self.get_stat()],
			second_response: None,
			on_complete: Some(Self::seek_l_complete),
		};

		self.drive_state = DriveState::Seek;

		first_response.second_response = Some((Box::new(second_response), DELAY_1MS));

		(first_response, AVG_CYCLES)
	}

	// TODO just a copy of SeekL for now
	pub fn seek_p(&mut self) -> (CmdResponse, u64) {
		debug!("SeekP");

		let mut first_response = CmdResponse::int3_status(&self);

		let second_response = CmdResponse {
			int_level: 2,
			result: vec![self.get_stat()],
			second_response: None,
			on_complete: Some(Self::seek_l_complete),
		};

		self.drive_state = DriveState::Seek;

		first_response.second_response = Some((Box::new(second_response), DELAY_1MS));

		(first_response, AVG_CYCLES)
	}

	pub fn seek_l_complete(&mut self) -> Option<(CmdResponse, u64)> {
		//trace!("SeekL complete");
		self.current_seek = self.seek_target;
		self.seek_complete = true;
		self.drive_state =  DriveState::Idle;

		None
	}

	pub fn set_mode(&mut self) -> (CmdResponse, u64) {
		if self.params_fifo.len() < 1 {
			return (CmdResponse::error(&self, ERROR_INVALID_PARAMS), AVG_CYCLES);
		}

		self.last_sector_size = self.sector_size;

		let new_mode = self.params_fifo.pop_front().unwrap();
		self.drive_speed = DriveSpeed::from_bits((new_mode >> 7) & 1 != 0);
		self.sector_size = SectorSize::from_bits((new_mode >> 5) & 1 != 0);
		self.ignore_cur_sector_size = (new_mode >> 4) & 1 != 0;

		debug!("SetMode 0b{new_mode:b} {:?} {:?} ignore bit: {}", self.drive_speed, self.sector_size, self.ignore_cur_sector_size);

		(CmdResponse::int3_status(self), AVG_CYCLES)
	}

	pub fn read_n(&mut self) -> (CmdResponse, u64) {
		if self.disc.is_none() {
			return (CmdResponse::error(&self, ERROR_CANNOT_RESPOND), AVG_CYCLES);
		}

		self.read_offset = CdIndex::ZERO;
		self.read_paused = false;
		self.drive_state = DriveState::Read;
		
		if !self.seek_complete {
			self.current_seek = self.seek_target;
		}
		
		debug!("ReadN START @ {}", self.current_seek);

		let mut first_response = CmdResponse::int3_status(self);

		let first_read = CmdResponse {
			int_level: 1,
			result: vec![self.get_stat()],
			second_response: None,
			on_complete: Some(Self::read_n_complete)
		};

		first_response.second_response = Some((Box::new(first_read), READ_CYCLES[self.drive_speed as usize]));
		(first_response, AVG_CYCLES)
	}

	pub fn read_n_complete(&mut self) -> Option<(CmdResponse, u64)> {
		if self.read_paused || self.drive_state != DriveState::Read {
			return None;
		}

		if let Some(disc) = &self.disc {
			trace!("Read sector {} ({} + {}) {} {:?}", (self.current_seek + self.read_offset), self.current_seek, self.read_offset, self.read_paused, self.drive_state);

			let sector = disc.read_sector(self.current_seek + self.read_offset);

			let data = match if !self.ignore_cur_sector_size { self.sector_size } else { self.last_sector_size } {
				SectorSize::DataOnly => sector.data_only(),
				SectorSize::WholeSector => sector.whole_sector()
			};

			self.data_fifo.read_sector(data);

			self.read_offset = self.read_offset + CdIndex::new(0, 0, 1);

			let next_read = CmdResponse {
				int_level: 1,
				result: vec![self.get_stat()],
				second_response: None,
				on_complete: Some(Self::read_n_complete)
			};

			return Some((next_read, READ_CYCLES[self.drive_speed as usize]));
		}

		None
	}

	pub fn pause(&mut self) -> (CmdResponse, u64) {
		debug!("Pause @ read {}", self.current_seek + self.read_offset);

		let mut first_response = CmdResponse::int3_status(self);

		let second_response_cycles = if self.read_paused { 0x1DF2 } else { PAUSE_CYCLES[self.drive_speed as usize] };
		
		// clear stat for second response
		self.read_paused = true;
		self.drive_state = DriveState::Idle;

		let second_response = CmdResponse {
			int_level: 2, 
			result: vec![self.get_stat()],
			second_response: None,
			on_complete: None,
		};


		first_response.second_response = Some((Box::new(second_response), second_response_cycles));
		(first_response, AVG_CYCLES)
	}

	// TODO
	pub fn play(&mut self) -> (CmdResponse, u64) {
		let Some(ref disc) = self.disc else {
			return (CmdResponse::error(&self, ERROR_CANNOT_RESPOND), AVG_CYCLES);
		};

		self.read_offset = CdIndex::ZERO;
		self.read_paused = false;
		self.drive_state = DriveState::Play;

		/* if !self.seek_complete {
			self.current_seek = self.seek_target;
		} */

		// if track param is sent and track>0, start playback at the start of the track
		// otherwise start playback for current seek location
		if let Some(track) = self.params_fifo.pop_front() {
			debug!("Play track {track}");
			if track > 0 {
				self.current_seek = disc.get_track_start(track as usize);
				debug!("Play track {track} @ {}", self.current_seek);
			}
		} else {
			self.current_seek = self.seek_target;
			debug!("Play @ {}", self.current_seek);
		}

		let mut first_response = CmdResponse::int3_status(self);

		let first_read = CmdResponse {
			int_level: 0,
			result: vec![],
			second_response: None,
			on_complete: Some(Self::play_complete)
		};

		first_response.second_response = Some((Box::new(first_read), READ_CYCLES[0]));
		(first_response, AVG_CYCLES)

	}

	fn play_complete(&mut self) -> Option<(CmdResponse, u64)> {
		if self.read_paused || self.drive_state != DriveState::Play {
			return None;
		}

		if let Some(disc) = &self.disc {
			trace!("Play sector {} ({} + {}) {} {:?}", (self.current_seek + self.read_offset), self.current_seek, self.read_offset, self.read_paused, self.drive_state);

			let sector = disc.read_sector(self.current_seek + self.read_offset);
			let data = sector.whole_sector();

			self.audio_buf.read_sector(data);

			self.read_offset = self.read_offset + CdIndex::new(0, 0, 1);

			// TODO report irqs
			let next_read = CmdResponse {
				int_level: 0,
				result: vec![0],
				second_response: None,
				on_complete: Some(Self::play_complete)
			};

			// single speed only (?)
			return Some((next_read, READ_CYCLES[0]));
		}

		None
	}

	// TODO
	pub fn stop(&mut self) -> (CmdResponse, u64) {
		self.drive_state = DriveState::Idle;

		(CmdResponse::int3_status(self), AVG_CYCLES)
	}

	pub fn init(&mut self) -> (CmdResponse, u64) {
		// TODO this cmd should abort all other commands
		// TODO set mode to 0x20
		// this should also happen on the second response
		self.motor_on = true;

		debug!("Init");

		let mut first_response = CmdResponse::int3_status(&self);
		let second_response = CmdResponse {
			int_level: 2,
			result: vec![self.get_stat()],
			second_response: None,
			on_complete: None,
		};

		first_response.second_response = Some((Box::new(second_response), DELAY_1MS));
		(first_response, 0x13CCE)
	}

	pub fn mute(&mut self) -> (CmdResponse, u64) {
		self.audio_muted = true;

		debug!("Mute");

		(CmdResponse::int3_status(&self), AVG_CYCLES)
	}

	pub fn demute(&mut self) -> (CmdResponse, u64) {
		self.audio_muted = false;

		debug!("Demute");

		(CmdResponse::int3_status(&self), AVG_CYCLES)
	}

	// just a copy of init
	pub fn motor_on(&mut self) -> (CmdResponse, u64) {
		self.motor_on = true;

		let mut first_response = CmdResponse::int3_status(&self);
		let second_response = CmdResponse {
			int_level: 2,
			result: vec![self.get_stat()],
			second_response: None,
			on_complete: None,
		};

		first_response.second_response = Some((Box::new(second_response), DELAY_1MS));
		(first_response, 0x13CCE)
	}

}
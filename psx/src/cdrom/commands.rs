use super::*;

pub const AVG_CYCLES: u64 = 0xC4E1;
pub const DELAY_1MS: u64 = 0x844D;

pub const READ_CYCLES: [u64; 2] = [0x6E1CD, 0x36CD2]; // single speed, double speed
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

	pub fn seek_l(&mut self) -> (CmdResponse, u64) {
		debug!("SeekL");

		let mut first_response = CmdResponse::int3_status(&self);

		let second_response = CmdResponse {
			int_level: 2,
			result: vec![self.get_stat()],
			second_response: None,
			on_complete: Some(Self::seek_l_complete),
		};

		first_response.second_response = Some((Box::new(second_response), 0x10000));

		(first_response, AVG_CYCLES)
	}

	pub fn seek_l_complete(&mut self) -> Option<(CmdResponse, u64)> {
		//trace!("SeekL complete");
		self.current_seek = self.seek_target;
		self.seek_complete = true;

		None
	}

	pub fn set_mode(&mut self) -> (CmdResponse, u64) {
		if self.params_fifo.len() < 1 {
			return (CmdResponse::error(&self, ERROR_INVALID_PARAMS), AVG_CYCLES);
		}

		let new_mode = self.params_fifo.pop_front().unwrap();
		self.drive_speed = DriveSpeed::from_bits((new_mode >> 7) & 1 != 0);
		self.sector_size = SectorSize::from_bits((new_mode >> 5) & 1 != 0);

		debug!("SetMode 0b{new_mode:b} {:?} {:?}", self.drive_speed, self.sector_size);

		(CmdResponse::int3_status(self), AVG_CYCLES)
	}

	pub fn read_n(&mut self) -> (CmdResponse, u64) {
		if self.disc.is_none() {
			return (CmdResponse::error(&self, ERROR_CANNOT_RESPOND), AVG_CYCLES);
		}

		self.read_offset = CdIndex::ZERO;
		self.read_paused = false;
		self.reading = true;
		
		self.data_fifio.clear();

		debug!("ReadN START");

		if !self.seek_complete {
			self.current_seek = self.seek_target;
		}

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
		if self.read_paused || !self.reading {
			return None;
		}

		if let Some(disc) = &self.disc {
			trace!("ReadN sector {} ({} + {}) {} {}", (self.current_seek + self.read_offset), self.current_seek, self.read_offset, self.read_paused, self.reading);

			let sector = disc.read_sector(self.current_seek + self.read_offset);

			let data = match self.sector_size {
				SectorSize::DataOnly => sector.data_only(),
				SectorSize::WholeSector => sector.whole_sector()
			};

			for byte in data {
				self.data_fifio.push_back(byte);
			}

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
		debug!("Pause");

		let mut first_response = CmdResponse::int3_status(self);

		let second_response_cycles = if self.read_paused { 0x1DF2 } else { PAUSE_CYCLES[self.drive_speed as usize] };
		let second_response = CmdResponse {
			int_level: 2, 
			// clear stat for second response
			result: vec![self.get_stat() & !(1 << 5)],
			second_response: None,
			on_complete: None,
		};

		self.read_paused = true;
		self.reading = false;

		first_response.second_response = Some((Box::new(second_response), second_response_cycles));
		(first_response, AVG_CYCLES)
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

}
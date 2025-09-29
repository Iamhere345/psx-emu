use std::{fmt::Display, fs::File, io::Read, ops::{Add, Sub}, path::Path};
use log::*;

const SECONDS_PER_MINUTE: usize = 60;
const SECTORS_PER_SECOND: usize = 75;
const BYTES_PER_SECTOR: usize = 0x930;

#[derive(Clone, Copy, Debug)]
pub struct CdIndex {
	pub minutes: u8,
	pub seconds: u8,
	pub sectors: u8,
}

impl CdIndex {

	pub const ZERO: Self = Self { minutes: 0, seconds: 0, sectors: 0 };

	pub fn new(minutes: u8, seconds: u8, sectors: u8) -> Self {
		Self {
			minutes,
			seconds,
			sectors
		}
	}

	pub fn from_bcd(minutes: u8, seconds: u8, sectors: u8) -> Self {
		Self {
			minutes: bcd_to_binary(minutes),
			seconds: bcd_to_binary(seconds),
			sectors: bcd_to_binary(sectors),
		}
	}

	pub fn from_lba(lba: usize) -> Self {
		let minutes = lba / (SECTORS_PER_SECOND * SECONDS_PER_MINUTE);
		let seconds = (lba / SECTORS_PER_SECOND) % SECONDS_PER_MINUTE;
		let sectors = lba % SECTORS_PER_SECOND;

		Self::new(minutes as u8, seconds as u8, sectors as u8)
	}

	pub fn to_lba(&self) -> usize {
		((usize::from(self.minutes) * SECONDS_PER_MINUTE * SECTORS_PER_SECOND) + (usize::from(self.seconds) * SECTORS_PER_SECOND) + usize::from(self.sectors)).saturating_sub(150)
	}
}

impl Add for CdIndex {
	type Output = Self;

	fn add(self, rhs: Self) -> Self::Output {
		let (sectors, carry) = add(self.sectors, rhs.sectors, false, SECTORS_PER_SECOND as u8);
		let (seconds, carry) = add(self.seconds, rhs.seconds, carry, SECONDS_PER_MINUTE as u8);
		let (minutes, _) = add(self.minutes, rhs.minutes, carry, 80);

		Self::new(minutes, seconds, sectors)
	}
}

impl Sub for CdIndex {
	type Output = Self;

	fn sub(self, rhs: Self) -> Self::Output {
		Self::from_lba(self.to_lba() - rhs.to_lba())
	}
}

impl Display for CdIndex {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}:{}:{} LBA {}", self.minutes, self.seconds, self.sectors, self.to_lba())
	}
}

pub struct Track {
	number: usize,
	data: Vec<u8>,
	sectors: usize,
	start_lba: usize,
	end_lba: usize,
}

impl Track {
	pub const EMPTY: Track = Track { number: 0, data: Vec::new(), sectors: 0, start_lba: 0, end_lba: 0 };
}

pub struct Disc {
	pub tracks: Vec<Track>
}

impl Disc {
	pub fn new() -> Self {
		Self {
			tracks: Vec::new(),
		}
	}

	pub fn add_tracks(&mut self, tracks: Vec<Vec<u8>>) {
		let mut total_sectors = 0;
		let mut track_num = 0;

		for track_data in tracks {
			let sectors = track_data.len() / BYTES_PER_SECTOR;

			let start_lba = total_sectors + 150 + (usize::from(track_num > 0) * 150);
			let end_lba = total_sectors + sectors;
			
			total_sectors += sectors;
			track_num += 1;

			self.tracks.push(Track {
				number: track_num,
				data: track_data,
				sectors: sectors,
				start_lba: start_lba,
				end_lba: end_lba,
			});

		}
	}

	pub fn read_sector(&self, index: CdIndex) -> Sector {
		let sector_addr = index.to_lba() * BYTES_PER_SECTOR;

		trace!("addr: {sector_addr} lba: {} msf: {} {sector_addr}..{sector_addr} + {BYTES_PER_SECTOR}", index.to_lba(), index);

		//if self.data.len() < sector_addr + BYTES_PER_SECTOR {
		let (track_num, start_addr) = self.get_track_number(sector_addr);
		let track_addr = sector_addr - start_addr;

		Sector::new(self.tracks[track_num].data[track_addr..track_addr + BYTES_PER_SECTOR].to_vec())

	}

	pub fn get_track_number(&self, sector_addr: usize) -> (usize, usize) {
		let mut track_addr = 0;
		for (track_num, track) in self.tracks.iter().enumerate() {
			if sector_addr >= track_addr && sector_addr < track_addr + track.data.len() {
				return (track_num, track_addr)
			}
			
			track_addr += track.data.len();
		}

		panic!("couldn't find track");
	}

	pub fn get_track_start(&self, track_num: usize) -> CdIndex {
		CdIndex::from_lba(self.tracks[track_num - 1].start_lba)
	}

	pub fn get_track_offset(&self, abs_index: CdIndex) -> (CdIndex, usize) {
		let track = &self.tracks[self.get_track_from_index(abs_index)];

		(CdIndex::from_lba(abs_index.to_lba() - track.start_lba), track.number)
	}

	fn get_track_from_index(&self, index: CdIndex) -> usize {
		let index_lba = index.to_lba();

		for (i,  track) in self.tracks.iter().enumerate() {
			if index_lba >= track.start_lba && index_lba <= track.end_lba {
				return i
			}
		}

		0
	}

	pub fn get_disc_end(&self) -> CdIndex {
		CdIndex::from_lba(self.tracks.last().unwrap().end_lba + 150)
	}
}

pub struct Sector {
	data: Vec<u8>
}

impl Sector {
	pub fn new(data: Vec<u8>) -> Self {
		Self {
			data
		}
	}

	pub fn whole_sector(&self) -> &[u8] {
		&self.data[0xC..]
	}

	pub fn data_only(&self) -> &[u8] {
		&self.data[0x18..0x18 + 0x800]
	}
}

pub fn bcd_to_binary(value: u8) -> u8 {
    10 * (value >> 4) + (value & 0xF)
}

pub fn binary_to_bcd(value: u8) -> u8 {
    ((value / 10) << 4) | (value % 10)
}

fn add(a: u8, b: u8, overflow: bool, base: u8) -> (u8, bool) {
	let sum = a + b + u8::from(overflow);
	(sum % base, sum >= base)
}


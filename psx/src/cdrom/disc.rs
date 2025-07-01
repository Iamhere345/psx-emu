use std::{fmt::Display, fs::File, io::Read, ops::{Add, Sub}, path::Path};
use log::*;

const SECONDS_PER_MINUTE: usize = 60;
const SECTORS_PER_SECOND: usize = 75;
const BYTES_PER_SECTOR: usize = 0x930;

#[derive(Clone, Copy, Debug)]
pub struct CdIndex {
	minutes: u8,
	seconds: u8,
	sectors: u8,
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

pub struct Disc {
	pub tracks: Vec<Vec<u8>>
}

impl Disc {
	pub fn new(tracks: Vec<Vec<u8>>) -> Self {
		Self {
			tracks
		}
	}

	pub fn read_sector(&self, index: CdIndex) -> Sector {
		let sector_addr = index.to_lba() * BYTES_PER_SECTOR;

		trace!("addr: {sector_addr} lba: {} msf: {} {sector_addr}..{sector_addr} + {BYTES_PER_SECTOR}", index.to_lba(), index);

		//if self.data.len() < sector_addr + BYTES_PER_SECTOR {
		let (track_num, start_addr) = self.get_track_number(sector_addr);
		let track_addr = sector_addr - start_addr;

		Sector::new(self.tracks[track_num][track_addr..track_addr + BYTES_PER_SECTOR].to_vec())

	}

	fn get_track_number(&self, sector_addr: usize) -> (usize, usize) {
		let mut track_addr = 0;
		for (track_num, track) in self.tracks.iter().enumerate() {
			if sector_addr >= track_addr && sector_addr < track_addr + track.len() {
				return (track_num, track_addr)
			}
			
			track_addr += track.len();
		}

		panic!("couldn't find track");
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

	pub fn whole_sector(&self) -> Vec<u8> {
		self.data[0xC..].to_vec()
	}

	pub fn data_only(&self) -> Vec<u8> {
		self.data[0x18..0x18 + 0x800].to_vec()
	}
}

fn bcd_to_binary(value: u8) -> u8 {
    10 * (value >> 4) + (value & 0xF)
}

fn binary_to_bcd(value: u8) -> u8 {
    ((value / 10) << 4) | (value % 10)
}

fn add(a: u8, b: u8, overflow: bool, base: u8) -> (u8, bool) {
	let sum = a + b + u8::from(overflow);
	(sum % base, sum >= base)
}


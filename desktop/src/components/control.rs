use std::fs::{self, File};
use std::io::Read;
use std::path::PathBuf;
use std::time::Duration;

use eframe::egui::Ui;
use rfd::FileDialog;
use rcue::parser::parse_from_file;
use rodio::buffer::SamplesBuffer;

use psx::PSXEmulator;
use psx::cdrom::disc::Disc;
use rodio::OutputStream;

use crate::app::BIOS_PATH;
use crate::components::breakpoints::Breakpoints;
use crate::components::tty_logger::TTYLogger;

pub struct Control {
	pub paused: bool,
	pub step: bool,
}

impl Control {
	pub fn new() -> Self {
		Self {
			paused: true,
			step: false,
		}
	}

	pub fn show(&mut self, ui: &mut Ui, psx: &mut PSXEmulator, tty: &mut TTYLogger, breakpoints: &mut Breakpoints, stream_handle: &mut OutputStream) {
		ui.strong("Control");

		ui.horizontal(|ui| {
			if ui.button(if self.paused { "Start" } else { "Stop" }).clicked() {
				self.paused = !self.paused;
			}
	
			if ui.button("Step").clicked() {
				self.step = true;
			}

			if ui.button("Load Disc").clicked() {
				let disc_path = self.select_file(("CUE File", &["cue"]));

				if let Some(disc) = disc_path {
					self.load_disc(disc.to_str().unwrap(), psx);
				}
			}

			if ui.button("Sideload EXE").clicked() {
				let exe_path = self.select_file(("EXE File", &["exe", "ps-exe"]));

				if let Some(exe) = exe_path {
					self.reset_emu(psx, tty, breakpoints, stream_handle);
					psx.sideload_exe(fs::read(exe).unwrap());
				}
			}

			if ui.button("Reset").clicked() {
				self.reset_emu(psx, tty, breakpoints, stream_handle);
			}
		});

	}

	pub fn select_file(&mut self, filter: (&str, &[&str])) -> Option<PathBuf> {

		let exe_path = std::env::current_dir().unwrap();

		FileDialog::new()
			.add_filter(filter.0, filter.1)
			.set_directory(exe_path)
			.pick_file()

	}

	pub fn load_disc(&self, cue_path: &str, psx: &mut PSXEmulator) {
		let cue = parse_from_file(cue_path, false).unwrap();

		let mut cue_dir = PathBuf::from(cue_path);
		cue_dir.pop();

		let mut tracks: Vec<Vec<u8>> = Vec::new();

		for track in cue.files {
			let mut track_path = cue_dir.clone();
			track_path.push(track.file);

			let mut track_file = File::open(track_path).unwrap();

			let mut data = Vec::new();
			track_file.read_to_end(&mut data).expect("Unable to read track data");

			tracks.push(data);
		}

		let disc = Disc::new(tracks);

		psx.load_disc(disc);
	}

	pub fn reset_emu(&mut self, psx: &mut PSXEmulator, tty: &mut TTYLogger, breakpoints: &mut Breakpoints, stream_handle: &mut OutputStream) {
		let sink = rodio::Sink::connect_new(&stream_handle.mixer());

		let audio_callback = Box::new(move |buffer: Vec<f32>| {
			if sink.len() > 2 {
				std::thread::sleep(Duration::from_millis(1));
			}

			sink.append(SamplesBuffer::new(1, 44100, buffer));
		});

		let bios = fs::read(BIOS_PATH).unwrap();
		*psx = PSXEmulator::new(bios, audio_callback);

		tty.out_buf.clear();
		breakpoints.breakpoints.clear();
	}

}
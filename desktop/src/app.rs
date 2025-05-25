use std::fs::{self, File};
use std::io::Read;
use std::path::PathBuf;

use eframe::egui::{self, Key};
use eframe::{App, CreationContext};

use rcue::parser::parse_from_file;

use psx::PSXEmulator;
use psx::cdrom::disc::Disc;

use crate::components::{control::*, vram::*, tty_logger::*, disassembly::*};

const BIOS_PATH: &str = "res/SCPH1001.bin";

const BTN_UP: Key		= Key::W;
const BTN_DOWN: Key		= Key::S;
const BTN_LEFT: Key 	= Key::A;
const BTN_RIGHT: Key 	= Key::D;
const BTN_CROSS: Key	= Key::K;
const BTN_SQUARE: Key	= Key::J;
const BTN_TRIANGLE: Key	= Key::I;
const BTN_CIRCLE: Key	= Key::L;
const BTN_L1: Key		= Key::Q;
const BTN_L2: Key		= Key::Num1;
const BTN_R1: Key		= Key::E;
const BTN_R2: Key		= Key::Num3;
const BTN_START: Key	= Key::Enter;
const BTN_SELECT: Key	= Key::Backslash;

pub struct Desktop {
	psx: PSXEmulator,

	control: Control,
	vram: VramViewer,
	tty_logger: TTYLogger,
	disassembly: Disassembly,

	control_open: bool,
}

impl Desktop {
	pub fn new(cc: &CreationContext) -> Self {

		let bios = fs::read(BIOS_PATH).unwrap();

		#[allow(unused_mut)]
		let mut psx = PSXEmulator::new(bios);
		//load_disc(r"E:\Roms\PS1\Ridge Racer (USA)\Ridge Racer (USA).cue", &mut psx);
		//load_disc(r"E:\Roms\PS1\Puzzle Bobble\Puzzle Bobble 2 (Japan).cue", &mut psx);
		
		//load_disc("res/hello-tests/hello_cd.cue", &mut psx);
		//psx.sideload_exe(fs::read("res/hello-tests/hello_cd.exe").unwrap());

		//load_disc(r"E:\Roms\PS1\Crash Bandicoot\Crash Bandicoot (Europe, Australia).cue", &mut psx);
		//load_disc(r"E:\Roms\PS1\Mega Man X4 (USA)\Mega Man X4 (USA).cue", &mut psx);
		//load_disc(r"E:\Roms\PS1\Gran Turismo 2 Arcade NTSCJAP\GRANTURISMO2.CUE", &mut psx);
		//load_disc(r"E:\Roms\PS1\Mortal Kombat II (Japan)\Mortal Kombat II (Japan).cue", &mut psx);
		load_disc(r"C:\Users\lunar\Downloads\SONICPSX\GAME.cue", &mut psx);
		//load_disc(r"E:\Roms\PS1\Castlevania - Symphony of the Night (USA)\Castlevania - Symphony of the Night (USA).cue", &mut psx);
		//load_disc(r"E:\Roms\PS1\Metal Gear Solid\Metal Gear Solid (Europe) (Disc 1).cue", &mut psx);

		//psx.sideload_exe(fs::read("res/hello-tests/hello_pad.exe").unwrap());
		//psx.sideload_exe(fs::read("res/pong.exe").unwrap());
		//psx.sideload_exe(fs::read("res/redux-tests/dma.exe").unwrap());
		//psx.sideload_exe(fs::read("res/RenderTextureRectangle15BPP.exe").unwrap());
		//psx.sideload_exe(fs::read("res/psxtest_cpx.exe").unwrap());
		//psx.sideload_exe(fs::read("res/tests/benchmark.exe").unwrap());

		Self {
			psx: psx,

			control: Control::new(),
			vram: VramViewer::new(cc),
			tty_logger: TTYLogger::new(),
			disassembly: Disassembly::new(),

			control_open: true,
		}
	}

	fn is_keyboard_input_down(&mut self, key: Key, ctx: &egui::Context) -> bool {
		ctx.input(|input| {
			input.key_down(key)
		})
	}

	fn handle_input(&mut self, ctx: &egui::Context) {
		
		let up = self.is_keyboard_input_down(BTN_UP, ctx);
		let down = self.is_keyboard_input_down(BTN_DOWN, ctx);
		let left = self.is_keyboard_input_down(BTN_LEFT, ctx);
		let right = self.is_keyboard_input_down(BTN_RIGHT, ctx);
		let cross = self.is_keyboard_input_down(BTN_CROSS, ctx);
		let square = self.is_keyboard_input_down(BTN_SQUARE, ctx);
		let triangle = self.is_keyboard_input_down(BTN_TRIANGLE, ctx);
		let circle = self.is_keyboard_input_down(BTN_CIRCLE, ctx);
		let l1 = self.is_keyboard_input_down(BTN_L1, ctx);
		let l2 = self.is_keyboard_input_down(BTN_L2, ctx);
		let r1 = self.is_keyboard_input_down(BTN_R1, ctx);
		let r2 = self.is_keyboard_input_down(BTN_R2, ctx);
		let start = self.is_keyboard_input_down(BTN_START, ctx);
		let select = self.is_keyboard_input_down(BTN_SELECT, ctx);
		
		self.psx.update_input(up, down, left, right, cross, square, triangle, circle, l1, l2, r1, r2, start, select);
	}

}

impl App for Desktop {
	fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {

		self.handle_input(ctx);

		if !self.control.paused {
			//for _ in 0..CYCLES_PER_SECOND {
				//self.psx.tick();
			//}
			
			self.psx.run_frame();
		}

		if self.control.step {
			self.psx.tick();
			self.control.step = false;
		}
		
		if self.control_open {
			egui::Window::new("CPU").show(ctx, |ui| {
				self.control.show(ui);

				ui.separator();

				self.disassembly.show(ui, &mut self.psx);
			});
		}

		egui::Window::new("VRAM Viewer").show(ctx, |ui| {
			self.vram.show(ui, &self.psx);
		});

		egui::Window::new("TTY Output").show(ctx, |ui| {
			egui::ScrollArea::vertical()
				.stick_to_bottom(true)
				.show(ui, |ui| {
					self.tty_logger.show(ui, &mut self.psx);
				});
		});

		ctx.request_repaint();
	}
}

fn load_disc(cue_path: &str, psx: &mut PSXEmulator) {
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
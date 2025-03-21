use std::fs;

use eframe::egui::{self, Key};
use eframe::{App, CreationContext};
use psx::PSXEmulator;

use crate::components::{control::*, vram::*, tty_logger::*};

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

	control_open: bool,
}

impl Desktop {
	pub fn new(cc: &CreationContext) -> Self {

		let bios = fs::read(BIOS_PATH).unwrap();

		#[allow(unused_mut)]
		let mut psx = PSXEmulator::new(bios);
		psx.sideload_exe(fs::read("res/hello-tests/hello_pad.exe").unwrap());
		//psx.sideload_exe(fs::read("res/tests/pad.exe").unwrap());
		//psx.sideload_exe(fs::read("res/redux-tests/dma.exe").unwrap());
		//psx.sideload_exe(fs::read("res/RenderTextureRectangle15BPP.exe").unwrap());

		Self {
			psx: psx,

			control: Control::new(),
			vram: VramViewer::new(cc),
			tty_logger: TTYLogger::new(),

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
		
		if self.control_open {
			egui::Window::new("Control").show(ctx, |ui| {
				self.control.show(ui);
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
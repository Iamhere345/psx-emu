use std::fs;

use eframe::egui;
use eframe::{App, CreationContext};
use psx::PSXEmulator;

use crate::components::control::Control;

const BIOS_PATH: &str = "res/SCPH1001.bin";
const CYCLES_PER_SECOND: usize = (33868800.0 / 60.0) as usize;

pub struct Desktop {
	psx: PSXEmulator,

	control: Control,

	control_open: bool,
}

impl Desktop {
	pub fn new(cc: &CreationContext) -> Self {

		let bios = fs::read(BIOS_PATH).unwrap();

		let mut psx = PSXEmulator::new(bios);
		psx.sideload_exe(fs::read("res/psxtest_cpu.exe").unwrap());

		println!("past sideload");

		Self {
			psx: psx,

			control: Control::new(),

			control_open: true,
		}
	}
}

impl App for Desktop {
	fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {

		if !self.control.paused {
			for _ in 0..CYCLES_PER_SECOND {
				self.psx.tick();
			}
		}

		//egui::CentralPanel::default().show(ctx, |ui| {
			
		//});
		
		if self.control_open {
			egui::Window::new("Control").show(ctx, |ui| {
				self.control.show(ui);
			});
		}
	}
}
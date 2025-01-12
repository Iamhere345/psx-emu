use std::fs;

use eframe::egui;
use eframe::{App, CreationContext};
use psx::PSXEmulator;

use crate::components::{control::*, vram::*, tty_logger::*};

const BIOS_PATH: &str = "res/SCPH1001.bin";
const CYCLES_PER_SECOND: usize = (33868800.0 / 60.0) as usize;

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

		let mut psx = PSXEmulator::new(bios);
		//psx.sideload_exe(fs::read("res/redux-tests/dma.exe").unwrap());
		//psx.sideload_exe(fs::read("res/MemoryTransfer16BPP.exe").unwrap());
		//psx.sideload_exe(fs::read("res/triangle.exe").unwrap());

		Self {
			psx: psx,

			control: Control::new(),
			vram: VramViewer::new(cc),
			tty_logger: TTYLogger::new(),

			control_open: true,
		}
	}
}

impl App for Desktop {
	fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {

		if !self.control.paused {
			for _ in 0..CYCLES_PER_SECOND {
				self.psx.tick();
			}
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
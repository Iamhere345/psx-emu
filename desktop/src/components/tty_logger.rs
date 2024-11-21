use eframe::egui::Ui;
use psx::PSXEmulator;

pub struct TTYLogger {
	out_buf: String
}

impl TTYLogger {
	pub fn new() -> Self {
		Self {
			out_buf: String::new()
		}
	}

	pub fn show(&mut self, ui: &mut Ui, psx: &mut PSXEmulator) {

		// this is probably terrible for performance
		self.out_buf.push_str(psx.get_tty_buf().as_str());

		ui.monospace(self.out_buf.clone());

	}
}
use eframe::egui::Ui;

pub struct Control {
	pub paused: bool,
}

impl Control {
	pub fn new() -> Self {
		Self {
			paused: true,
		}
	}

	pub fn show(&mut self, ui: &mut Ui) {
		if ui.button(if self.paused { "Start" } else { "Stop" }).clicked() {
			self.paused = !self.paused;
		}
	}
}
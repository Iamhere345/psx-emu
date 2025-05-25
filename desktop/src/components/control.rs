use eframe::egui::Ui;

pub struct Control {
	pub paused: bool,
	pub step: bool,
}

impl Control {
	pub fn new() -> Self {
		Self {
			paused: true,
			step: false
		}
	}

	pub fn show(&mut self, ui: &mut Ui) {
		ui.strong("Control");

		ui.horizontal(|ui| {
			if ui.button(if self.paused { "Start" } else { "Stop" }).clicked() {
				self.paused = !self.paused;
			}
	
			if ui.button("Step").clicked() {
				self.step = true;
			}
		});
	}
}
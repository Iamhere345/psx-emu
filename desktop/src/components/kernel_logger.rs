use eframe::egui::Ui;
use egui_extras::{Column, TableBuilder};

pub struct KernelLogger;

impl KernelLogger {
	pub fn new() -> Self {
		Self {}
	}

	pub fn show(&mut self, ui: &mut Ui, log_buf: &mut Vec<String>) {
		let table = TableBuilder::new(ui)
			.column(Column::remainder())
			.striped(true)
			.stick_to_bottom(true);

		table.header(20.0, |mut header| {
			header.col(|ui| {
				ui.label("Function");
			});
		})
		.body(|body| {
			body.rows(12.0, log_buf.len(), |mut row| {
				let i = row.index();
				row.col(|ui| {
					ui.monospace(log_buf[i].clone());
				});
			});
		});
	}
}
//#![windows_subsystem = "windows"]
use eframe::{egui::{Vec2, ViewportBuilder}, NativeOptions};
use app::Desktop;

pub mod components;
mod app;

fn main() {

	let viewport = ViewportBuilder {
		inner_size: Some(Vec2::new(1280.0, 720.0)),
		..Default::default()
	};

	let native_options = NativeOptions {
		viewport: viewport,
		vsync: false,
		..Default::default()
	};

    eframe::run_native("My egui App", native_options, Box::new(|cc| Ok(Box::new(Desktop::new(cc))))).expect("Unable to initialise egui app");

}
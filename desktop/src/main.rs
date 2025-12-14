//#![windows_subsystem = "windows"]
use eframe::{egui::{Pos2, Vec2, ViewportBuilder}, NativeOptions};
use env_logger::*;

use app::Desktop;

pub mod components;
mod app;
mod input;

fn main() {

	let mut builder = Builder::from_env(Env::default().default_filter_or("psx=debug"));
	builder.target(Target::Stdout);
	builder.init();

	let viewport = ViewportBuilder {
		inner_size: Some(Vec2::new(1800.0, 800.0)),
		position: Some(Pos2::new(0.0, 0.0)),
		..Default::default()
	};

	let native_options = NativeOptions {
		viewport: viewport,
		vsync: false,
		..Default::default()
	};

    eframe::run_native("PSX Emu", native_options, Box::new(|cc| Ok(Box::new(Desktop::new(cc))))).expect("Unable to initialise egui app");

}
use std::time::Duration;

use eframe::egui::{self, CentralPanel, Key};
use eframe::{App, CreationContext};
use egui_dock::{DockArea, DockState, NodeIndex, Style, SurfaceIndex, TabViewer};
use rodio::buffer::SamplesBuffer;
use rodio::OutputStream;

use psx::PSXEmulator;

use crate::components::breakpoints::Breakpoints;
use crate::components::kernel_logger::KernelLogger;
use crate::components::{control::*, disassembly::*, tty_logger::*, display::*};

type Tab = String;

pub const BIOS_PATH: &str = "res/SCPH1001.bin";

const BTN_UP: 		Key = Key::W;
const BTN_DOWN: 	Key = Key::S;
const BTN_LEFT: 	Key = Key::A;
const BTN_RIGHT: 	Key = Key::D;
const BTN_CROSS: 	Key = Key::K;
const BTN_SQUARE: 	Key = Key::J;
const BTN_TRIANGLE: Key = Key::I;
const BTN_CIRCLE: 	Key = Key::L;
const BTN_L1: 		Key = Key::Q;
const BTN_L2: 		Key = Key::Num1;
const BTN_R1: 		Key = Key::E;
const BTN_R2: 		Key = Key::Num3;
const BTN_START: 	Key = Key::Enter;
const BTN_SELECT: 	Key = Key::Backslash;

pub struct FrontendState {
	psx: PSXEmulator,

	control: Control,
	vram: VramViewer,
	display: DisplayViwer,
	tty_logger: TTYLogger,
	kernel_logger: KernelLogger,
	disassembly: Disassembly,
	breakpoints: Breakpoints,

	new_breakpoint_open: bool,

	stream_handle: OutputStream,
	//sink: Sink,
}

pub struct Desktop {
	context: FrontendState,
	tree: DockState<Tab>,
}

impl Desktop {
	pub fn new(cc: &CreationContext) -> Self {
		let mut dock_state = DockState::new(vec!["Display".to_string(), "VRAM".to_string()]);

		let surface = dock_state.main_surface_mut();

		let [root_node, left_split] = surface.split_left(NodeIndex::root(), 0.2, vec!["Control".to_string()]);
		surface.split_below(left_split, 0.2, vec!["Disassembly".to_string()]);

		let [root_node, right_split] = surface.split_right(root_node, 0.725, vec!["TTY Logger".to_string()]);
		surface.split_below(right_split, 0.5, vec!["Kernel Logger".to_string()]);

		surface.split_below(root_node, 0.7, vec!["Breakpoints".to_string()]);

		Self {
			context: FrontendState::new(cc),
			tree: dock_state,
		}
	}
}

impl App for Desktop {
	fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
		self.context.update(ctx);

		egui::TopBottomPanel::top("Menu Bar").show(ctx, |ui| {
			egui::menu::bar(ui, |ui| {
				ui.menu_button("View", |ui| {
					for tab in &["Disassembly", "TTY Logger", "Kernel Logger", "Breakpoints"] {
						if ui.button(*tab).clicked() {
							if let Some(index) = self.tree.find_tab(&tab.to_string()) {
								self.tree.remove_tab(index);
							} else {
								self.tree[SurfaceIndex::main()]
									.push_to_focused_leaf(tab.to_string());
							}
						}
					}
				});
			});
		});

		CentralPanel::default()
			.frame(egui::Frame::central_panel(&ctx.style()).inner_margin(0.0))
			.show(ctx, |ui| {
				DockArea::new(&mut self.tree)
					.style(Style::from_egui(ctx.style().as_ref()))
					.show_inside(ui, &mut self.context);
			});
		
		self.context.breakpoints.show_new_breakpoint(ctx, &mut self.context.psx, &mut self.context.new_breakpoint_open);

		ctx.request_repaint();
	}
}

impl TabViewer for FrontendState {
	type Tab = Tab;

	fn title(&mut self, tab: &mut Self::Tab) -> egui::WidgetText {
		tab.as_str().into()
	}

	fn ui(&mut self, ui: &mut egui::Ui, tab: &mut Self::Tab) {
		match tab.as_str() {
			"Control" => self.control.show(ui, &mut self.psx, &mut self.tty_logger, &mut self.breakpoints, &mut self.stream_handle),
			"Disassembly" => self.disassembly.show(ui, &mut self.psx),
			"VRAM" => self.vram.show(ui, &self.psx),
			"Display" => self.display.show(ui, &self.psx),
			"TTY Logger" => self.tty_logger.show(ui, &mut self.psx),
			"Kernel Logger" => self.kernel_logger.show(ui, &mut self.psx.cpu.kernel_log),
			"Breakpoints" => self.breakpoints.show(ui, &mut self.psx, &mut self.new_breakpoint_open),
			_ => {
				ui.label(tab.as_str());
			}
		};
	}

	fn closeable(&mut self, tab: &mut Self::Tab) -> bool {
		!["Control", "VRAM", "Display"].contains(&&tab.as_str())
	}
}

impl FrontendState {
	pub fn new(cc: &CreationContext) -> Self {
		let bios = std::fs::read(BIOS_PATH).unwrap();

		let stream_handle = rodio::OutputStreamBuilder::open_default_stream().expect("open default audio stream");
		let sink = rodio::Sink::connect_new(&stream_handle.mixer());
		
		let audio_callback = Box::new(move |buffer: Vec<f32>| {
			while sink.len() > 2 {
				std::thread::sleep(Duration::from_millis(1));
			}

			sink.append(SamplesBuffer::new(2, 44100, buffer));
		});

		#[allow(unused_mut)]
		let mut psx = PSXEmulator::new(bios, audio_callback);
		Self {
			psx: psx,

			control: Control::new(),
			vram: VramViewer::new(cc),
			display: DisplayViwer::new(cc),
			tty_logger: TTYLogger::new(),
			kernel_logger: KernelLogger::new(),
			disassembly: Disassembly::new(),
			breakpoints: Breakpoints::new(),

			new_breakpoint_open: false,

			stream_handle: stream_handle,
		}

	}

	fn is_keyboard_input_down(&mut self, key: Key, ctx: &egui::Context) -> bool {
		ctx.input(|input| input.key_down(key))
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

		self.psx.update_input(
			up, down, left, right, cross, square, triangle, circle, l1, l2, r1, r2, start, select,
		);
	}

	fn update(&mut self, ctx: &egui::Context) {
		self.handle_input(ctx);

		if !self.control.paused && !self.psx.breakpoint_hit {
			self.psx.run_frame();

			if self.psx.breakpoint_hit {
				self.control.paused = true;
			}
		}

		if self.control.step {
			self.psx.tick();
			self.control.step = false;
		}
	}
}
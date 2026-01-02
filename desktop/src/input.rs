use eframe::egui::{self, Context, Key, Ui};
use gilrs::*;

use psx::sio0::InputState;

// hardcoding this because it seems to be a non-existent controller which always connects first on windows
const EXCLUDED_CONTROLLER: &str = "HID-compliant game controller";

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

pub struct Input {
	active_gamepad: Option<GamepadId>,
	gilrs: Gilrs,

	pub analog_enabled: bool,
}

impl Input {
	pub fn new() -> Self {
		Self {
			active_gamepad: None,
			gilrs: Gilrs::new().unwrap(),

			analog_enabled: false,
		}
	}

	pub fn get_input(&mut self, ctx: &Context) -> InputState {
		if self.active_gamepad.is_some() {
			if let Some(gamepad) = self.gilrs.connected_gamepad(self.active_gamepad.unwrap()) {
				return InputState {
					btn_up: gamepad.is_pressed(Button::DPadUp),
					btn_down: gamepad.is_pressed(Button::DPadDown),
					btn_left: gamepad.is_pressed(Button::DPadLeft),
					btn_right: gamepad.is_pressed(Button::DPadRight),

					btn_cross: gamepad.is_pressed(Button::South),
					btn_square: gamepad.is_pressed(Button::West),
					btn_triangle: gamepad.is_pressed(Button::North),
					btn_circle: gamepad.is_pressed(Button::East),

					btn_l1: gamepad.is_pressed(Button::LeftTrigger),
					btn_l2: gamepad.is_pressed(Button::LeftTrigger2),
					btn_l3: gamepad.is_pressed(Button::LeftThumb),

					btn_r1: gamepad.is_pressed(Button::RightTrigger),
					btn_r2: gamepad.is_pressed(Button::RightTrigger2),
					btn_r3: gamepad.is_pressed(Button::RightThumb),

					btn_start: gamepad.is_pressed(Button::Start),
					btn_select: gamepad.is_pressed(Button::Select),

					l_stick_x: self.get_stick_x(&gamepad, Axis::LeftStickX),
					l_stick_y: self.get_stick_y(&gamepad, Axis::LeftStickY),
					r_stick_x: self.get_stick_x(&gamepad, Axis::RightStickX),
					r_stick_y: self.get_stick_y(&gamepad, Axis::RightStickY),
				};
				
			}
		}

		InputState {
			btn_up: self.is_keyboard_input_down(BTN_UP, ctx),
			btn_down: self.is_keyboard_input_down(BTN_DOWN, ctx),
			btn_left: self.is_keyboard_input_down(BTN_LEFT, ctx),
			btn_right: self.is_keyboard_input_down(BTN_RIGHT, ctx),

			btn_cross: self.is_keyboard_input_down(BTN_CROSS, ctx),
			btn_square: self.is_keyboard_input_down(BTN_SQUARE, ctx),
			btn_triangle: self.is_keyboard_input_down(BTN_TRIANGLE, ctx),
			btn_circle: self.is_keyboard_input_down(BTN_CIRCLE, ctx),

			btn_l1: self.is_keyboard_input_down(BTN_L1, ctx),
			btn_l2: self.is_keyboard_input_down(BTN_L2, ctx),
			btn_l3: false,

			btn_r1: self.is_keyboard_input_down(BTN_R1, ctx),
			btn_r2: self.is_keyboard_input_down(BTN_R2, ctx),
			btn_r3: false,

			btn_start: self.is_keyboard_input_down(BTN_START, ctx),
			btn_select: self.is_keyboard_input_down(BTN_SELECT, ctx),

			l_stick_x: 0x80,
			l_stick_y: 0x80,
			r_stick_x: 0x80,
			r_stick_y: 0x80,
		}
	}

	pub fn handle_events(&mut self) {
		while let Some(event) = self.gilrs.next_event() {
			match event {
				Event { id, event: EventType::Connected, .. } => {
					if self.active_gamepad.is_none() && self.gilrs.connected_gamepad(id).unwrap().name() != EXCLUDED_CONTROLLER {
						self.active_gamepad = Some(id);
					}
				},
				Event { id, event: EventType::Disconnected, .. } => {
					if let Some(active_id) = self.active_gamepad {
						if id == active_id {
							self.active_gamepad = None;
						}
					}
				},
				_ => {},
			}
		}
	}

	fn is_keyboard_input_down(&mut self, key: Key, ctx: &Context) -> bool {
		ctx.input(|input| input.key_down(key))
	}

	fn get_stick_x(&self, gamepad: &Gamepad, axis: Axis) -> u8 {
		if let Some(data) = gamepad.axis_data(axis) {
			return ((data.value() + 1.0) * 127.5).round() as u8;
		}

		0x80
	}

	fn get_stick_y(&self, gamepad: &Gamepad, axis: Axis) -> u8 {
		if let Some(data) = gamepad.axis_data(axis) {
			return ((data.value() * -1.0 + 1.0) * 127.5).round() as u8;
		}

		0x80
	}

	pub fn show_settings(&mut self, ui: &mut Ui) {
		let controllers: Vec<(GamepadId, Gamepad<'_>)> = self.gilrs.gamepads().filter(|(_, gamepad)| gamepad.name() != EXCLUDED_CONTROLLER).collect();

		let mut selected_controller = "Keyboard".to_string();
		if let Some(id) = self.active_gamepad {
			if let Some(gamepad) = self.gilrs.connected_gamepad(id) {
				selected_controller = gamepad.name().to_string();
			}
		}

		egui::ComboBox::from_label("Connected controller")
			.selected_text(selected_controller)
			.show_ui(ui, |ui| {
				for (gamepad_id, gamepad) in controllers {
					ui.selectable_value(&mut self.active_gamepad, Some(gamepad_id), gamepad.name());
				}

				ui.selectable_value(&mut self.active_gamepad, None, "keyboard")
			});
		
		ui.checkbox(&mut self.analog_enabled, "Analog");
	}
}
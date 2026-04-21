use std::collections::VecDeque;

use log::*;

use crate::{interrupts::Interrupts, scheduler::{EventType, Scheduler, SchedulerEvent}};

/*
serial words:
DTR - Data Terminal Ready
DSR - Data Set Ready / "more-data-request"
RTS - Request To Send
CTS - Clear To Send
*/

/*
Serial transfer loop:
 - write Tx
 - ~1024 cycles later push rx
 - assert ack
 - ~100 cycles later fire IRQ and clear ack
*/

const CONTROLLER_ADDR: usize = 0x1;
const MEMCARD_ADDR: usize = 0x81;

#[derive(PartialEq, Clone, Copy, Debug)]
enum TxState {
	Disabled,
	Ready,
	Transfering { index: u8 }
}

#[derive(Default)]
pub struct InputState {
	pub btn_up: bool,
	pub btn_down: bool,
	pub btn_left: bool,
	pub btn_right: bool,

	pub btn_cross: bool,
	pub btn_square: bool,
	pub btn_triangle: bool,
	pub btn_circle: bool,

	pub btn_l1: bool,
	pub btn_l2: bool,
	pub btn_l3: bool,

	pub btn_r1: bool,
	pub btn_r2: bool,
	pub btn_r3: bool,

	pub btn_select: bool,
	pub btn_start: bool,

	pub l_stick_x: u8,
	pub l_stick_y: u8,
	pub r_stick_x: u8,
	pub r_stick_y: u8,
}

#[derive(Default)]
pub struct ControllerState {
	input_state: InputState,
	analog_enabled: bool,

	config_mode: bool,
	command: u8,
	pub analog_locked: bool,

	enter_config_mode: bool,

	rumble_indexes: [u8; 6],
	old_rumble_indexes: [u8; 6],

	rumble_m1: u8,
	rumble_m2: u8,

	variable_response_ii: u8,
}

impl ControllerState {
	fn new() -> Self {
		Self {
			rumble_indexes: [0xFF; 6],
			old_rumble_indexes: [0xFF; 6],

			..Default::default()
		}
	}

	fn digital_switches_low(&self) -> u8 {
		// invert inputs (0=Pressed, 1=Released)
		!(
			u8::from(self.input_state.btn_select) << 0
				| u8::from(self.input_state.btn_l3 && self.analog_enabled) << 1  // analog only
				| u8::from(self.input_state.btn_r3 && self.analog_enabled) << 2  // analog only
				| u8::from(self.input_state.btn_start) << 3
				| u8::from(self.input_state.btn_up) << 4
				| u8::from(self.input_state.btn_right) << 5
				| u8::from(self.input_state.btn_down) << 6
				| u8::from(self.input_state.btn_left) << 7
		)
	}

	fn digital_switches_high(&self) -> u8 {
		// invert inputs (0=Pressed, 1=Released)
		!(
			u8::from(self.input_state.btn_l2) << 0
				| u8::from(self.input_state.btn_r2) << 1
				| u8::from(self.input_state.btn_l1) << 2
				| u8::from(self.input_state.btn_r1) << 3
				| u8::from(self.input_state.btn_triangle) << 4
				| u8::from(self.input_state.btn_circle) << 5
				| u8::from(self.input_state.btn_cross) << 6
				| u8::from(self.input_state.btn_square) << 7
		)
	}

	pub fn update_input(&mut self, new_state: InputState) {
		self.input_state = new_state;
	}

	pub fn set_analog_enabled(&mut self, set: bool) {
		if !self.analog_locked {
			self.analog_enabled = set;
		}
	}

	pub fn get_rumble(&self) -> (u8, u8) {
		(self.rumble_m1, self.rumble_m2)
	}

	pub fn _tx_reply(&self, index: u8) -> (u8, bool) {
		if self.analog_enabled {
			let reply = match index {
				0 => 0x73,
				1 => 0x5A,
				2 => self.digital_switches_low(),
				3 => self.digital_switches_high(),
				4 => self.input_state.r_stick_x,
				5 => self.input_state.r_stick_y,
				6 => self.input_state.l_stick_x,
				7 => self.input_state.l_stick_y,
				_ => 0,
			};

			return (reply, index < 7);
		} else {
			let reply = match index {
				0 => 0x41,
				1 => 0x5A,
				2 => self.digital_switches_low(),
				3 => self.digital_switches_high(),
				_ => 0,
			};

			return (reply, index < 3);
		}
	}

	pub fn tx_reply(&mut self, index: u8, tx: u8) -> (u8, bool) {
		if index == 0 {
			self.command = tx;

			let id_low = if self.config_mode {
				0xF3
			} else if self.analog_enabled {
				0x73
			} else {
				0x41
			};

			trace!("IDLO: 0x{id_low:X}");

			return (id_low, true);
		} else if index == 1 {
			return (0x5A, true);
		}

		if self.config_mode {
			match self.command {
				// Same as normal mode but always returns analog inputs
				0x42 => {
					self.update_rumble(index, tx);

					let old_analog = self.analog_enabled;
					self.analog_enabled = true;

					let (reply, ack) = self.normal_mode(index);
					self.analog_enabled = old_analog;

					(reply, ack)
				},
				0x43 => self.change_config_mode(index, tx),
				0x44 => self.set_led_state(index, tx),
				0x45 => self.get_led_state(index),
				0x46 => self.variable_response_a(index, tx),
				0x47 => self.unknown_response(index),
				0x4C => self.variable_response_b(index, tx),
				0x4D => self.rumble_protocol(index, tx),
				_ => (0, index < 7),
			}
		} else {
			match self.command {
				0x42 => {
					if self.analog_enabled {
						self.update_rumble(index, tx);
					}
					self.normal_mode(index)
				},
				0x43 if self.analog_enabled => self.change_config_mode(index, tx),
				_ => (0, index < 7),
			}
		}
	}

	fn normal_mode(&self, index: u8) -> (u8, bool) {
		if self.analog_enabled {
			let reply = match index {
				2 => self.digital_switches_low(),
				3 => self.digital_switches_high(),
				4 => self.input_state.r_stick_x,
				5 => self.input_state.r_stick_y,
				6 => self.input_state.l_stick_x,
				7 => self.input_state.l_stick_y,
				_ => 0,
			};

			trace!("analog reply: 0x{reply:X}");

			return (reply, index < 7);
		} else {
			let reply = match index {
				2 => self.digital_switches_low(),
				3 => self.digital_switches_high(),
				_ => 0,
			};

			trace!("digital reply: 0x{reply:X}");

			return (reply, index < 3);
		}
	}

	fn change_config_mode(&mut self, index: u8, tx: u8) -> (u8, bool) {
		if index == 2 {
			self.enter_config_mode = tx == 1;
		}

		let (reply, ack) = if self.config_mode { (0, index < 7) } else { self.normal_mode(index) };

		if index == 7 {
			self.config_mode = self.enter_config_mode;
		}

		debug!("new config mode: {}", self.config_mode);

		(reply, ack)
	}

	fn set_led_state(&mut self, index: u8, tx: u8) -> (u8, bool) {
		match index {
			// Led
			3 => {
				if tx == 0 {
					self.analog_enabled = false;
				} else if tx == 1 {
					self.analog_enabled = true;
				};
			},
			// Key
			4 => {
				if (tx & 3) == 3 {
					self.analog_locked = true;
				} else {
					self.analog_locked = false;
				};
			}
			_ => {},
		}

		(0, index < 7)
	}

	fn get_led_state(&self, index: u8) -> (u8, bool) {
		let reply = match index {
			2 => 0x1,
			3 => 0x2,
			4 => u8::from(self.analog_enabled),
			5 => 0x2,
			6 => 0x1,
			7 => 0x0,
			_ => 0x0,
		};

		(reply, index < 7)
	}

	fn variable_response_a(&mut self, index: u8, tx: u8) -> (u8, bool) {
		if index == 2 {
			self.variable_response_ii = tx;
		}

		let reply = if self.variable_response_ii == 0 {
			match index {
				3 => 0x0,
				4 => 0x1,
				5 => 0x2,
				6 => 0x0,
				7 => 0xA,
				_ => 0,
			}
		} else if self.variable_response_ii == 1 {
			match index {
				3 => 0x00,
				4 => 0x01,
				5 => 0x01,
				6 => 0x01,
				7 => 0x14,
				_ => 0,
			}
		} else {
			0
		};

		(reply, index < 7)
	}

	fn variable_response_b(&mut self, index: u8, tx: u8) -> (u8, bool) {
		if index == 2 {
			self.variable_response_ii = tx;
		}

		let reply = if self.variable_response_ii == 0 && index == 5 {
			0x04
		} else if self.variable_response_ii == 1 && index == 5 {
			0x07
		} else {
			0
		};

		(reply, index < 7)
	}

	fn unknown_response(&self, index: u8) -> (u8, bool) {
		let reply = match index {
			2 => 0x0,
			3 => 0x0,
			4 => 0x2,
			5 => 0x0,
			6 => 0x1,
			7 => 0x0,
			_ => 0x0,
		};

		(reply, index < 7)
	}

	fn rumble_protocol(&mut self, index: u8, tx: u8) -> (u8, bool) {
		self.rumble_m1 = 0;
		self.rumble_m2 = 0;

		self.rumble_indexes[(index - 2) as usize] = tx;

		let old_index = self.old_rumble_indexes[(index - 2) as usize];
		if index == 7 {
			self.old_rumble_indexes = self.rumble_indexes;
		}

		(old_index, index < 7)
	}

	fn update_rumble(&mut self, index: u8, tx: u8) {
		if self.rumble_indexes[(index - 2) as usize] == 0 {
			trace!("rumble M2: 0x{tx:X}");

			// the weak motor is only 0/1
			self.rumble_m2 = (tx & 1) * 0xFF;
		} else if self.rumble_indexes[(index - 2) as usize] == 1 {
			trace!("rumble M1: 0x{tx:X}");

			self.rumble_m1 = tx;
		}
	}
}

pub struct Sio0 {
	pub controller_state: ControllerState,

	rx_fifo: VecDeque<u8>,
	tx_state: TxState,

	tx_enable: bool,
	cs: bool,       // SIO0: chip select (active low), SIO1: data terminal ready (DTR) output level
	rx_enable: bool, // SIO0: 0=only receive when /CS low 1=force receive a single byte,

	tx_ie: bool,     // TX/RX interrupt enable
	rx_ie: bool,     // ^
	rx_int_mode: u8, // 0..3 = IRQ when RX FIFO contains 1,2,4,8 bytes
	ack_ie: bool, // when SIO_STAT.7  ;DSR high or /ACK low (more data request)
	
	port_select: bool, // port 1 / port 2
	
	// stubbed
	sio_mode: u16,
	baudrate: u16,
	
	irq: bool,
	ack: bool,    // acknowledge / "get more data request"
}

impl Sio0 {
	pub fn new() -> Self {
		Self {
			controller_state: ControllerState::new(),

			rx_fifo: VecDeque::new(),
			tx_state: TxState::Disabled,

			tx_enable: false,
			cs: true,
			rx_enable: false,

			tx_ie: false,
			rx_ie: false,
			rx_int_mode: 0,
			ack_ie: false,
			
			port_select: false,
			
			sio_mode: 0,
			baudrate: 0,
			
			irq: false,
			ack: false,
		}
	}

	pub fn read32(&mut self, addr: u32) -> u32 {
		match addr {
			0x1F801040 => self.read_rx(),
			0x1F801044 => self.read_stat(),
			0x1F801048 => self.read_mode().into(),
			0x1F80104A => self.read_ctrl().into(),
			0x1F80104E => self.baudrate.into(),
			_ => unreachable!()
		}
	}

	pub fn write32(&mut self, addr: u32, write: u32, scheduler: &mut Scheduler) {
		match addr {
			0x1F801040 => self.write_tx(write as u8, scheduler),
			0x1F801044 => {},
			0x1F801048 => self.write_mode(write as u16),
			0x1F80104A => self.write_ctrl(write as u16),
			0x1F80104E => self.baudrate = write as u16,
			_ => unreachable!()
		}
	}

	fn read_stat(&self) -> u32 {
		//trace!("Read stat");

		u32::from(self.tx_state != TxState::Disabled)				// TX FIFO Not Full
			| (u32::from(!self.rx_fifo.is_empty()) << 1)		    // RX FIFO Not Empty
			| (u32::from(self.tx_state == TxState::Ready) << 2)		// TX Idle
			| (u32::from(self.ack) << 7)                            // /ACK low
			| (u32::from(self.irq) << 9)                            // IRQ fired
	}

	fn read_mode(&self) -> u16 {
		self.sio_mode
	}

	fn write_mode(&mut self, write: u16) {
		self.sio_mode = write;
	}

	fn read_ctrl(&self) -> u16 {
		trace!("read ctrl");
		
		u16::from(self.tx_enable)
		| u16::from(self.cs) << 1
		| u16::from(self.rx_enable) << 2
		| u16::from(self.rx_int_mode) << 9
		| u16::from(self.tx_ie) << 10
		| u16::from(self.rx_ie) << 11
		| u16::from(self.ack_ie) << 12
		| u16::from(self.port_select) << 13
		
	}

	fn write_ctrl(&mut self, write: u16) {
		trace!("write ctrl 0x{write:X} state: {:?}", self.tx_state);
		
		self.tx_enable = write & 1 != 0;
		
		if self.tx_enable && self.tx_state == TxState::Disabled {
			trace!("enable TX");
			self.tx_state = TxState::Ready;
		} else if self.tx_enable == false {
			trace!("disable TX");
			self.tx_state = TxState::Disabled;
		}
		
		self.cs = (write >> 1) & 1 != 0;
		self.rx_enable = (write >> 2) & 1 != 0;
		
		self.irq = !((write >> 4) != 0);

		if (write >> 6) & 1 != 0 {
			trace!("SIO0 reset");

			self.sio_mode = 0xC;
			self.write_ctrl(0);
			self.rx_fifo.clear();

			return;
		}

		self.rx_int_mode = ((write >> 9) & 3) as u8;
		self.tx_ie = (write >> 10) & 1 != 0;
		self.rx_ie = (write >> 11) & 1 != 0;
		self.ack_ie = (write >> 12) & 1 != 0;

		let old_port = self.port_select;
		self.port_select = (write >> 13) & 1 != 0;

		// reset state machine when CS is low or the selected port is changed
		if !self.cs || self.port_select != old_port {
			trace!("CS: {} PS: {}", self.cs, self.port_select);
			self.rx_fifo.clear();
			self.tx_state = TxState::Ready;
		}
	}

	fn write_tx(&mut self, write: u8, scheduler: &mut Scheduler) {
		trace!("write TX 0x{write:X} (state: {:?})", self.tx_state);

		self.tx_state = match self.tx_state {
			TxState::Disabled => {
				TxState::Disabled
			},
			TxState::Ready => {
				if write as usize == CONTROLLER_ADDR {
					// port 2 is stubbed
					if self.port_select {
						//trace!("communication aborted (ps: {} cs: {})", self.port_select, self.cs);
						self.push_rx(scheduler, 0xFF, false);
						self.ack = false;
						return;
					}
					// reply Hi-Z
					self.push_rx(scheduler, 0, true);
					// fire an interrupt whenever a byte is received by a device
					trace!("start transfer");

				} else if write as usize == MEMCARD_ADDR {
					warn!("tried to read memcard");

					self.push_rx(scheduler, 0xFF, false);
					self.ack = false;
					return;
				}

				TxState::Transfering { index: 0 }
			},
			TxState::Transfering { index } => {
				// TODO for now always assuming the device is a controller
				if index == 0 && !self.controller_state.analog_enabled && write != 0x42 {
					// invalid command, abort transfer
					error!("abort transfer 0x{write:X}");
					self.push_rx(scheduler, 0xFF, false);
					TxState::Ready
				} else {
					let (reply, should_int) = self.controller_state.tx_reply(index, write);

					trace!("write 0x{write:X} controller reply 0x{reply:X} (index: {index}) (int: {should_int})");

					// don't ack bytes past normal communication sequence
					// last byte shouldn't be acknowldge because no more data should be sent (ack = "more-data-request")
					self.push_rx(scheduler, reply, should_int);
					
					TxState::Transfering { index: index + 1 }
				}
			}
		};

	}

	fn read_rx(&mut self) -> u32 {
		if let Some(rx) = self.rx_fifo.pop_front() {
			let pop = rx as u32;

			trace!("read rx 0x{pop:X}");

			pop
		} else {
			trace!("read rx 0");
			0
		}
	}

	fn push_rx(&mut self, scheduler: &mut Scheduler, value: u8, interrupt: bool) {
		scheduler.schedule_event(SchedulerEvent::new(EventType::Sio0Rx(value, interrupt)), 1500);
	}

	pub fn rx_event(&mut self, scheduler: &mut Scheduler, value: u8, interrupt: bool) {
		self.rx_fifo.push_front(value);

		if interrupt {
			self.ack = true;
			scheduler.schedule_event(SchedulerEvent::new(EventType::Sio0Irq), 100);
		}
	}

	pub fn irq_event(&mut self, interrupts: &mut Interrupts) {
		trace!("IRQ7");

		self.ack = false;
		self.irq = true;

		interrupts.raise_interrupt(crate::interrupts::InterruptFlag::Controller);
	}
}

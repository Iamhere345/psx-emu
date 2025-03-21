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
pub struct ControllerState {
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
    pub btn_r1: bool,
    pub btn_r2: bool,

    pub btn_select: bool,
    pub btn_start: bool,
}

impl ControllerState {
    fn new() -> Self {
        Self::default()
    }

    fn digital_switches_low(&self) -> u8 {
        // invert inputs (0=Pressed, 1=Released)
        !(
            u8::from(self.btn_select) << 0
                | u8::from(true) << 1  // analog only
                | u8::from(true) << 2  // analog only
                | u8::from(self.btn_start) << 3
                | u8::from(self.btn_up) << 4
                | u8::from(self.btn_right) << 5
                | u8::from(self.btn_down) << 6
                | u8::from(self.btn_left) << 7
        )
    }

    fn digital_switches_high(&self) -> u8 {
        // invert inputs (0=Pressed, 1=Released)
        !(
            u8::from(self.btn_l2) << 0
                | u8::from(self.btn_r2) << 1
                | u8::from(self.btn_l1) << 2
                | u8::from(self.btn_r1) << 3
                | u8::from(self.btn_triangle) << 4
                | u8::from(self.btn_circle) << 5
                | u8::from(self.btn_cross) << 6
                | u8::from(self.btn_square) << 7
        )
    }
}

pub struct Sio0 {
    pub controller_state: ControllerState,

    rx_fifo: VecDeque<u8>,
    tx_state: TxState,

    tx_enable: bool,
    cs: bool, // SIO0: chip select (active low), SIO1: data terminal ready (DTR) output level
    rx_enable: bool, // SIO0: 0=only receive when /CS low 1=force receive a single byte,

    tx_ie: bool,     // TX/RX interrupt enable
    rx_ie: bool,     // ^
    rx_int_mode: u8, // 0..3 = IRQ when RX FIFO contains 1,2,4,8 bytes
    ack_ie: bool, // when SIO_STAT.7  ;DSR high or /ACK low (more data request)
    
    port_select: bool, // port 1 / port 2
    
    // stubbed
    sio_mode: u16,
    
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
            0x1F80104E => { info!("Unhandled read to SIO0_BAUD"); 0 },
            _ => unreachable!()
        }
    }

    pub fn write32(&mut self, addr: u32, write: u32, scheduler: &mut Scheduler) {
        match addr {
            0x1F801040 => self.write_tx(write as u8, scheduler),
            0x1F801044 => {},
            0x1F801048 => self.write_mode(write as u16),
            0x1F80104A => self.write_ctrl(write as u16),
            0x1F80104E => info!("Unhandled write to SIO0_BAUD"),
            _ => unreachable!()
        }
    }

    fn read_stat(&self) -> u32 {
        //trace!("Read stat");

        u32::from(self.tx_state != TxState::Disabled)				// TX FIFO Not Full
			| (u32::from(!self.rx_fifo.is_empty()) << 1)		// RX FIFO Not Empty
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
        
        self.tx_enable = write & 1 != 0;
        
        if self.tx_enable && self.tx_state == TxState::Disabled {
            self.tx_state = TxState::Ready;
        } else if self.tx_enable == false {
            self.tx_state = TxState::Disabled;
        }
        
        trace!("write ctrl 0x{write:X} state: {:?}", self.tx_state);

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
        self.port_select = (write >> 13) & 1 != 0;
    }

    fn write_tx(&mut self, write: u8, scheduler: &mut Scheduler) {
        trace!("write TX 0x{write:X} (state: {:?})", self.tx_state);

		self.tx_state = match self.tx_state {
            TxState::Disabled => {
                TxState::Disabled
            },
            TxState::Ready => {
                if write as usize == CONTROLLER_ADDR {
                    // reply Hi-Z
                    self.push_rx(scheduler, 0, true);
                    // fire an interrupt whenever a byte is received by a device
                    trace!("start transfer");

                } else if write as usize == MEMCARD_ADDR {
                    self.push_rx(scheduler, 0xFF, false);
                    self.ack = false;
                    return;
                }

                TxState::Transfering { index: 0 }
            },
            TxState::Transfering { index } => {
                // TODO for now always assuming the device is a controller
                if index == 0 && write != 0x42 {
                    // invalid command, abort transfer
                    error!("abort transfer 0x{write:X}");
                    self.push_rx(scheduler, 0xFF, false);
                    TxState::Ready
                } else {
                    let reply = match index {
                        0 => 0x41,
                        1 => 0x5A,
                        2 => self.controller_state.digital_switches_low(),
                        3 => self.controller_state.digital_switches_high(),
                        _ => 0,
                    };

                    trace!("controller reply 0x{reply:X} (index: {index})");

                    // don't ack bytes past normal communication sequence
                    // last byte shouldn't be acknowldge because no more data should be sent (ack = "more-data-request")
                    self.push_rx(scheduler, reply, index < 3);
                    
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
        debug!("IRQ7");

        self.ack = false;
        self.irq = true;

        interrupts.raise_interrupt(crate::interrupts::InterruptFlag::Controller);
    }
}

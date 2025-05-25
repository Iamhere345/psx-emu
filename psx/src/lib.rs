use cpu::R3000;
use bus::Bus;
use scheduler::{EventType, Scheduler, SchedulerEvent};
use cdrom::disc::Disc;

pub mod cpu;
mod gpu;
mod dma;
pub mod cdrom;
mod interrupts;
mod timers;
mod sio0;
mod scheduler;
pub mod bus;

pub struct PSXEmulator {
	pub cpu: R3000,
	pub bus: Bus,
	pub scheduler: Scheduler,

	out_vram: Box<[u16]>,
}

impl PSXEmulator {
	pub fn new(bios: Vec<u8>) -> Self {
		let mut psx = Self {
			cpu: R3000::new(),
			bus: Bus::new(bios),
			scheduler: Scheduler::new(),

			out_vram: vec![0; 512 * 2048].into_boxed_slice().try_into().unwrap(),
		};

		psx.scheduler.schedule_event(SchedulerEvent::new(scheduler::EventType::Vblank), 571212);

		psx
	}

	pub fn tick(&mut self) {
		if self.scheduler.next_event_ready() {
			let event = self.scheduler.pop_event();
			self.scheduler.handle_event(event.clone(), &mut self.bus);

			if event.event_type == EventType::Vblank {
				self.out_vram = self.bus.gpu.vram.clone();
			}
		}

		self.cpu.run_instruction(&mut self.bus, &mut self.scheduler);
		self.scheduler.tick_scheduler(2);
	}

	pub fn run_frame(&mut self) {
		
		loop {
			while !self.scheduler.next_event_ready() {
				self.cpu.run_instruction(&mut self.bus, &mut self.scheduler);

				self.scheduler.tick_scheduler(2);
			}

			let last_event = self.scheduler.pop_event();

			self.scheduler.handle_event(last_event.clone(), &mut self.bus);

			if last_event.event_type == EventType::Vblank {
				break;
			}

		}

		self.out_vram = self.bus.gpu.vram.clone();
	}

	pub fn load_disc(&mut self, disc: Disc) {
		self.bus.cdrom.load_disc(disc);
	}

	pub fn update_input(
		&mut self,

		up: bool,
		down: bool,
		left: bool,
		right: bool,

		cross: bool,
		square: bool,
		triangle: bool,
		circle: bool,

		l1: bool,
		l2: bool,
		r1: bool,
		r2: bool,

		start: bool,
		select: bool,
	) {
		let ctx = &mut self.bus.sio0.controller_state;

		ctx.btn_up = up;
		ctx.btn_down = down;
		ctx.btn_left = left;
		ctx.btn_right = right;

		ctx.btn_cross = cross;
		ctx.btn_square = square;
		ctx.btn_triangle = triangle;
		ctx.btn_circle = circle;

		ctx.btn_l1 = l1;
		ctx.btn_l2 = l2;
		ctx.btn_r1 = r1;
		ctx.btn_r2 = r2;

		ctx.btn_start = start;
		ctx.btn_select = select;
	}

	// from https://jsgroth.dev/blog/posts/ps1-sideloading/
	pub fn sideload_exe(&mut self, exe: Vec<u8>) {

		// Wait for the BIOS to jump to the shell
		while self.cpu.pc != 0x80030000 {
			self.tick();
		}

		// Parse EXE header
		let initial_pc = u32::from_le_bytes(exe[0x10..0x14].try_into().unwrap());
		let initial_r28 = u32::from_le_bytes(exe[0x14..0x18].try_into().unwrap());
		let exe_ram_addr = u32::from_le_bytes(exe[0x18..0x1C].try_into().unwrap()) & 0x1FFFFF;
		let exe_size = u32::from_le_bytes(exe[0x01C..0x020].try_into().unwrap());
		let initial_sp = u32::from_le_bytes(exe[0x30..0x34].try_into().unwrap());

		// Copy EXE code/data into PS1 RAM
		self.bus.ram[exe_ram_addr as usize..(exe_ram_addr + exe_size) as usize]
			.copy_from_slice(&exe[2048..2048 + exe_size as usize]);

		// Set initial register values
		self.cpu.registers.write_gpr(28, initial_r28);
		if initial_sp != 0 {
			self.cpu.registers.write_gpr(29, initial_sp);
			self.cpu.registers.write_gpr(30, initial_sp);
		}

		// Jump to the EXE entry point; execution can continue normally after this
		self.cpu.pc = initial_pc;

	}

	pub fn get_vram(&self) -> &Box<[u16]> {
		&self.out_vram
	}

	pub fn get_tty_buf(&mut self) -> String {
		let old_buf = self.cpu.tty_buf.clone();

		self.cpu.tty_buf = String::new();

		old_buf
	}
}
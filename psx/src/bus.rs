#![allow(unused_variables)]
use log::*;

use crate::cdrom::Cdrom;
use crate::gpu::Gpu;
use crate::dma::DmaController;
use crate::interrupts::Interrupts;
use crate::scheduler::Scheduler;
use crate::sio0::Sio0;
use crate::spu::Spu;
use crate::timers::Timers;

const BIOS_START: usize = 0x1FC00000;
const BIOS_END: usize = BIOS_START + (512 * 1024);

const RAM_START: usize = 0x0;
const RAM_END: usize = 0x7FFFFF;
const RAM_SIZE: usize = 0x1FFFFF;

const SCRATCHPAD_START: usize = 0x1F800000;
const SCRATCHPAD_END: usize = 0x1F8003FF;

const MEMCONTROL_START: usize = 0x1F801000;
const MEMCONTROL_END: usize = 0x1F801000 + 36;

const RAM_SIZE_START: usize = 0x1F801060;
const RAM_SIZE_END: usize = 0x1F801064;

const IRQ_START: usize = 0x1F801070;
const IRQ_END: usize = 0x1F801074;

const SPU_START: usize = 0x1F801C00;
const SPU_END: usize = 0x1F801E80;

const TIMERS_START: usize = 0x1F801100;
const TIMERS_END: usize = 0x1F80112F;

const DMA_START: usize = 0x1F801080;
const DMA_END: usize = DMA_START + 0x80 - 1;

const GPU_START: usize = 0x1F801810;
const GPU_END: usize = 0x1F801814;

const EXPANSION1_START: usize = 0x1F000000;
const EXPANSION1_END: usize = 0x1F080000;

const EXPANSION2_START: usize = 0x1F802000;
const EXPANSION2_END: usize = EXPANSION2_START + 0x42;

const PAD_START: usize = 0x1F801040;
const PAD_END: usize = 0x1F80104E;

const SIO1_START: usize = 0x1F801050;
const SIO1_END: usize = 0x1F80105E;

const CDROM_START: usize = 0x1F801800;
const CDROM_END: usize = 0x1F801803;

const MDEC_START: usize = 0x1F801820;
const MDEC_END: usize = 0x1F801824;

const REDUX_START: usize = 0x1F802080;
const REDUX_END: usize = 0x1F802084;

const REGION_MASK: [u32; 8] = [
	// KUSEG 2048Mb
	0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 
	// KSEG0 512Mb
	0x7FFFFFFF,
	// KSEG1 512Mb
	0x1FFFFFFF,
	// KSEG2 1024Mb
	0xFFFFFFFF, 0xFFFFFFFF
];

pub struct Bus {
	bios: Vec<u8>,
	pub ram: Vec<u8>,
	scratchpad: Vec<u8>,

	pub gpu: Gpu,
	pub dma: DmaController,
	pub cdrom: Cdrom,
	pub interrupts: Interrupts,
	pub timers: Timers,
	pub sio0: Sio0,
	pub spu: Spu,

	pub read_breakpoints: Vec<u32>,
	pub write_breakpoints: Vec<u32>,
	pub breakpoint_hit: (bool, u32),
}

fn mask_addr(addr: u32) -> u32 {
	addr & REGION_MASK[(addr >> 29) as usize]
}

impl Bus {
	pub fn new(bios: Vec<u8>) -> Self {
		Self {
			bios: bios,
			ram: vec![0xFA; 2048 * 1024],
			scratchpad: vec![0xBA; 1024],

			gpu: Gpu::new(),
			dma: DmaController::new(),
			cdrom: Cdrom::new(),
			interrupts: Interrupts::new(),
			timers: Timers::new(),
			sio0: Sio0::new(),
			spu: Spu::new(),

			read_breakpoints: Vec::new(),
			write_breakpoints: Vec::new(),
			breakpoint_hit: (false, 0),
		}
	}

	pub fn read8(&mut self, unmasked_addr: u32, scheduler: &mut Scheduler) -> u8 {
		
		let addr = mask_addr(unmasked_addr);

		match addr as usize {
			BIOS_START			..=	BIOS_END => self.bios[addr as usize - BIOS_START],
			RAM_START			..= RAM_END => self.ram[(addr as usize) & RAM_SIZE - RAM_START],
			SCRATCHPAD_START	..= SCRATCHPAD_END => self.scratchpad[addr as usize - SCRATCHPAD_START],

			EXPANSION1_START	..= EXPANSION1_END => { debug!("read to expansion 1 register 0x{:X}", unmasked_addr); 0xFF },
			EXPANSION2_START	..= EXPANSION2_END => { debug!("read to expansion 2 register 0x{:X}", unmasked_addr); 0 },
			MEMCONTROL_START	..= MEMCONTROL_END => 0,
			RAM_SIZE_START		..= RAM_SIZE_END => 0,

			SPU_START			..= SPU_END => self.spu.read16(addr) as u8,
			TIMERS_START		..= TIMERS_END =>{ error!("[0x{addr:X}] timer read8"); self.timers.read32(addr, scheduler) as u8},
			GPU_START			..= GPU_END => 0,
			DMA_START			..= DMA_END => self.dma.read8(addr),
			PAD_START 			..= PAD_END => self.sio0.read32(addr) as u8,
			SIO1_START			..= SIO1_END => { warn!("[0x{addr:X}] Unhandled SIO1 read8"); 0 }
			CDROM_START			..= CDROM_END => self.cdrom.read8(addr),
			REDUX_START			..= REDUX_END => 0,

			_ => panic!("unhandled read8 0x{:X}", addr)
		}

	}

	pub fn read16(&mut self, unmasked_addr: u32, scheduler: &mut Scheduler) -> u16 {

		let addr = mask_addr(unmasked_addr);

		match addr as usize {
			BIOS_START	..= BIOS_END => u16::from_le_bytes([
				self.read8(unmasked_addr, scheduler),
				self.read8(unmasked_addr + 1, scheduler)
			]),
			RAM_START	..= RAM_END => u16::from_le_bytes([
				self.read8(unmasked_addr, scheduler),
				self.read8(unmasked_addr + 1, scheduler)
			]),
			SCRATCHPAD_START	..= SCRATCHPAD_END => u16::from_le_bytes([
				self.read8(unmasked_addr, scheduler),
				self.read8(unmasked_addr + 1, scheduler)
			]),
			IRQ_START	..= IRQ_END => self.interrupts.read32(addr) as u16,
			SPU_START	..= SPU_END => self.spu.read16(addr),
			PAD_START	..= PAD_END => self.sio0.read32(addr) as u16,
			SIO1_START			..= SIO1_END => { warn!("[0x{addr:X}] Unhandled SIO1 read16"); 0 }
			TIMERS_START..= TIMERS_END => self.timers.read32(addr, scheduler) as u16,

			_ => panic!("unhandled read16 0x{addr:X}/0x{unmasked_addr:X}"),
		}

	}

	pub fn read32(&mut self, unmasked_addr: u32, scheduler: &mut Scheduler) -> u32 {
		if unmasked_addr % 4 != 0 {
			panic!("unaligned 32 bit read at addr 0x{:X}", unmasked_addr);
		}
		
		let addr = mask_addr(unmasked_addr);

		match addr as usize {
			GPU_START			..= GPU_END => self.gpu.read32(addr),
			DMA_START			..= DMA_END => self.dma.read32(addr),
			MEMCONTROL_START	..= MEMCONTROL_END =>  {warn!("[{addr:X}] Unhandled read from memcontrol"); 0 },
			IRQ_START			..= IRQ_END => self.interrupts.read32(addr),
			TIMERS_START		..= TIMERS_END => self.timers.read32(addr, scheduler),
			SPU_START			..= SPU_END => self.spu.read32(addr),
			MDEC_START			..= MDEC_END => { warn!("[{addr:X}] Unhandled read to MDEC"); 0 },

			_ => u32::from_le_bytes([
				self.read8(addr, scheduler),
				self.read8(addr + 1, scheduler),
				self.read8(addr + 2, scheduler),
				self.read8(addr + 3, scheduler),
			]),
		}
		
	}

	pub fn read32_debug(&self, unmasked_addr: u32) -> u32 {
		if unmasked_addr % 4 != 0 {
			panic!("unaligned 32 bit read at addr 0x{:X}", unmasked_addr);
		}
		
		let addr = mask_addr(unmasked_addr);

		match addr as usize {
			DMA_START			..= DMA_END => self.dma.read32(addr),
			IRQ_START			..= IRQ_END => self.interrupts.read32(addr),
			_ => u32::from_le_bytes([
				self.read8_debug(addr),
				self.read8_debug(addr + 1),
				self.read8_debug(addr + 2),
				self.read8_debug(addr + 3),
			]),
		}
		
	}

	pub fn read8_debug(&self, unmasked_addr: u32) -> u8 {
		
		let addr = mask_addr(unmasked_addr);

		match addr as usize {
			BIOS_START			..=	BIOS_END => self.bios[addr as usize - BIOS_START],
			RAM_START			..= RAM_END => self.ram[addr as usize - RAM_START],
			SCRATCHPAD_START	..= SCRATCHPAD_END => self.scratchpad[addr as usize - SCRATCHPAD_START],

			_ => 0xDE
		}

	}

	pub fn write8(&mut self, unmasked_addr: u32, write: u8, scheduler: &mut Scheduler) {

		let addr = mask_addr(unmasked_addr);

		match addr as usize {
			RAM_START			..= RAM_END => self.ram[(addr as usize) & RAM_SIZE - RAM_START] = write,
			SCRATCHPAD_START	..= SCRATCHPAD_END => self.scratchpad[addr as usize -  SCRATCHPAD_START] = write,

			SPU_START			..= SPU_END => self.spu.write16(addr, write.into()),
			TIMERS_START		..= TIMERS_END => { error!("write8 to timers [0x{addr:X} 0x{write:X}"); self.timers.write32(addr, write as u32, scheduler); },
			EXPANSION2_START	..= EXPANSION2_END => debug!("write to expansion 2 register [0x{addr:X}] 0x{write:X}. Ignoring."),
			CDROM_START			..= CDROM_END => self.cdrom.write8(addr, write, scheduler),
			PAD_START			..= PAD_END => self.sio0.write32(addr, write.into(), scheduler),
			SIO1_START			..= SIO1_END => warn!("[0x{addr:X}] Unhandled SIO1 write8 0x{write:X}"),
			DMA_START			..= DMA_END => self.dma.write8(addr, write),
			REDUX_START 		..= REDUX_END => match addr {
				0x1F802080 => print!("{}", char::from_u32(write as u32).unwrap_or('?')),
				_ => {},
			}

			_ => panic!("unhandled write8 [0x{:X}] 0x{:X}", addr, write)
		}
	}

	pub fn write16(&mut self, unmasked_addr: u32, write: u16, scheduler: &mut Scheduler) {

		if unmasked_addr % 2 != 0 {
			panic!("unaligned 16 bit write [0x{unmasked_addr:X}] 0x{write:X}");
		}

		let [lsb, msb] = write.to_le_bytes();

		let addr = mask_addr(unmasked_addr);

		match addr as usize {
			IRQ_START		..= IRQ_END => self.interrupts.write32(addr, write as u32),
			SPU_START		..=	SPU_END => self.spu.write16(addr, write),
			TIMERS_START	..= TIMERS_END => self.timers.write32(addr, write as u32, scheduler),
			PAD_START 		..= PAD_END => self.sio0.write32(addr, write.into(), scheduler),
			SIO1_START		..= SIO1_END => warn!("[0x{addr:X}] Unhandled SIO1 write16 0x{write:X}"),
			
			RAM_START		..= RAM_END => {
				self.write8(unmasked_addr, lsb, scheduler);
				self.write8(unmasked_addr + 1, msb, scheduler);
			}
			SCRATCHPAD_START..= SCRATCHPAD_END => {
				self.write8(unmasked_addr, lsb, scheduler);
				self.write8(unmasked_addr + 1, msb, scheduler);
			},

			_ => panic!("[0x{unmasked_addr:X}] write16 0x{write:X}"),
		}	

	}

	pub fn write32(&mut self, unmasked_addr: u32, write: u32, scheduler: &mut Scheduler) {

		if unmasked_addr % 4 != 0 {
			panic!("unaligned 32 bit write [0x{unmasked_addr:X}] 0x{write:X}");
		}

		let addr = mask_addr(unmasked_addr);

		match addr as usize {
			RAM_START			..= RAM_END => {
				self.write8(addr + 0, (write >> 0) as u8, scheduler);
				self.write8(addr + 1, (write >> 8) as u8, scheduler);
				self.write8(addr + 2, (write >> 16) as u8, scheduler);
				self.write8(addr + 3, (write >> 24) as u8, scheduler);
			},
			SCRATCHPAD_START	..= SCRATCHPAD_END => {
				self.write8(addr + 0, (write >> 0) as u8, scheduler);
				self.write8(addr + 1, (write >> 8) as u8, scheduler);
				self.write8(addr + 2, (write >> 16) as u8, scheduler);
				self.write8(addr + 3, (write >> 24) as u8, scheduler);
			},

			MEMCONTROL_START	..=	MEMCONTROL_END => {
				match addr as usize - MEMCONTROL_START {
					0 => if write != 0x1F000000 { panic!("write to expansion 1 base addr 0x{:X}", write) },
					4 => if write != 0x1F802000 { panic!("write to expansion 2 base addr 0x{:X}", write) },
					_ => {}//debug!("unhandled write to memcontrol [0x{:X}] 0x{write:X}", addr as usize),
				}
			}
			IRQ_START			..= IRQ_END => self.interrupts.write32(addr, write),
			TIMERS_START		..= TIMERS_END => self.timers.write32(addr, write, scheduler),
			// io register RAM_SIZE
			0x1F801060			..= 0x1F801064 => {},
			// io register CACHE_CONTROL
			0xFFFE0130			..= 0xFFFE0134 => {},
			DMA_START			..= DMA_END => {
				match addr {
					0x1F801080	..= 0x1F8010EF => {
						self.dma.write32(addr, write);

						let channel = &self.dma.channels[((addr >> 0x4) & 0x7) as usize];

						if channel.active() {
							trace!("triggered DMA{}", channel.channel_num);
							self.do_dma(channel.channel_num, scheduler);
						}
					},
					_ => self.dma.write32(addr, write),
				}
			},
			GPU_START			..= GPU_END => self.gpu.write32(addr, write),
			SPU_START			..= SPU_END => self.spu.write32(addr, write),
			MDEC_START			..= MDEC_END => warn!("[0x{addr:X}] Unhandled write to MDEC 0x{write:X}"),
			REDUX_START			..= REDUX_END => {},

			_ => panic!("unhandled write32 [0x{:X}/0x{:X}] 0x{:X}", addr, unmasked_addr, write)
		}

	}
}
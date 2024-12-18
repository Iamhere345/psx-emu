use log::*;

use crate::gpu::Gpu;
use crate::dma::DmaController;

const BIOS_START: usize = 0x1FC00000;
const BIOS_END: usize = BIOS_START + (512 * 1024);

const RAM_START: usize = 0x0;
const RAM_END: usize = RAM_START + (2048 * 1024);

const SCRATCHPAD_START: usize = 0x1F800000;
const SCRATCHPAD_END: usize = 0x1F8003FF;

const MEMCONTROL_START: usize = 0x1F801000;
const MEMCONTROL_END: usize = 0x1F801000 + 36;

const IRQ_START: usize = 0x1F801070;
const IRQ_END: usize = IRQ_START + 8;

const SPU_START: usize = 0x1F801C00;
const SPU_END: usize = SPU_START + 0x280;

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
}

fn mask_addr(addr: u32) -> u32 {
	addr & REGION_MASK[(addr >> 29) as usize]
}

impl Bus {
	pub fn new(bios: Vec<u8>) -> Self {
		Self {
			bios: bios,
			ram: vec![0xDA; 2048 * 1024],
			scratchpad: vec![0xBA; 1024],

			gpu: Gpu::new(),
			dma: DmaController::new(),
		}
	}

	pub fn read8(&self, unmasked_addr: u32) -> u8 {
		
		let addr = mask_addr(unmasked_addr);

		match addr as usize {
			BIOS_START			..=	BIOS_END => self.bios[addr as usize - BIOS_START],
			RAM_START			..= RAM_END => self.ram[addr as usize - RAM_START],
			SCRATCHPAD_START	..SCRATCHPAD_END => self.scratchpad[addr as usize - SCRATCHPAD_START],

			EXPANSION1_START	..= EXPANSION1_END => {info!("read to expansion 1 register 0x{:X}", unmasked_addr); 0xFF},
			EXPANSION2_START	..= EXPANSION2_END => {info!("read to expansion 2 register 0x{:X}", unmasked_addr); 0},

			SPU_START			..= SPU_END => {info!("read to SPU register 0x{:X}", unmasked_addr); 0},
			TIMERS_START		..= TIMERS_END => 0,
			IRQ_START			..= IRQ_END => 0,
			GPU_START			..= GPU_END => 0,
			PAD_START 			..= PAD_END => 0,

			_ => panic!("unhandled read8 0x{:X}", addr)
		}

	}

	pub fn read16(&self, unmasked_addr: u32) -> u16 {

		let addr = mask_addr(unmasked_addr);

		match addr as usize {
			RAM_START	..= RAM_END => u16::from_le_bytes([
				self.read8(unmasked_addr),
				self.read8(unmasked_addr + 1)
			]),
			SCRATCHPAD_START	..= SCRATCHPAD_END => u16::from_le_bytes([
				self.read8(unmasked_addr),
				self.read8(unmasked_addr + 1)
			]),
			
			IRQ_START	..= IRQ_END => 0,
			SPU_START	..= SPU_END => 0,
			PAD_START	..= PAD_END => 0,
			TIMERS_START..= TIMERS_END => 0,

			_ => panic!("unhandled read16 0x{:X}", addr),
		}

	}

	pub fn read32(&mut self, addr: u32) -> u32 {

		if addr % 4 != 0 {
			panic!("unaligned 32 bit read at addr 0x{:X}", addr);
		}

		let masked_addr = mask_addr(addr);

		match masked_addr as usize {
			GPU_START	..= GPU_END => self.gpu.read32(addr),
			DMA_START	..= DMA_END => self.dma.read32(addr),
			MEMCONTROL_START	..= MEMCONTROL_END => 0,
			_ => u32::from_le_bytes([
				self.read8(addr),
				self.read8(addr + 1),
				self.read8(addr + 2),
				self.read8(addr + 3),
			]),
		}
		
	}

	pub fn write8(&mut self, unmasked_addr: u32, write: u8) {

		let addr = mask_addr(unmasked_addr);

		match addr as usize {
			RAM_START			..= RAM_END => self.ram[addr as usize - RAM_START] = write,
			SCRATCHPAD_START	..= SCRATCHPAD_END => self.scratchpad[addr as usize -  SCRATCHPAD_START] = write,

			SPU_START			..= SPU_END => info!("write to SPU register [0x{addr:X}] 0x{write:X}. Ignoring."),
			TIMERS_START		..= TIMERS_END => info!("Unhandled write to timers [0x{addr:X} 0x{write:X}. Ignoring"),
			EXPANSION2_START	..= EXPANSION2_END => info!("write to expansion 2 register [0x{addr:X}] 0x{write:X}. Ignoring."),
			_ => panic!("unhandled write8 [0x{:X}] 0x{:X}", addr, write)
		}
	}

	pub fn write16(&mut self, unmasked_addr: u32, write: u16) {

		if unmasked_addr % 2 != 0 {
			panic!("unaligned 16 bit write [0x{unmasked_addr:X}] 0x{write:X}");
		}

		let [lsb, msb] = write.to_le_bytes();

		let addr = mask_addr(unmasked_addr);

		match addr as usize {
			IRQ_START		..= IRQ_END => {}
			SPU_START		..=	SPU_END => {}
			TIMERS_START	..= TIMERS_END => {}
			PAD_START ..= PAD_END => {},
			
			RAM_START		..= RAM_END => {
				self.write8(unmasked_addr, lsb);
				self.write8(unmasked_addr + 1, msb);
			}
			SCRATCHPAD_START		..= SCRATCHPAD_END => {
				self.write8(unmasked_addr, lsb);
				self.write8(unmasked_addr + 1, msb);
			},

			_ => panic!("[0x{unmasked_addr:X}] write16 0x{write:X}"),
		}	

	}

	pub fn write32(&mut self, unmasked_addr: u32, write: u32) {

		if unmasked_addr % 4 != 0 {
			panic!("unaligned 32 bit write [0x{unmasked_addr:X}] 0x{write:X}");
		}

		let addr = mask_addr(unmasked_addr);

		match addr as usize {
			RAM_START			..= RAM_END => {
				self.write8(addr + 0, (write >> 0) as u8);
				self.write8(addr + 1, (write >> 8) as u8);
				self.write8(addr + 2, (write >> 16) as u8);
				self.write8(addr + 3, (write >> 24) as u8);
			},
			SCRATCHPAD_START			..= SCRATCHPAD_END => {
				self.write8(addr + 0, (write >> 0) as u8);
				self.write8(addr + 1, (write >> 8) as u8);
				self.write8(addr + 2, (write >> 16) as u8);
				self.write8(addr + 3, (write >> 24) as u8);
			},

			MEMCONTROL_START	..=	MEMCONTROL_END => {
				match addr as usize - MEMCONTROL_START {
					0 => if write != 0x1F000000 { panic!("write to expansion 1 base addr 0x{:X}", write) },
					1 => if write != 0x1F802000 { panic!("write to expansion 2 base addr 0x{:X}", write) },
					_ => info!("unhandled write to memcontrol [0x{:X}] 0x{write:X}", addr as usize - MEMCONTROL_START),
				}
			}
			IRQ_START			..= IRQ_END => info!("Unhandled write to IRQ register [0x{:X}] 0x{:X}", addr, write),
			TIMERS_START		..= TIMERS_END => {},
			// io register RAM_SIZE
			0x1F801060	..= 0x1F801064 => {},
			// io register CACHE_CONTROL
			0xFFFE0130	..= 0xFFFE0134 => {},
			DMA_START	..= DMA_END => {
				match addr {
					0x1F801080	..= 0x1F8010EF => {
						self.dma.write32(addr, write);

						let channel = &self.dma.channels[((addr >> 0x4) & 0x7) as usize];

						if channel.active() {
							trace!("triggered DMA{}", channel.channel_num);
							self.do_dma(channel.channel_num);
						}
					},
					_ => self.dma.write32(addr, write),
				}
			},
			GPU_START	..= GPU_END => self.gpu.write32(addr, write),

			_ => panic!("unhandled write32 [0x{:X}/0x{:X}] 0x{:X}", addr, unmasked_addr, write)
		}

	}
}
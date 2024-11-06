const BIOS_START: usize = 0x1FC00000;
const BIOS_END: usize = BIOS_START + (512 * 1024);

const RAM_START: usize = 0x0;
const RAM_END: usize = RAM_START + (2048 * 1024);

const MEMCONTROL_START: usize = 0x1F801000;
const MEMCONTROL_END: usize = 0x1F801000 + 36;

const IRQ_START: usize = 0x1F801070;
const IRQ_END: usize = IRQ_START + 8;

const SPU_START: usize = 0x1F801C00;
const SPU_END: usize = SPU_START + 0x280;

const TIMERS_START: usize = 0x1F801100;
const TIMERS_END: usize = 0x1F80112F;

const EXPANSION1_START: usize = 0x1F000000;
const EXPANSION1_END: usize = 0x1F080000;

const EXPANSION2_START: usize = 0x1F802000;
const EXPANSION2_END: usize = EXPANSION2_START + 0x42;

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
	ram: Vec<u8>,
}

fn mask_addr(addr: u32) -> u32 {
	addr & REGION_MASK[(addr >> 29) as usize]
}

impl Bus {
	pub fn new(bios: Vec<u8>) -> Self {
		Self {
			bios: bios,
			ram: vec![0xF0; 2048 * 1024],
		}
	}

	pub fn read8(&self, unmasked_addr: u32) -> u8 {
		
		let addr = mask_addr(unmasked_addr);

		match addr as usize {
			BIOS_START	..=	BIOS_END => self.bios[addr as usize - BIOS_START],
			RAM_START	..= RAM_END => self.ram[addr as usize - RAM_START],

			EXPANSION1_START	..= EXPANSION1_END => {/* println!("read to expansion 1 register 0x{:X}", unmasked_addr); */ 0xFF},

			EXPANSION2_START	..= EXPANSION2_END => {/* println!("read to expansion 2 register 0x{:X}", unmasked_addr); */ 0},
			SPU_START	..= SPU_END => {/* println!("read to SPU register 0x{:X}", unmasked_addr); */ 0},
			TIMERS_START	..= TIMERS_END => 0,

			IRQ_START	..= IRQ_END => 0,
			_ => panic!("unhandled read8 0x{:X}", addr)
		}

	}

	pub fn read32(&self, addr: u32) -> u32 {

		if addr % 4 != 0 {
			panic!("unaligned 32 bit read at addr 0x{:X}", addr);
		}

		u32::from_le_bytes([
			self.read8(addr),
			self.read8(addr + 1),
			self.read8(addr + 2),
			self.read8(addr + 3),
		])
	}

	pub fn write8(&mut self, unmasked_addr: u32, write: u8) {

		let addr = mask_addr(unmasked_addr);

		match addr as usize {
			RAM_START			..= RAM_END => self.ram[addr as usize - RAM_START] = write,

			SPU_START			..= SPU_END => {},//println!("write to SPU register [0x{:X}] 0x{:X}. Ignoring.", unmasked_addr, write),
			TIMERS_START		..= TIMERS_END => {},
			EXPANSION2_START	..= EXPANSION2_END => {},//println!("write to expansion 2 register [0x{:X}] 0x{:X}. Ignoring.", unmasked_addr, write),
			_ => panic!("unhandled write8 [0x{:X}] 0x{:X}", addr, write)
		}
	}

	pub fn write16(&mut self, unmasked_addr: u32, write: u16) {

		if unmasked_addr % 2 != 0 {
			panic!("unaligned 16 bit write [0x{:X}] 0x{:X}", unmasked_addr, write);
		}

		let [lsb, msb] = write.to_le_bytes();

		let addr = mask_addr(unmasked_addr);

		match addr as usize {
			SPU_START		..=	SPU_END => {}
			TIMERS_START	..= TIMERS_END => {}
			RAM_START		..= RAM_END => {
				self.write8(unmasked_addr, lsb);
				self.write8(unmasked_addr + 1, msb);
			}
			_ => panic!("[0x{:X}] write16 0x{:X}", unmasked_addr, write),
		}	

	}

	pub fn write32(&mut self, unmasked_addr: u32, write: u32) {

		if unmasked_addr % 4 != 0 {
			panic!("unaligned 32 bit write [0x{:X}] 0x{:X}", unmasked_addr, write);
		}

		let addr = mask_addr(unmasked_addr);

		match addr as usize {
			RAM_START			..= RAM_END => {
				self.write8(addr + 0, (write >> 0) as u8);
				self.write8(addr + 1, (write >> 8) as u8);
				self.write8(addr + 2, (write >> 16) as u8);
				self.write8(addr + 3, (write >> 24) as u8);
			},

			MEMCONTROL_START	..=	MEMCONTROL_END => {
				match addr as usize - MEMCONTROL_START {
					0 => if write != 0x1F000000 { panic!("write to expansion 1 base addr 0x{:X}", write) },
					1 => if write != 0x1F802000 { panic!("write to expansion 2 base addr 0x{:X}", write) },
					_ => {}//println!("unhandled write to memcontrol [0x{:X}] 0x{:X}", addr as usize - MEMCONTROL_START, write),
				}
			}
			IRQ_START			..= IRQ_END => {},//println!("Unhandled write to IRQ register [0x{:X}] 0x{:X}", addr, write),
			// io register RAM_SIZE
			0x1F801060	..= 0x1F801064 => {}
			// io register CACHE_CONTROL
			0xFFFE0130	..= 0xFFFE0134 => {}
			_ => panic!("unhandled write32 [0x{:X}/0x{:X}] 0x{:X}", addr, unmasked_addr, write)
		}

	}
}
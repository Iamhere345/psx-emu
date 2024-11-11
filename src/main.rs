use cpu::R3000;
use bus::Bus;

mod cpu;
pub mod bus;

struct Emulator {
    cpu: R3000,
    bus: Bus,
}

impl Emulator {
    fn new(bios: Vec<u8>) -> Self {
        Self {
            cpu: R3000::new(),
            bus: Bus::new(bios)
        }
    }

    fn tick(&mut self) {
        self.cpu.run_instruction(&mut self.bus);
    }

    // from https://jsgroth.dev/blog/posts/ps1-sideloading/
    fn sideload_exe(&mut self, exe: Vec<u8>) {

        // Wait for the BIOS to jump to the shell
        while self.cpu.pc != 0x80030000 {
            // Tick must be instruction-by-instruction to avoid possibly missing the $80030000 jump
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

        println!("sideloading done!");

    }
}

fn main() {
    let bios = std::fs::read("res/SCPH1001.BIN").unwrap();
    let sideloaded_exe = std::fs::read("res/psxtest_cpu.exe").unwrap();

    let mut emu = Emulator::new(bios);

    emu.sideload_exe(sideloaded_exe);

    loop {
        emu.tick();
    }

}
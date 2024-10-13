use cpu::R3000;
use bus::Bus;

mod cpu;
pub mod bus;

fn main() {
    let bios = std::fs::read("res/SCPH1001.BIN").unwrap();


    let mut bus = Bus::new(bios);
    let mut cpu = R3000::new();

    loop {
        cpu.run_instruction(&mut bus);
    }

}

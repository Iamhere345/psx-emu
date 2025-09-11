# PSX-Emu

A WIP Sony PlayStation (PS1) Emulator.

## Building

Compile from source using `cargo run --release`. If you don't have cargo you can install it [here](https://www.rust-lang.org/tools/install)

## Usage

To use the emulator you need to have a PS1 BIOS file (only SCPH1001 and SCPH101 BIOSes have been tested). It should be placed in a folder called `res` in the project directory and named `SCPH1001.bin` (You can change this by editing the `BIOS_PATH` variable in `desktop/src/app.rs`)

### Controls

For now only keyboard controls are supported.

 - Up: W
 - Down: S
 - Left: A
 - Right: D
 - Cross: K
 - Square: J
 - Triangle: I
 - Circle: L
 - L1: Q
 - L2: 1
 - R1: E
 - R2: 3
 - Start: Enter
 - Select: Backslash

## Screenshots
<img width="1802" height="832" alt="Screenshot 2025-09-07 162252" src="https://github.com/user-attachments/assets/8747fc08-253e-41e3-b0a1-515a692f424e" />
<img width="1802" height="832" alt="Screenshot 2025-09-07 162423" src="https://github.com/user-attachments/assets/5f4a2053-7d88-4fe4-b88f-a8866e81d6b5" />
<img width="1802" height="832" alt="Screenshot 2025-09-07 163120" src="https://github.com/user-attachments/assets/f20711dc-f907-4b7f-bc11-b6cee865d25c" />
<img width="1802" height="832" alt="Screenshot 2025-09-07 163340" src="https://github.com/user-attachments/assets/7cc26a5b-cf58-4a3c-833e-4f0e0764db27" />
<img width="1802" height="832" alt="Screenshot 2025-09-07 163623" src="https://github.com/user-attachments/assets/b81a5e60-3a8e-41e6-a76b-7d38b7d728fe" />

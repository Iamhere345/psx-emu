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
<img width="1802" height="832" alt="Screenshot 2025-09-30 133538" src="https://github.com/user-attachments/assets/2190b21d-4215-463d-a5ee-315ae196f4a1" />
<img width="1802" height="832" alt="Screenshot 2025-09-30 134950" src="https://github.com/user-attachments/assets/3239c450-6007-4c4b-bee0-c38ae415f331" />
<img width="1802" height="832" alt="mgs" src="https://github.com/user-attachments/assets/9d6428f8-39f4-478b-b43c-9866eaf007f3" />
<img width="1802" height="832" alt="Screenshot 2025-09-30 114607" src="https://github.com/user-attachments/assets/d9812294-a928-438b-acf5-96e9e7d1ce91" />
<img width="1802" height="832" alt="Screenshot 2025-09-30 135129" src="https://github.com/user-attachments/assets/49e7eb11-5970-469a-97a2-968ae8173025" />
<img width="1802" height="832" alt="Screenshot 2025-09-30 133843" src="https://github.com/user-attachments/assets/2362269e-3028-4540-a261-f6b54dcdf8f9" />
<img width="1802" height="832" alt="Screenshot 2025-09-30 135702" src="https://github.com/user-attachments/assets/f8e76d1f-013d-4955-80e2-e4be0881042d" />
<img width="1802" height="832" alt="Screenshot 2025-09-07 163623" src="https://github.com/user-attachments/assets/2d47cc51-52aa-4157-8562-8dd0a2612580" />

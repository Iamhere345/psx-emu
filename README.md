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
![image](https://github.com/user-attachments/assets/8f0124f8-ad8b-4e15-a200-9275ed306742)
![image](https://github.com/user-attachments/assets/b26b1dd9-47a8-410b-b68f-5ffa2e27ec33)
![image](https://github.com/user-attachments/assets/eceeace9-3c76-40c9-92df-a3033295c6c7)
![image](https://github.com/user-attachments/assets/a892cfb3-ca70-4cce-b937-fd5fd29f720d)
![image](https://github.com/user-attachments/assets/59b9696c-b3df-440f-84fe-c9b9b2028b2b)
![image](https://github.com/user-attachments/assets/c39e5fd1-535c-4f3a-849b-8f4725d62889)

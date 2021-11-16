# RMG-001
Rust Matrix Game - 001: Yet Another Rust Gameboy Emulator.

This is just a fun project I'm making for learning and practice purposes. If you want a fully-featured Gameboy emulator, this is probably not the best one :P

Any help or suggestion is welcome!

## TODO
- [x] CPU implementation
- [x] Interrupts
- [x] Timer
- [x] Joypad (not configurable yet)
- [X] PPU implementations
- [x] Render the pixels
- [ ] MBC Implementations
  - [x] NoMBC
  - [x] MBC1
  - [x] MBC2
  - [ ] MBC3
  - [ ] MBC5
  - [ ] MBC6
  - [ ] MBC7
  - [ ] HuC1
- [ ] Web Assembly support (because this is a Rust project and it has to support Web Assembly)
- [ ] Gameboy boot ROM (Not important for now)
- [ ] Gameboy Color compatibility
- [ ] Sound
- [ ] Many code refactors and optimizations are needed

# Resources
This project would have been completely impossible without all the documentation and help that exists online for the Nintendo Gameboy:
- The EmuDev community
- Pandocs: https://gbdev.io/pandocs/
- Gameboy emulation guide: https://hacktixme.ga/GBEDG/
- CPU opcodes table: https://izik1.github.io/gbops/
- Opcodes behaviour: https://rgbds.gbdev.io/docs/v0.5.1/gbz80.7
- The Ultimate Gameboy talk: https://youtu.be/HyzD8pNlpwI

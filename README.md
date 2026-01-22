# Boy

![Rust](https://img.shields.io/badge/rust-stable-orange)
![Platform](https://img.shields.io/badge/platform-DMG--01-lightgrey)

Work-in-progress **Nintendo Game Boy (DMG-01) emulator in Rust**.

This is a learning-focused project with an emphasis on **hardware fidelity**.

## Screenshots

|                                                          |                                  |
| -------------------------------------------------------- | -------------------------------- |
| ![tetris](/resources/tetris.png)                         | Tetris                           |
| ![kirby](/resources/kirby.png)                           | Kirby's Dream Land               |
| ![pokemon](/resources/pokemon-red-2.png)                 | Pokémon Red Version              |
| ![instruction timing](/resources/instruction_timing.png) | Blargg's instruction timing test |
| ![cpu instr](/resources/cpu_instr.png)                   | Blargg's CPU instruction test    |

## Implementation

- [x] Cart loading
- [x] Basic MMU
- [x] CPU instructions
- [x] Interrupts
- [x] Timers
- [x] PPU
  - [x] Background
  - [x] Window
  - [x] Sprites
- [x] Input
- [ ] Sound
- [ ] Memory banking
  - [x] MBC1
  - [x] MBC2
  - [x] MBC3
  - [ ] MBC4
  - [ ] MBC5
  - [ ] MBC6
  - [ ] MBC7
  - [ ] Saving RAM to disk (battery backed ram)
  - [ ] RTC
## TODO

- Tick correct DMA transfer instead of instant copy
- Correct OAM scan to handle object draw priority
- Input on tick rather than frame

## Disclaimer

Not affiliated with Nintendo.

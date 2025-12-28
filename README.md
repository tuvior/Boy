# Boy

![Rust](https://img.shields.io/badge/rust-stable-orange)
![Platform](https://img.shields.io/badge/platform-DMG--01-lightgrey)

Work-in-progress **Nintendo Game Boy (DMG-01) emulator in Rust**.

This is a learning-focused project with an emphasis on **hardware fidelity**.

## Screenshots

|                                              |                                  |
| -------------------------------------------- | -------------------------------- |
| ![tetris](/resources/tetris.png)             | Tetris                           |
| ![tetris](/resources/instruction_timing.png) | Blargg's instruction timing test |
| ![tetris](/resources/cpu_instr.png)          | Blargg's CPU instruction test    |

## Implementation

- [x] Cart loading
- [x] Basic MMU
- [x] CPU instructions
- [x] Interrupts
- [x] Timers
- [x] PPU
- - [x] Background
- - [x] Window
- - [x] Sprites
- [x] Input
- [ ] Sound
- [ ] Memory banking

## TODO

- Tick correct DMA transfer instead of instant copy
- Correct OAM scan to handle object draw priority
- Input on tick rather than frame

## Disclaimer

Not affiliated with Nintendo.

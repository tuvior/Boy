use crate::{
    cart::Cart,
    cpu::CPU,
    mmu::MMU,
    ppu::{SCREEN_H, SCREEN_W},
};

pub struct GameBoy {
    cpu: CPU,
    mmu: MMU,
}

// Using a green tint to emulate the DMG-01 LCD screen.
const LCD_PALETTE: [u32; 4] = [
    0xE8F8D0, // White
    0x88C070, // Light gray
    0x346856, // Dark gray
    0x081818, // Black
];

impl GameBoy {
    pub fn new(cart: Cart) -> Self {
        let title = cart.get_title();
        println!("Booted ROM: {title}");

        let header = &cart.header;

        println!("{header}");

        GameBoy {
            cpu: CPU::init(),
            mmu: MMU::new(cart),
        }
    }

    pub fn run_frame(&mut self, key_states: KeyStates) {
        self.mmu.handle_joypad(key_states);
        loop {
            let cycles = self.cpu.step(&mut self.mmu);
            let frame_ready = self.mmu.tick(cycles);

            if frame_ready {
                break;
            }
        }
    }

    pub fn get_last_frame_buffer(&self) -> [u32; SCREEN_W * SCREEN_H] {
        let mut colors = [0u32; SCREEN_H * SCREEN_W];

        for (i, &pix) in self.mmu.get_fb().iter().enumerate() {
            let c = LCD_PALETTE[pix as usize];
            colors[i] = c;
        }

        colors
    }
}

#[derive(Default)]
pub struct KeyStates {
    pub a: bool,
    pub b: bool,
    pub start: bool,
    pub select: bool,
    pub up: bool,
    pub down: bool,
    pub left: bool,
    pub right: bool,
}

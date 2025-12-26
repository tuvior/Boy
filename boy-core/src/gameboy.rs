use crate::{
    cart::Cart,
    cpu::cpu::{CPU, Cycles},
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
        let header = &cart.header;
        println!("Booted ROM: {header}");

        GameBoy {
            cpu: CPU::init(),
            mmu: MMU::new(cart),
        }
    }

    pub fn frame(&mut self) -> [u32; SCREEN_W * SCREEN_H] {
        loop {
            let cycles = self.cpu.step(&mut self.mmu);
            let frame_ready = self.mmu.tick(cycles);

            if frame_ready {
                break;
            }
        }

        // Doing it this way for now to iterate
        let mut colors = [0u32; SCREEN_H * SCREEN_W];

        for (i, &pix) in self.mmu.get_fb().iter().enumerate() {
            let c = LCD_PALETTE[pix as usize];
            colors[i] = c;
        }

        colors
    }
}

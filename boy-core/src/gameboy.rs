use crate::{
    cart::Cart,
    cpu::cpu::{CPU, Cycles},
    mmu::MMU,
};

pub struct GameBoy {
    cpu: CPU,
    mmu: MMU,
}

impl GameBoy {
    pub fn new(cart: Cart) -> Self {
        let header = &cart.header;
        println!("Booted ROM: {header}");

        GameBoy {
            cpu: CPU::init(),
            mmu: MMU::new(cart),
        }
    }
    pub fn step(&mut self) -> (Cycles, bool) {
        let cycles = self.cpu.step(&mut self.mmu);
        let frame_ready = self.mmu.tick(cycles);
        (cycles, frame_ready)
    }
}

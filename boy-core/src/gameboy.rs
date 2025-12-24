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
    pub fn step(&mut self) -> Cycles {
        let m = self.cpu.step(&mut self.mmu);
        self.mmu.tick(m);
        m
    }
}

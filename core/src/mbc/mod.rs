pub mod mbc1;
pub mod mbc2;
pub mod rom_only;

pub trait MemoryController {
    fn rb(&mut self, addr: u16) -> u8;
    fn wb(&mut self, addr: u16, value: u8);
    fn save(&self);
}

pub struct Missing;

impl MemoryController for Missing {
    fn rb(&mut self, _: u16) -> u8 {
        panic!("Unimplemented cartridge type")
    }

    fn wb(&mut self, _: u16, _: u8) {
        panic!("Unimplemented cartridge type")
    }

    fn save(&self) {}
}

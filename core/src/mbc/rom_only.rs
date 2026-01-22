use crate::mbc::MemoryController;

pub struct RomOnly {
    rom: Vec<u8>,
    eram: Vec<u8>,
}

impl RomOnly {
    pub fn new(rom: Vec<u8>, ram_size: u32) -> Self {
        RomOnly {
            rom,
            eram: vec![0; ram_size as usize],
        }
    }
}

impl MemoryController for RomOnly {
    fn rb(&mut self, addr: u16) -> u8 {
        match addr {
            0x0000..=0x7FFF => self.rom[addr as usize],
            0xA000..=0xBFFF => self.eram[(addr - 0xA000) as usize],
            _ => unreachable!(),
        }
    }

    fn wb(&mut self, addr: u16, value: u8) {
        match addr {
            0x0000..=0x7FFF => (),
            0xA000..=0xBFFF => self.eram[(addr - 0xA000) as usize] = value,
            _ => unreachable!(),
        }
    }

    fn save(&self) -> Option<Vec<u8>> {
        None
    }
}

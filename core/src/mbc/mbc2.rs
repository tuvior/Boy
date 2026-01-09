use crate::mbc::MemoryController;

pub struct Mbc2 {
    rom: Vec<u8>,
    ram: [u8; 0x200],
    has_battery: bool,
    // Registers
    ram_enable: bool, // [0x0000 – 0x1FFF] — RAM Enable, ROM Bank Number
    rom_bank: u8,     // [0x0000 – 0x1FFF] — RAM Enable, ROM Bank Number
}

impl Mbc2 {
    const ROM_BANK_SIZE: usize = 16 * 1024;

    pub fn new(rom: Vec<u8>, has_battery: bool) -> Self {
        Mbc2 {
            rom,
            ram: [0; 0x200],
            has_battery,
            ram_enable: false,
            rom_bank: 0,
        }
    }

    fn rom_bank_addr_start(&self) -> usize {
        Mbc2::ROM_BANK_SIZE * self.rom_bank as usize
    }
}

impl MemoryController for Mbc2 {
    fn rb(&mut self, addr: u16) -> u8 {
        match addr {
            0x0000..=0x3FFF => self.rom[addr as usize],
            0x4000..=0x7FFF => self.rom[(addr - 0x4000) as usize + self.rom_bank_addr_start()],
            0xA000..=0xA1FF => {
                if self.ram_enable {
                    self.ram[(addr - 0xA000) as usize] | 0xF0 // MBC2 has 4 bit ram
                } else {
                    0xFF
                }
            }
            0xA200..=0xBFFF => {
                if self.ram_enable {
                    self.ram[((addr - 0xA200) & 0x1FF) as usize] | 0xF0 // Echo ram
                } else {
                    0xFF
                }
            }
            _ => 0xFF,
        }
    }

    fn wb(&mut self, addr: u16, value: u8) {
        match addr {
            0x0000..=0x3FFF => {
                if addr & 0x100 != 0 {
                    self.rom_bank = u8::max(value & 0x0F, 1);
                } else {
                    self.ram_enable = value & 0x0F == 0x0A
                }
            }
            0xA000..=0xA1FF => {
                if self.ram_enable {
                    self.ram[(addr - 0xA000) as usize] = value | 0xF0
                }
            }
            0xA200..=0xBFFF => {
                if self.ram_enable {
                    self.ram[((addr - 0xA200) & 0x1FF) as usize] = value | 0xF0
                }
            }
            _ => (),
        }
    }

    fn save(&self) {
        if self.has_battery {}
    }
}

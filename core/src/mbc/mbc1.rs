use crate::mbc::MemoryController;

pub struct Mbc1 {
    rom: Vec<u8>,
    ram: Vec<u8>,
    has_ram: bool,
    has_battery: bool,
    // State
    ram_enable: bool, // [0x0000 – 0x1FFF] — Enable ram by writing $A to any address
    rom_bank: u8,     // [0x2000 – 0x3FFF]
    ram_bank_or_upper_rom: u8, // [0x4000 – 0x5FFF]
    banking_mode: u8, // [0x6000 – 0x7FFF] — 0: ROM, 1: RAM
}

impl Mbc1 {
    const ROM_BANK_SIZE: usize = 16 * 1024;
    const RAM_BANK_SIZE: usize = 8 * 1024;

    pub fn new(rom: Vec<u8>, ram_size: u32, has_ram: bool, has_battery: bool) -> Self {
        Mbc1 {
            rom,
            ram: vec![0; ram_size as usize],
            has_ram,
            has_battery,
            ram_enable: false,
            rom_bank: 0,
            ram_bank_or_upper_rom: 0,
            banking_mode: 0,
        }
    }

    fn selected_rom_bank(&self) -> u16 {
        let lower_bank = u16::max(self.rom_bank as u16, 1);
        if self.banking_mode == 1 {
            lower_bank
        } else {
            ((self.ram_bank_or_upper_rom as u16) << 5) | lower_bank
        }
    }

    fn selected_ram_bank(&self) -> u16 {
        if self.banking_mode == 0 {
            0
        } else {
            self.ram_bank_or_upper_rom as u16
        }
    }

    fn rom_bank_addr_start(&self) -> usize {
        let selected_bank = self.selected_rom_bank();
        Mbc1::ROM_BANK_SIZE * selected_bank as usize
    }

    fn ram_bank_addr_start(&self) -> usize {
        let selected_bank = self.selected_ram_bank();
        Mbc1::RAM_BANK_SIZE * selected_bank as usize
    }
}

impl MemoryController for Mbc1 {
    fn rb(&mut self, addr: u16) -> u8 {
        match addr {
            0x0000..=0x3FFF => match self.banking_mode {
                0x00 => self.rom[addr as usize],
                0x01 => self.rom[addr as usize + self.rom_bank_addr_start()],
                _ => unreachable!(),
            },
            0x4000..=0x7FFF => self.rom[(addr - 0x4000) as usize + self.rom_bank_addr_start()],
            0xA000..=0xBFFF => {
                if !self.has_ram || !self.ram_enable {
                    0xFF
                } else {
                    self.ram[(addr - 0xA000) as usize + self.ram_bank_addr_start()]
                }
            }
            _ => 0xFF,
        }
    }

    fn wb(&mut self, addr: u16, value: u8) {
        match addr {
            0x0000..=0x1FFF => self.ram_enable = value & 0x0F == 0x0A,
            0x2000..=0x3FFF => self.rom_bank = value & 0x1F,
            0x4000..=0x5FFF => self.ram_bank_or_upper_rom = value & 0x03,
            0x6000..=0x7FFF => self.banking_mode = value & 0x01,
            0xA000..=0xBFFF => {
                if self.ram_enable {
                    self.ram[(addr - 0xA000) as usize] = value
                }
            }
            _ => (),
        }
    }

    fn save(&self) {
        if self.has_battery {}
    }
}

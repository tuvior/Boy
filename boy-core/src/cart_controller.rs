pub trait CartController {
    fn rb(&mut self, addr: u16) -> u8;
    fn wb(&mut self, addr: u16, value: u8);
}

pub struct Missing;

impl CartController for Missing {
    fn rb(&mut self, _: u16) -> u8 {
        panic!("Unimplemented cartridge type")
    }

    fn wb(&mut self, _: u16, _: u8) {
        panic!("Unimplemented cartridge type")
    }
}

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

impl CartController for RomOnly {
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
}

pub struct Mbc1 {
    rom: Vec<u8>,
    ram: Vec<u8>,
    has_ram: bool,
    has_battery: bool,
    ram_active: bool,
}

impl Mbc1 {
    pub fn new(rom: Vec<u8>, ram_size: u32, has_ram: bool, has_battery: bool) -> Self {
        Mbc1 {
            rom,
            ram: vec![0; ram_size as usize],
            has_ram,
            has_battery,
            ram_active: false,
        }
    }
}

impl CartController for Mbc1 {
    fn rb(&mut self, addr: u16) -> u8 {
        match addr {
            0x0000..=0x3FFF => self.rom[addr as usize],
            0x4000..=0x7FFF => todo!(),
            0xA000..=0xBFFF => {
                if !self.has_ram || !self.ram_active {
                    0xFF
                } else {
                    let addr = addr - 0xA000;
                    todo!()
                }
            }
            _ => 0xFF,
        }
    }

    fn wb(&mut self, addr: u16, value: u8) {
        match addr {
            0x0000..=0x1FFF => todo!("RAM enable if (value & 0x0F) == 0x0A"),
            0x2000..=0x3FFF => todo!("set low5 = value & 0x1F; if 0 => 1"),
            0x4000..=0x5FFF => todo!("set upper2 = value & 0x03"),
            0x6000..=0x7FFF => todo!("mode = value & 0x01"),
            0xA000..=0xBFFF => todo!("write RAM if enabled"),
            _ => (),
        }
    }
}

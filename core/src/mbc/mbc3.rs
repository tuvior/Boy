use crate::mbc::{MemoryController, rtc::RTC};

pub struct Mbc3 {
    rom: Vec<u8>,
    ram: Vec<u8>,
    has_ram: bool,
    has_battery: bool,
    has_timer: bool,
    // Registers
    ram_timer_enable: bool, // [0x0000 – 0x1FFF] — Enable ram and timer by writing $A to any address
    rom_bank: u8,           // [0x2000 – 0x3FFF]
    ram_bank_rtc_register: u8, // [0x4000 – 0x5FFF] — $00-$07 The corresponding RAM Bank. $08-$0C The corresponding RTC Register
    rtc: Option<RTC>,
}

impl Mbc3 {
    const ROM_BANK_SIZE: usize = 16 * 1024;
    const RAM_BANK_SIZE: usize = 8 * 1024;

    pub fn new(
        rom: Vec<u8>,
        ram_size: u32,
        has_ram: bool,
        has_battery: bool,
        has_timer: bool,
    ) -> Self {
        Mbc3 {
            rom,
            ram: vec![0; ram_size as usize],
            has_ram,
            has_battery,
            has_timer,
            ram_timer_enable: false,
            rom_bank: 1,
            ram_bank_rtc_register: 0,
            rtc: has_timer.then(RTC::init),
        }
    }

    fn rom_bank_addr_start(&self) -> usize {
        Mbc3::ROM_BANK_SIZE * self.rom_bank as usize
    }

    fn ram_bank_addr_start(&self) -> usize {
        Mbc3::RAM_BANK_SIZE * self.ram_bank_rtc_register as usize
    }
}

impl MemoryController for Mbc3 {
    fn rb(&mut self, addr: u16) -> u8 {
        match addr {
            0x0000..=0x3FFF => self.rom[addr as usize],
            0x4000..=0x7FFF => self.rom[(addr - 0x4000) as usize + self.rom_bank_addr_start()],
            0xA000..=0xBFFF => {
                if !self.has_ram || !self.ram_timer_enable {
                    0xFF
                } else {
                    let ram_selected = self.ram_bank_rtc_register <= 0x07;

                    if ram_selected {
                        self.ram[(addr - 0xA000) as usize + self.ram_bank_addr_start()]
                    } else if self.has_timer
                        && let Some(rtc) = &self.rtc
                    {
                        rtc.read_register(self.ram_bank_rtc_register)
                    } else {
                        0xFF
                    }
                }
            }
            _ => 0xFF,
        }
    }

    fn wb(&mut self, addr: u16, value: u8) {
        match addr {
            0x0000..=0x1FFF => {
                if value & 0x0F == 0x0A {
                    self.ram_timer_enable = true
                } else if value & 0x0F == 0x00 {
                    self.ram_timer_enable = false
                }
            }
            0x2000..=0x3FFF => self.rom_bank = u8::max(value & 0x7F, 1),
            0x4000..=0x5FFF => self.ram_bank_rtc_register = value & 0x0F,
            0xA000..=0xBFFF => {
                if self.has_ram && self.ram_timer_enable {
                    let ram_selected = self.ram_bank_rtc_register <= 0x07;

                    if ram_selected {
                        let bank_start = self.ram_bank_addr_start();
                        self.ram[(addr - 0xA000) as usize + bank_start] = value
                    } else if self.has_timer
                        && let Some(rtc) = &mut self.rtc
                    {
                        rtc.write_regisetr(self.ram_bank_rtc_register, value)
                    }
                }
            }
            0x6000..=0x7FFF => {
                if self.ram_timer_enable
                    && self.has_timer
                    && let Some(rtc) = &mut self.rtc
                {
                    rtc.latch(value);
                }
            }
            _ => (),
        }
    }

    fn save(&self) {
        if self.has_battery {}
    }
}

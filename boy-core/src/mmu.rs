use crate::{
    cart::Cart,
    cpu::cpu::Cycles,
    interrupt::{INTERRUPT_MASK, Interrupt},
    timer::{DIV_ADDR, TAC_ADDR, Timer},
};

const IF_ADDR: u16 = 0xFF0F;

pub struct MMU {
    cart: Cart,         // [0x0000 - 0x7FFF] - Cartridge ROM
    vram: [u8; 0x2000], // [0x8000 - 0x9FFF] - Video RAM
    eram: [u8; 0x2000], // [0xA000 - 0xBFFF] - External RAM (from cartirdge in real HW)
    wram: [u8; 0x2000], // [0xC000 - 0xDFFF] - Work RAM
    oam: [u8; 0xA0],    // [0xFE00 - 0xFE9F] - Object Attribute Memory
    hram: [u8; 0x7F],   // [0xFF80 - 0xFFFE] - High RAM
    if_: u8,            // [0xFF0F] - Interrupt Flag
    ie: u8,             // [0xFFFF] - Interrupt Enable Register
    timer: Timer,
}

impl MMU {
    pub fn new(cart: Cart) -> Self {
        MMU {
            cart,
            vram: [0; 0x2000],
            eram: [0; 0x2000],
            wram: [0; 0x2000],
            oam: [0; 0xA0],
            hram: [0; 0x7F],
            if_: 0,
            ie: 0,
            timer: Timer::default(),
        }
    }

    #[inline]
    pub fn rb(&self, addr: u16) -> u8 {
        match addr {
            0x0000..=0x7FFF => self.cart.read_rom(addr),
            0x8000..=0x9FFF => self.vram[(addr - 0x8000) as usize],
            0xA000..=0xBFFF => self.eram[(addr - 0xA000) as usize],
            0xC000..=0xDFFF => self.wram[(addr - 0xC000) as usize],
            0xE000..=0xFDFF => self.wram[(addr - 0xE000) as usize], // Echo
            0xFE00..=0xFE9F => self.oam[(addr - 0xFE00) as usize],
            0xFEA0..=0xFEFF => 0xFF, // Unusable
            0xFF00..=0xFF7F => match addr {
                DIV_ADDR..=TAC_ADDR => self.timer.rb(addr),
                IF_ADDR => self.if_ | 0xE0,
                _ => 0x0, // Unimplemented
            },
            0xFF80..=0xFFFE => self.hram[(addr - 0xFF80) as usize],
            0xFFFF => self.ie,
        }
    }

    pub fn rw(&self, addr: u16) -> u16 {
        let lo = self.rb(addr) as u16;
        let hi = self.rb(addr.wrapping_add(1)) as u16;
        (hi << 8) | lo
    }

    #[inline]
    pub fn wb(&mut self, addr: u16, value: u8) {
        match addr {
            0x0000..=0x7FFF => (), // Unwriteable
            0x8000..=0x9FFF => self.vram[(addr - 0x8000) as usize] = value,
            0xA000..=0xBFFF => self.eram[(addr - 0xA000) as usize] = value,
            0xC000..=0xDFFF => self.wram[(addr - 0xC000) as usize] = value,
            0xE000..=0xFDFF => self.wram[(addr - 0xE000) as usize] = value,
            0xFE00..=0xFE9F => self.oam[(addr - 0xFE00) as usize] = value,
            0xFEA0..=0xFEFF => (), // Unwriteable
            0xFF00..=0xFF7F => match addr {
                DIV_ADDR..=TAC_ADDR => self.timer.wb(addr, value),
                IF_ADDR => self.if_ = value | 0xE0,
                _ => (), // Unimplemented
            },
            0xFF80..=0xFFFE => self.hram[(addr - 0xFF80) as usize] = value,
            0xFFFF => self.ie = value,
        }
    }

    pub fn ww(&mut self, addr: u16, value: u16) {
        self.wb(addr, (value & 0x00FF) as u8);
        self.wb(addr.wrapping_add(1), (value >> 8) as u8);
    }

    pub fn tick(&mut self, cycles: Cycles) {
        if self.timer.tick(to_tcycles(cycles)) {
            self.request_interrupt(Interrupt::Timer.bit());
        }
    }

    pub fn pending_interrupts(&self) -> u8 {
        let mask = INTERRUPT_MASK;
        let if_ = self.rb(IF_ADDR);

        self.ie & if_ & mask
    }

    pub fn request_interrupt(&mut self, bit: u8) {
        let if_ = self.rb(IF_ADDR) | bit;
        self.wb(IF_ADDR, if_);
    }

    pub fn clear_interrupt(&mut self, bit: u8) {
        let if_ = self.rb(IF_ADDR) & !bit;
        self.wb(IF_ADDR, if_);
    }
}

pub type TCycles = u32;

pub fn to_tcycles(cycles: Cycles) -> TCycles {
    cycles as TCycles * 4
}

use crate::{
    cart::Cart,
    cpu::cpu::Cycles,
    interrupt::{INTERRUPT_MASK, Interrupt},
    ppu::{DMA_ADDR, LCDC_ADDR, PPU, WX_ADDR},
    timer::{DIV_ADDR, TAC_ADDR, Timer},
};

const IF_ADDR: u16 = 0xFF0F;

pub struct MMU {
    cart: Cart,         // [0x0000 - 0x7FFF] - Cartridge ROM
    eram: [u8; 0x2000], // [0xA000 - 0xBFFF] - External RAM (from cartirdge in real HW)
    wram: [u8; 0x2000], // [0xC000 - 0xDFFF] - Work RAM
    hram: [u8; 0x7F],   // [0xFF80 - 0xFFFE] - High RAM
    if_: u8,            // [0xFF0F] - Interrupt Flag
    ie: u8,             // [0xFFFF] - Interrupt Enable Register
    ppu: PPU,
    timer: Timer,
}

impl MMU {
    pub fn new(cart: Cart) -> Self {
        MMU {
            cart,
            eram: [0; 0x2000],
            wram: [0; 0x2000],
            hram: [0; 0x7F],
            if_: 0xE0,
            ie: 0,
            ppu: PPU::new(),
            timer: Timer::default(),
        }
    }

    #[inline]
    pub fn rb(&self, addr: u16) -> u8 {
        match addr {
            0x0000..=0x7FFF => self.cart.read_rom(addr),
            0x8000..=0x9FFF => self.ppu.rb(addr), // VRAM
            0xA000..=0xBFFF => self.eram[(addr - 0xA000) as usize],
            0xC000..=0xDFFF => self.wram[(addr - 0xC000) as usize],
            0xE000..=0xFDFF => self.wram[(addr - 0xE000) as usize], // Echo
            0xFE00..=0xFE9F => self.ppu.rb(addr),                   // OAM
            0xFEA0..=0xFEFF => 0xFF,                                // Unusable
            0xFF00..=0xFF7F => match addr {
                DIV_ADDR..=TAC_ADDR => self.timer.rb(addr), // Redirect to timer
                DMA_ADDR => 0xFF,                           // Unsupported
                LCDC_ADDR..=WX_ADDR => self.ppu.rb(addr),   // Redirect to PPU
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
            0x0000..=0x7FFF => (),                       // Unwriteable
            0x8000..=0x9FFF => self.ppu.wb(addr, value), // VRAM
            0xA000..=0xBFFF => self.eram[(addr - 0xA000) as usize] = value,
            0xC000..=0xDFFF => self.wram[(addr - 0xC000) as usize] = value,
            0xE000..=0xFDFF => self.wram[(addr - 0xE000) as usize] = value,
            0xFE00..=0xFE9F => self.ppu.wb(addr, value), // OAM
            0xFEA0..=0xFEFF => (),                       // Unwriteable
            0xFF00..=0xFF7F => match addr {
                DIV_ADDR..=TAC_ADDR => self.timer.wb(addr, value), // Redirect to timer
                DMA_ADDR => (), // OAM DMA source address & start (unimplemented for now)
                LCDC_ADDR..=WX_ADDR => self.ppu.wb(addr, value), // Redirect to PPU
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

    pub fn tick(&mut self, cycles: Cycles) -> bool {
        let mut interrupts = 0;
        interrupts |= self.timer.tick(to_tcycles(cycles));

        let (ppu_interrupts, frame_ready) = self.ppu.tick(to_tcycles(cycles));

        interrupts |= ppu_interrupts;

        if interrupts != 0 {
            self.request_interrupt(interrupts);
        }

        frame_ready
    }

    pub fn pending_interrupts(&self) -> u8 {
        let mask = INTERRUPT_MASK;
        self.ie & self.if_ & mask
    }

    pub fn request_interrupt(&mut self, bits: u8) {
        self.if_ |= bits;
    }

    pub fn clear_interrupt(&mut self, bit: u8) {
        self.if_ &= !bit;
    }
}

pub type TCycles = u32;

#[inline]
fn to_tcycles(cycles: Cycles) -> TCycles {
    cycles as TCycles * 4
}

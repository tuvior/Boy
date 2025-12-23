pub const INTERRUPT_MASK: u8 = 0x1F;
pub const INTERRUPT_CYCLES: u8 = 5;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Interrupt {
    VBlank = 0,
    Stat = 1,
    Timer = 2,
    Serial = 3,
    Joypad = 4,
}

impl Interrupt {
    #[inline]
    pub const fn bit(self) -> u8 {
        1u8 << (self as u8)
    }

    #[inline]
    pub const fn vector(self) -> u16 {
        match self {
            Interrupt::VBlank => 0x0040,
            Interrupt::Stat => 0x0048,
            Interrupt::Timer => 0x0050,
            Interrupt::Serial => 0x0058,
            Interrupt::Joypad => 0x0060,
        }
    }
}

#[inline]
pub fn highest_priority(pending_interrupt: u8) -> Option<Interrupt> {
    if pending_interrupt & (1 << Interrupt::VBlank.bit()) != 0 {
        return Some(Interrupt::VBlank);
    }
    if pending_interrupt & (1 << Interrupt::Stat.bit()) != 0 {
        return Some(Interrupt::Stat);
    }
    if pending_interrupt & (1 << Interrupt::Timer.bit()) != 0 {
        return Some(Interrupt::Timer);
    }
    if pending_interrupt & (1 << Interrupt::Serial.bit()) != 0 {
        return Some(Interrupt::Serial);
    }
    if pending_interrupt & (1 << Interrupt::Joypad.bit()) != 0 {
        return Some(Interrupt::Joypad);
    }
    None
}

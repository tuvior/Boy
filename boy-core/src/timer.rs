use crate::mmu::TCycles;

pub const DIV_ADDR: u16 = 0xFF04;
pub const TIMA_ADDR: u16 = 0xFF05;
pub const TMA_ADDR: u16 = 0xFF06;
pub const TAC_ADDR: u16 = 0xFF07;

#[derive(Default)]
pub struct Timer {
    div: u16,      // [0xFF04] — DIV: Divider register
    tima: u8,      // [0xFF05] — TIMA: Timer counter
    tma: u8,       // [0xFF06] — TMA: Timer modulo
    tac: u8,       // [0xFF07] — TAC: Timer control [ - - - - - 2 1 0 ] 2: Enable 1 0: Clock select
    tima_acc: u32, // TIMA accumulator
}

impl Timer {
    pub fn rb(&self, addr: u16) -> u8 {
        match addr {
            DIV_ADDR => (self.div >> 8) as u8,
            TIMA_ADDR => self.tima,
            TMA_ADDR => self.tma,
            TAC_ADDR => self.tac,
            _ => panic!("Unexpected read at addr: 0x{addr:04X} on Timer."),
        }
    }

    pub fn wb(&mut self, addr: u16, value: u8) {
        match addr {
            DIV_ADDR => self.div = 0,
            TIMA_ADDR => self.tima = value,
            TMA_ADDR => self.tma = value,
            TAC_ADDR => self.tac = value & 0x07,
            _ => panic!("Unexpected write at addr: 0x{addr:04X} on Timer."),
        }
    }

    pub fn tick(&mut self, cycles: TCycles) -> bool {
        self.div = self.div.wrapping_add(cycles as u16);

        // Timer enabled
        if (self.tac & 0x04) == 0 {
            return false;
        }

        // Clock selection
        let period = match self.tac & 0x03 {
            0x00 => 1024,
            0x01 => 16,
            0x02 => 64,
            0x03 => 256,
            _ => unreachable!(),
        };

        self.tima_acc = self.tima_acc.wrapping_add(cycles);

        let mut overflowed = false;
        while self.tima_acc >= period {
            self.tima_acc -= period;

            let (new, did_overflow) = self.tima.overflowing_add(1);
            if did_overflow {
                self.tima = self.tma;
                overflowed = true;
            } else {
                self.tima = new;
            }
        }

        overflowed
    }
}

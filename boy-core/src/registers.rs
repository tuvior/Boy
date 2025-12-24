#[derive(Default)]
pub struct Registers {
    pub a: u8, // Accumulator
    f: u8,     // Flags

    pub b: u8, // BC
    pub c: u8, // ^^

    pub d: u8, // DE
    pub e: u8, // ^^

    pub h: u8, // HL
    pub l: u8, // ^^

    pub sp: u16, // Stack Pointer
    pub pc: u16, // Program Counter
}

#[inline]
fn build_w(hi: u8, lo: u8) -> u16 {
    ((hi as u16) << 8) | (lo as u16)
}

// f: [z n h c - - - -]
//
// z: Zero flag
// n: Subtraction flag
// h: Half Carry flag
// c: Carry flag
const Z_BIT: u8 = 1 << 7;
const N_BIT: u8 = 1 << 6;
const H_BIT: u8 = 1 << 5;
const C_BIT: u8 = 1 << 4;

impl Registers {
    #[inline]
    pub fn af(&self) -> u16 {
        build_w(self.a, self.f)
    }
    #[inline]
    pub fn set_af(&mut self, value: u16) {
        self.a = (value >> 8) as u8;
        self.f = (value as u8) & 0xF0;
    }

    #[inline]
    pub fn bc(&self) -> u16 {
        build_w(self.b, self.c)
    }
    #[inline]
    pub fn set_bc(&mut self, value: u16) {
        self.b = (value >> 8) as u8;
        self.c = value as u8;
    }

    #[inline]
    pub fn de(&self) -> u16 {
        build_w(self.d, self.e)
    }
    #[inline]
    pub fn set_de(&mut self, value: u16) {
        self.d = (value >> 8) as u8;
        self.e = value as u8;
    }

    #[inline]
    pub fn hl(&self) -> u16 {
        build_w(self.h, self.l)
    }
    #[inline]
    pub fn set_hl(&mut self, value: u16) {
        self.h = (value >> 8) as u8;
        self.l = value as u8;
    }

    #[inline]
    pub fn z(&self) -> bool {
        self.f & Z_BIT != 0
    }
    #[inline]
    pub fn set_z(&mut self, v: bool) {
        self.f = if v { self.f | Z_BIT } else { self.f & !Z_BIT };
    }

    #[inline]
    pub fn n(&self) -> bool {
        self.f & N_BIT != 0
    }
    #[inline]
    pub fn set_n(&mut self, v: bool) {
        self.f = if v { self.f | N_BIT } else { self.f & !N_BIT };
    }

    #[inline]
    pub fn h(&self) -> bool {
        self.f & H_BIT != 0
    }
    #[inline]
    pub fn set_h(&mut self, v: bool) {
        self.f = if v { self.f | H_BIT } else { self.f & !H_BIT };
    }

    #[inline]
    pub fn c(&self) -> bool {
        self.f & C_BIT != 0
    }
    #[inline]
    pub fn set_c(&mut self, v: bool) {
        self.f = if v { self.f | C_BIT } else { self.f & !C_BIT };
    }
}

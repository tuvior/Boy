use crate::mmu::TCycles;

pub const LCDC_ADDR: u16 = 0xFF40;
const STAT_ADDR: u16 = 0xFF41;
const SCY_ADDR: u16 = 0xFF42;
const SCX_ADDR: u16 = 0xFF43;
const LY_ADDR: u16 = 0xFF44;
const LYC_ADDR: u16 = 0xFF45;
const BGP_ADDR: u16 = 0xFF47;
const OBP0_ADDR: u16 = 0xFF48;
const OBP1_ADDR: u16 = 0xFF49;
const WY_ADDR: u16 = 0xFF4A;
pub const WX_ADDR: u16 = 0xFF4B;

pub struct PPU {
    vram: [u8; 0x2000], // [0x8000 - 0x9FFF] — Video RAM
    oam: [u8; 0xA0],    // [0xFE00 - 0xFE9F] — Object Attribute Memory
    lcdc: u8,           // [0xFF40] — LCD control [ 7 6 5 4 3 2 1 0 ]
    stat: u8,           // [0xFF41] — LCD status [ - 6 5 4 3 2 1 0 ]
    scy: u8,            // [0xFF42] — Background viewport Y position
    scx: u8,            // [0xFF43] — Background viewport X position
    ly: u8,             // [0xFF44] — LCD Y coordinate [read-only]
    lyc: u8,            // [0xFF45] — LY compare -> LY == LYC triggers a STAT interrupt
    bgp: u8,            // [0xFF47] — DMG BG palette data
    obp0: u8,           // [0xFF48] — DMG OBJ palette 0 data
    obp1: u8,           // [0xFF49] — DMG OBJ palette 1 data
    wy: u8,             // [0xFF4A] — Window Y position
    wx: u8,             // [0xFF4B] — Window X position plus 7
    mode: Mode,
    dot: u16,
    frame_buffer: [u8; 160 * 144],
}

// LCDC
// 7 - LCD & PPU enable: 0 = Off; 1 = On
// 6 - Window tile map area: 0 = 9800–9BFF; 1 = 9C00–9FFF
// 5 - Window enable: 0 = Off; 1 = On
// 4 - BG & Window tile data area: 0 = 8800–97FF; 1 = 8000–8FFF
// 3 - BG tile map area: 0 = 9800–9BFF; 1 = 9C00–9FFF
// 2 - OBJ size: 0 = 8×8; 1 = 8×16
// 1 - OBJ enable: 0 = Off; 1 = On
// 0 - BG & Window enable: 0 = Off; 1 = On

// STAT
//   6 - LYC int select (Read/Write): If set, selects the LYC == LY condition for the STAT interrupt
//   5 - Mode 2 int select (Read/Write): If set, selects the Mode 2 condition for the STAT interrupt
//   4 - Mode 1 int select (Read/Write): If set, selects the Mode 1 condition for the STAT interrupt.
//   3 - Mode 0 int select (Read/Write): If set, selects the Mode 0 condition for the STAT interrupt.
//   2 - LYC == LY (Read-only): Set when LY contains the same value as LYC; it is constantly updated.
// 1 0 - PPU mode (Read-only): Indicates the PPU’s current status. Reports 0 instead when the PPU is disabled.

impl PPU {
    pub fn new() -> Self {
        PPU {
            vram: [0; 0x2000],
            oam: [0; 0xA0],
            lcdc: 0x0,
            stat: 0x0,
            scy: 0x0,
            scx: 0x0,
            ly: 0x0,
            lyc: 0x0,
            bgp: 0x0,
            obp0: 0x0,
            obp1: 0x0,
            wy: 0x0,
            wx: 0x0,
            mode: Mode::OamScan,
            dot: 0,
            frame_buffer: [0; 160 * 144],
        }
    }

    pub fn tick(&mut self, cycles: TCycles) -> (u8, bool) {
        todo!();
    }

    pub fn rb(&self, addr: u16) -> u8 {
        match addr {
            0x8000..=0x9FFF => self.vram[(addr - 0x8000) as usize],
            0xFE00..=0xFE9F => self.oam[(addr - 0xFE00) as usize],
            LCDC_ADDR => self.lcdc,
            STAT_ADDR => self.stat,
            SCY_ADDR => self.scy,
            SCX_ADDR => self.scx,
            LY_ADDR => self.ly,
            LYC_ADDR => self.lyc,
            BGP_ADDR => self.bgp,
            OBP0_ADDR => self.obp0,
            OBP1_ADDR => self.obp1,
            WY_ADDR => self.wy,
            WX_ADDR => self.wx,
            _ => panic!("Unexpected read at addr: 0x{addr:04X} on PPU."),
        }
    }

    pub fn wb(&mut self, addr: u16, value: u8) {
        match addr {
            0x8000..=0x9FFF => self.vram[(addr - 0x8000) as usize] = value,
            0xFE00..=0xFE9F => self.oam[(addr - 0xFE00) as usize] = value,
            LCDC_ADDR => self.lcdc = value,
            STAT_ADDR => self.stat = value,
            SCY_ADDR => self.scy = value,
            SCX_ADDR => self.scx = value,
            LY_ADDR => (), // Read only
            LYC_ADDR => self.lyc = value,
            BGP_ADDR => self.bgp = value,
            OBP0_ADDR => self.obp0 = value,
            OBP1_ADDR => self.obp1 = value,
            WY_ADDR => self.wy = value,
            WX_ADDR => self.wx = value,
            _ => panic!("Unexpected write at addr: 0x{addr:04X} on PPU."),
        }
    }
}

#[derive(PartialEq, Eq)]
#[repr(u8)]
enum Mode {
    HBlank = 0,
    VBlank = 1,
    OamScan = 2,
    Drawing = 3,
}

impl Mode {
    #[inline]
    pub const fn bit(self) -> u8 {
        1u8 << (self as u8)
    }
}

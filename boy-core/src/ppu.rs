use crate::{interrupt::Interrupt, mmu::TCycles};

pub const LCDC_ADDR: u16 = 0xFF40;
const STAT_ADDR: u16 = 0xFF41;
const SCY_ADDR: u16 = 0xFF42;
const SCX_ADDR: u16 = 0xFF43;
const LY_ADDR: u16 = 0xFF44;
const LYC_ADDR: u16 = 0xFF45;
pub const DMA_ADDR: u16 = 0xFF46;
const BGP_ADDR: u16 = 0xFF47;
const OBP0_ADDR: u16 = 0xFF48;
const OBP1_ADDR: u16 = 0xFF49;
const WY_ADDR: u16 = 0xFF4A;
pub const WX_ADDR: u16 = 0xFF4B;

const SCREEN_W: u8 = 160; // Visible pixels
const SCREEN_H: u8 = 144; // Visible pixels
const VBLANK_LINES: u8 = 10;
const OAM_END: u16 = 80; // OAM scan ends after 80 dots
const DRAW_END: u16 = OAM_END + 172; // Finished sending pixels to the LCD (Approximative for now)
const SCANLINE_END: u16 = 456; // Total dots, regardless of draw duration

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
    frame_buffer: [u8; SCREEN_W as usize * SCREEN_H as usize],
    stat_latch: bool,
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

const STAT_LY_LYC: u8 = 6;
const STAT_OAM_SCAN: u8 = 5;
const STAT_VBLANK: u8 = 4;
const STAT_HBLANK: u8 = 3;

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
            mode: Mode::HBlank,
            dot: 0,
            frame_buffer: [0; SCREEN_W as usize * SCREEN_H as usize],
            stat_latch: false,
        }
    }

    fn lcd_off(&self) -> bool {
        (self.lcdc & 1 << 7) == 0
    }

    fn set_mode(&mut self, mode: Mode) {
        self.mode = mode;
        self.stat = (self.stat & 0xFC) | mode as u8
    }

    fn ly_lyc_check(&mut self) -> bool {
        if self.ly == self.lyc {
            self.stat |= 0x04;
            self.stat_condition(STAT_LY_LYC)
        } else {
            self.stat &= 0xFB;
            false
        }
    }

    fn stat_condition(&self, bit: u8) -> bool {
        if bit <= 2 {
            panic!("Invalid requested STAT condition {bit}");
        }
        self.stat & (1 << bit) != 0
    }

    fn reset(&mut self) {
        self.set_mode(Mode::HBlank);
        self.ly = 0;
        self.dot = 0;
        self.stat &= 0xFB; // Clear LY == LYC
        self.stat_latch = false;
    }

    pub fn tick(&mut self, cycles: TCycles) -> (u8, bool) {
        if self.lcd_off() {
            self.reset();
            return (0, false);
        }

        let start_mode = self.mode;

        let mut interrupts = 0;
        let mut frame_ready = false;

        if self.stat_latch {
            self.stat_latch = false;
            interrupts |= Interrupt::Stat.bit();
        }

        self.dot = self.dot.wrapping_add(cycles as u16);

        while self.dot >= SCANLINE_END {
            self.ly = self.ly.wrapping_add(1);

            if self.ly == SCREEN_H {
                interrupts |= Interrupt::VBlank.bit();
            } else if self.ly == SCREEN_H + VBLANK_LINES {
                self.ly = 0;
                frame_ready = true;
            }

            if self.ly_lyc_check() {
                interrupts |= Interrupt::Stat.bit();
            }

            self.dot -= SCANLINE_END;
        }

        if self.ly >= SCREEN_H {
            self.set_mode(Mode::VBlank);
        } else if self.dot < OAM_END {
            self.set_mode(Mode::OamScan);
        } else if self.dot < DRAW_END {
            self.set_mode(Mode::Drawing);
        } else {
            self.set_mode(Mode::HBlank);
        }

        if start_mode != self.mode {
            let bit = match self.mode {
                Mode::HBlank => STAT_HBLANK,
                Mode::VBlank => STAT_VBLANK,
                Mode::OamScan => STAT_OAM_SCAN,
                Mode::Drawing => 0,
            };

            if bit != 0 && self.stat_condition(bit) {
                interrupts |= Interrupt::Stat.bit();
            }
        }

        (interrupts, frame_ready)
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
            STAT_ADDR => self.stat = (self.stat & 0x07) | (value & 0x78), // Don't allow overwriting PPU mode and LYC == LY
            SCY_ADDR => self.scy = value,
            SCX_ADDR => self.scx = value,
            LY_ADDR => (), // Read only
            LYC_ADDR => {
                let need_check = self.lyc != value;
                self.lyc = value;
                if need_check && self.ly_lyc_check() {
                    self.stat_latch = true;
                }
            }
            BGP_ADDR => self.bgp = value,
            OBP0_ADDR => self.obp0 = value,
            OBP1_ADDR => self.obp1 = value,
            WY_ADDR => self.wy = value,
            WX_ADDR => self.wx = value,
            _ => panic!("Unexpected write at addr: 0x{addr:04X} on PPU."),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
enum Mode {
    HBlank = 0,
    VBlank = 1,
    OamScan = 2,
    Drawing = 3,
}

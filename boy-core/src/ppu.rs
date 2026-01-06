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

pub const SCREEN_W: usize = 160; // Visible pixels
pub const SCREEN_H: usize = 144; // Visible pixels
const VBLANK_LINES: u8 = 10;
const OAM_END: u16 = 80; // OAM scan ends after 80 dots
const DRAW_END: u16 = OAM_END + 172; // Finished sending pixels to the LCD (Approximative for now)
const SCANLINE_END: u16 = 456; // Total dots, regardless of draw duration
const MAX_SPRITES_PER_LINE: u8 = 10;

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
    frame_buffer: [u8; SCREEN_W * SCREEN_H],
    bg_color: [u8; SCREEN_W * SCREEN_H],
    stat_latch: bool,
}

// OAM entry
// Byte 0 — Y Position
// Byte 1 — X Position
// Byte 2 — Tile Index
// Byte 3 — Attributes/Flags [ 7 6 5 4 3 2 1 0 ]
//
//              7 - Priority: 0 = No, 1 = BG and Window color indices 1–3 are drawn over this OBJ
//              6 - Y flip: 0 = Normal, 1 = Entire OBJ is vertically mirrored
//              5 - X flip: 0 = Normal, 1 = Entire OBJ is horizontally mirrored
//              4 - DMG palette [Non CGB Mode only]: 0 = OBP0, 1 = OBP1
//              3 - [irrelevant for DMG] Bank [CGB Mode Only]: 0 = Fetch tile from VRAM bank 0, 1 = Fetch tile from VRAM bank 1
//          2 1 0 - [irrelevant for DMG] CGB palette [CGB Mode Only]: Which of OBP0–7 to use

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
    pub fn init() -> Self {
        // This is the register state after the DMG Bios has run.
        // ref: [https://gbdev.io/pandocs/Power_Up_Sequence.html]

        PPU {
            vram: [0; 0x2000],
            oam: [0; 0xA0],
            lcdc: 0x91,
            stat: 0x85,
            scy: 0x0,
            scx: 0x0,
            ly: 0x0,
            lyc: 0x0,
            bgp: 0xFC,
            obp0: 0x0,
            obp1: 0x0,
            wy: 0x0,
            wx: 0x0,
            mode: Mode::VBlank,
            dot: 0,
            frame_buffer: [0; SCREEN_W * SCREEN_H],
            bg_color: [0; SCREEN_W * SCREEN_H],
            stat_latch: false,
        }
    }

    pub fn get_fb(&self) -> [u8; SCREEN_W * SCREEN_H] {
        self.frame_buffer
    }

    fn lcd_off(&self) -> bool {
        (self.lcdc & 1 << 7) == 0
    }

    fn bg_window_enable(&self) -> bool {
        (self.lcdc & 1) != 0
    }

    fn window_enable(&self) -> bool {
        // The Window is visible (if enabled) when both coordinates are in the ranges WX=0..166, WY=0..143 respectively.
        // Values WX=7, WY=0 place the Window at the top left of the screen, completely covering the background.
        (self.lcdc & (1 << 5)) != 0
            && self.bg_window_enable()
            && self.ly >= self.wy
            && self.ly < SCREEN_H as u8
            && self.wx <= 166
    }

    fn obj_enable(&self) -> bool {
        (self.lcdc & (1 << 1)) != 0 && self.ly < SCREEN_H as u8
    }

    fn obj_size(&self) -> (u8, u8) {
        if self.lcdc & (1 << 2) != 0 {
            (8, 16)
        } else {
            (8, 8)
        }
    }

    fn bg_tile_map_area(&self) -> u16 {
        if self.lcdc & (1 << 3) != 0 {
            0x9C00
        } else {
            0x9800
        }
    }

    fn window_tile_map_area(&self) -> u16 {
        if self.lcdc & (1 << 6) != 0 {
            0x9C00
        } else {
            0x9800
        }
    }

    fn tile_data_unsigned_mode(&self) -> bool {
        self.lcdc & (1 << 4) != 0
    }

    fn tile_data_area(&self) -> u16 {
        if self.tile_data_unsigned_mode() {
            0x8000
        } else {
            0x9000
        }
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
        self.stat_latch = false;
        self.ly_lyc_check();
    }

    fn render_bg_scanline(&mut self) {
        if !self.bg_window_enable() {
            let current_line = self.ly as usize;
            self.frame_buffer[current_line * SCREEN_W..(current_line + 1) * SCREEN_W].fill(0);
            return;
        }

        let ly = self.ly as u16;
        let scy = self.scy as u16;
        let scx = self.scx as u16;

        let bg_y = (scy + ly) % 256;
        let tile_row = bg_y / 8;
        let pixel_row = bg_y % 8;

        for x in 0..SCREEN_W {
            let bg_x = (scx + x as u16) % 256;
            let tile_col = bg_x / 8;
            let pixel_col = bg_x % 8;

            let tile_map_addr = self.bg_tile_map_area() + (tile_row * 32 + tile_col);
            let tile_index = self.rb(tile_map_addr);

            let tile_addr = if self.tile_data_unsigned_mode() {
                self.tile_data_area() + (tile_index as u16) * 16
            } else {
                let signed_index = tile_index as i8 as i16;
                (self.tile_data_area() as i32 + (signed_index as i32) * 16) as u16
            } + (pixel_row * 2);

            let low = self.rb(tile_addr);
            let high = self.rb(tile_addr + 1);

            let bit = 7 - pixel_col;

            let px_idx: usize = self.ly as usize * SCREEN_W + x;

            let color_id = ((high >> bit) & 1) << 1 | ((low >> bit) & 1);

            self.bg_color[px_idx] = color_id;

            let shade = (self.bgp >> (color_id * 2)) & 0b11;

            self.frame_buffer[px_idx] = shade;
        }
    }

    fn render_window_scanline(&mut self) {
        if !self.window_enable() {
            return;
        }

        let win_x0 = self.wx as i16 - 7;

        let win_y = (self.ly - self.wy) as u16;
        let tile_row = win_y / 8;
        let pixel_row = win_y % 8;

        let start_x = win_x0.max(0) as usize;

        for x in start_x..SCREEN_W {
            let win_x = (x as i16 - win_x0) as u16;
            let tile_col = win_x / 8;
            let pixel_col = win_x % 8;

            let tile_map_addr = self.window_tile_map_area() + (tile_row * 32 + tile_col);
            let tile_index = self.rb(tile_map_addr);

            let tile_addr = if self.tile_data_unsigned_mode() {
                self.tile_data_area() + (tile_index as u16) * 16
            } else {
                let signed_index = tile_index as i8 as i16;
                (self.tile_data_area() as i32 + (signed_index as i32) * 16) as u16
            } + (pixel_row * 2);

            let low = self.rb(tile_addr);
            let high = self.rb(tile_addr + 1);

            let bit = 7 - pixel_col;

            let px_idx: usize = self.ly as usize * SCREEN_W + x;

            let color_id = ((high >> bit) & 1) << 1 | ((low >> bit) & 1);

            self.bg_color[px_idx] = color_id;

            let shade = (self.bgp >> (color_id * 2)) & 0b11;

            self.frame_buffer[px_idx] = shade;
        }
    }

    fn render_objects_scaline(&mut self) {
        if !self.obj_enable() {
            return;
        }

        let (obj_w, obj_h) = self.obj_size();

        let mut sprites_drawn = 0;

        for e in self.oam.chunks_exact(4) {
            let obj_y = e[0];
            let obj_x = e[1];
            let mut index = e[2];
            let attr = e[3];

            let sprite_y = obj_y.wrapping_sub(16);
            let sprite_x = obj_x.wrapping_sub(8);

            let line = self.ly.wrapping_sub(sprite_y);
            if line >= obj_h {
                continue;
            }

            // Attrs / Flags
            let x_flip = (attr & 0x20) != 0;
            let y_flip = (attr & 0x40) != 0;
            let priority = (attr & 0x80) != 0;
            let use_obp1 = (attr & 0x10) != 0;

            let palette = if use_obp1 { self.obp1 } else { self.obp0 };

            let mut pixel_row = if y_flip { obj_h - 1 - line } else { line };

            if obj_h == 16 {
                index = (index & 0xFE) + (pixel_row / 8);
                pixel_row %= 8;
            }

            let tile_addr = 0x8000 + (index as u16) * 16 + (pixel_row as u16) * 2;

            let low = self.rb(tile_addr);
            let high = self.rb(tile_addr + 1);

            for pixel_col in 0..obj_w {
                let screen_x = sprite_x as i16 + pixel_col as i16;
                if !(0..160).contains(&screen_x) {
                    continue;
                }

                let bit = if x_flip { pixel_col } else { 7 - pixel_col };

                let color_id = ((high >> bit) & 1) << 1 | ((low >> bit) & 1);

                // Transparency
                if color_id == 0 {
                    continue;
                }

                let px_idx: usize = self.ly as usize * SCREEN_W + screen_x as usize;

                if priority && self.bg_color[px_idx] != 0 {
                    continue;
                }

                let shade = (palette >> (color_id * 2)) & 0b11;

                self.frame_buffer[px_idx] = shade;
            }

            sprites_drawn += 1;

            // Hardware limitation
            if sprites_drawn >= MAX_SPRITES_PER_LINE {
                break;
            }
        }
    }

    pub fn tick(&mut self, cycles: TCycles) -> (u8, bool) {
        if self.lcd_off() {
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

            if self.ly == SCREEN_H as u8 {
                interrupts |= Interrupt::VBlank.bit();
            } else if self.ly == SCREEN_H as u8 + VBLANK_LINES {
                self.ly = 0;
                frame_ready = true;
            }

            if self.ly_lyc_check() {
                interrupts |= Interrupt::Stat.bit();
            }

            self.dot -= SCANLINE_END;
        }

        if self.ly >= SCREEN_H as u8 {
            self.set_mode(Mode::VBlank);
        } else if self.dot < OAM_END {
            self.set_mode(Mode::OamScan);
        } else if self.dot < DRAW_END {
            self.set_mode(Mode::Drawing);
        } else if self.mode != Mode::HBlank {
            self.set_mode(Mode::HBlank);
            self.render_bg_scanline();
            self.render_window_scanline();
            self.render_objects_scaline();
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
            LCDC_ADDR => {
                let was_on = !self.lcd_off();
                self.lcdc = value;
                if was_on && self.lcd_off() {
                    // Reset on  rising edge
                    self.reset();
                }
            }
            STAT_ADDR => self.stat = (self.stat & 0x07) | (value & 0x78) | 0x80, // Don't allow overwriting PPU mode and LYC == LY
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

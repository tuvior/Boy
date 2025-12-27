use crate::{gameboy::KeyStates, interrupt::Interrupt};

pub const JOYP_ADDR: u16 = 0xFF00;

pub struct Joypad {
    joyp: u8, // OxFF00 — P1/JOYP: Joypad
    states: KeyStates,
}

// JOYP [ - - 5 4 3 2 1 0 ]
// 5 — Select buttons
// 4 — Select d-pad
// 3 — Start / Down
// 2 — Select / Up
// 1 — B / Left
// 0 — A / Right

impl Joypad {
    pub fn new() -> Self {
        Joypad {
            joyp: 0xFF,
            states: KeyStates::default(),
        }
    }

    pub fn tick(&mut self, new_states: KeyStates) -> u8 {
        let mut interruts = 0;

        if (new_states.a && !self.states.a)
            || (new_states.b && !self.states.b)
            || (new_states.start && !self.states.start)
            || (new_states.select && !self.states.select)
            || (new_states.up && !self.states.up)
            || (new_states.down && !self.states.down)
            || (new_states.left && !self.states.left)
            || (new_states.right && !self.states.right)
        {
            interruts |= Interrupt::Joypad.bit();
        }

        self.states = new_states;

        interruts
    }

    pub fn rb(&self, addr: u16) -> u8 {
        match addr {
            JOYP_ADDR => self.build_joyp(),
            _ => unreachable!(),
        }
    }

    pub fn wb(&mut self, addr: u16, value: u8) {
        match addr {
            JOYP_ADDR => self.joyp = value & 0x30, // Drop lower nibble
            _ => unreachable!(),
        }
    }

    fn build_joyp(&self) -> u8 {
        match self.get_select_mode() {
            Mode::Buttons => self.joyp | self.build_buttons(),
            Mode::DPad => self.joyp | self.build_dpad(),
            Mode::All => self.joyp | (self.build_buttons() & self.build_dpad()),
            Mode::Release => self.joyp | 0xF,
        }
    }

    fn build_buttons(&self) -> u8 {
        (!self.states.start as u8) << 3
            | (!self.states.select as u8) << 2
            | (!self.states.b as u8) << 1
            | !self.states.a as u8
    }

    fn build_dpad(&self) -> u8 {
        (!self.states.down as u8) << 3
            | (!self.states.up as u8) << 2
            | (!self.states.left as u8) << 1
            | !self.states.right as u8
    }

    fn get_select_mode(&self) -> Mode {
        match self.joyp & 0x30 {
            0x00 => Mode::All,
            0x10 => Mode::Buttons,
            0x20 => Mode::DPad,
            0x30 => Mode::Release,
            _ => unreachable!(),
        }
    }
}

enum Mode {
    Buttons,
    DPad,
    All,
    Release,
}

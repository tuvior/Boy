use crate::{mmu::MMU, registers::Registers};

pub struct CPU {
    pub r: Registers,
    pub ime: bool, // IME: Interrupt master enable flag
}

// return value is MACHINE cycles.
type OP = fn(&mut CPU, &mut MMU) -> Cycles;
type Cycles = u8;

impl CPU {
    pub fn init() -> Self {
        let mut r = Registers::default();

        // This is the register state after the DMG Bios has run.
        // ref: [https://gbdev.io/pandocs/Power_Up_Sequence.html]
        r.set_af(0x01B0);
        r.set_bc(0x0013);
        r.set_de(0x00D8);
        r.set_hl(0x014D);
        r.sp = 0xFFFE;
        r.pc = 0x0100; // Program entrypoint

        Self { r, ime: false }
    }

    #[inline]
    fn pc_inc(&mut self, val: u16) {
        self.r.pc = self.r.pc.wrapping_add(val)
    }

    #[inline]
    fn pc_sub(&mut self, val: u16) {
        self.r.pc = self.r.pc.wrapping_sub(val)
    }

    #[inline]
    pub fn rb(&mut self, mmu: &MMU) -> u8 {
        let v = mmu.rb(self.r.pc);
        self.pc_inc(1);
        v
    }

    #[inline]
    pub fn rw(&mut self, mmu: &MMU) -> u16 {
        let v = mmu.rw(self.r.pc);
        self.pc_inc(2);
        v
    }

    pub fn step(&mut self, mmu: &mut MMU) -> Cycles {
        let op = self.rb(mmu);

        if op == 0xCB {
            let cb = self.rb(mmu);
            (CB_TABLE[cb as usize])(self, mmu)
        } else {
            (OP_TABLE[op as usize])(self, mmu)
        }
    }
}

fn op_unimp(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let pc = cpu.r.pc.wrapping_sub(1);
    let op = mmu.rb(pc);
    panic!("Unimplemented opcode 0x{:02X} at PC=0x{:04X}", op, pc);
}

fn cb_unimp(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let pc = cpu.r.pc.wrapping_sub(1);
    let op = mmu.rb(pc);
    panic!("Unimplemented CB opcode 0x{:02X} at PC=0x{:04X}", op, pc);
}

fn nop(_: &mut CPU, _: &mut MMU) -> Cycles {
    1
}

// ld r16, imm16

fn ld_bc_d16(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let v = mmu.rw(cpu.r.pc);
    cpu.r.set_bc(v);
    3
}

fn ld_de_d16(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let v = cpu.rw(mmu);
    cpu.r.set_de(v);
    3
}

fn ld_hl_d16(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let v = cpu.rw(mmu);
    cpu.r.set_hl(v);
    3
}

fn ld_sp_d16(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let v = cpu.rw(mmu);
    cpu.r.sp = v;
    3
}

pub const OP_TABLE: [OP; 256] = {
    let mut t: [OP; 256] = [op_unimp; 256];

    t[0x00] = nop; // NOP
    t[0x01] = ld_bc_d16; // LD BC, d16
    t[0x11] = ld_de_d16; // LD DE, d16
    t[0x21] = ld_hl_d16; // LD HL, d16
    t[0x31] = ld_sp_d16; // LD SP, d16

    t
};

pub const CB_TABLE: [OP; 256] = {
    let mut t: [OP; 256] = [cb_unimp; 256];

    t
};

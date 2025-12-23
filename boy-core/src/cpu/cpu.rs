use core::panic;

use crate::cpu::instructions::*;
use crate::interrupt::{INTERRUPT_CYCLES, highest_priority};
use crate::{mmu::MMU, registers::Registers};

pub struct CPU {
    pub r: Registers,
    pub ime: bool, // IME: Interrupt master enable flag
    ime_delay: u8,
    halted: bool,
    stopped: bool,
}

// return value is MACHINE cycles.
type OP = fn(&mut CPU, &mut MMU) -> Cycles;
pub type Cycles = u8;

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

        Self {
            r,
            ime: false,
            ime_delay: 0,
            halted: false,
            stopped: false,
        }
    }

    pub fn step(&mut self, mmu: &mut MMU) -> Cycles {
        if self.halted {
            if mmu.pending_interrupts() != 0 {
                self.halted = false;
            } else {
                return 1;
            }
        }

        // stubbed
        if self.stopped {
            return 1;
        }

        if self.ime
            && let Some(cycles) = self.service_interrupts(mmu)
        {
            return cycles;
        }

        let op = self.rb(mmu);

        let cycles = if op == 0xCB {
            let cb = self.rb(mmu);
            (CB_TABLE[cb as usize])(self, mmu)
        } else {
            (OP_TABLE[op as usize])(self, mmu)
        };

        // The effect of ei is delayed by one instruction.
        // This means that ei followed immediately by di does not allow any interrupts between them.
        if self.ime_delay > 0 {
            self.ime_delay -= 1;
            if self.ime_delay == 0 {
                self.ime = true;
            }
        }

        cycles
    }

    fn service_interrupts(&mut self, mmu: &mut MMU) -> Option<Cycles> {
        let pending = mmu.pending_interrupts();
        let interrupt = highest_priority(pending)?;

        self.ime = false;
        mmu.clear_interrupt(interrupt.bit());
        call(self, mmu, interrupt.vector());

        Some(INTERRUPT_CYCLES)
    }

    pub fn pc_inc(&mut self, val: u16) {
        self.r.pc = self.r.pc.wrapping_add(val)
    }

    pub fn sp_inc(&mut self, val: u16) {
        self.r.sp = self.r.sp.wrapping_add(val)
    }

    pub fn sp_sub(&mut self, val: u16) {
        self.r.sp = self.r.sp.wrapping_sub(val)
    }

    pub fn rb(&mut self, mmu: &MMU) -> u8 {
        let v = mmu.rb(self.r.pc);
        self.pc_inc(1);
        v
    }

    pub fn rw(&mut self, mmu: &MMU) -> u16 {
        let v = mmu.rw(self.r.pc);
        self.pc_inc(2);
        v
    }

    pub fn push(&mut self, mmu: &mut MMU, val: u16) {
        self.sp_sub(2);
        mmu.ww(self.r.sp, val);
    }

    pub fn pop(&mut self, mmu: &mut MMU) -> u16 {
        let val = mmu.rw(self.r.sp);
        self.sp_inc(2);
        val
    }

    pub fn set_flags(&mut self, z: bool, n: bool, h: bool, c: bool) {
        self.r.set_z(z);
        self.r.set_n(n);
        self.r.set_h(h);
        self.r.set_c(c);
    }

    pub fn set_ime_pending(&mut self, val: bool) {
        self.ime_delay = if val { 2 } else { 0 };
    }

    pub fn stop(&mut self) {
        self.stopped = true;
    }

    pub fn halt(&mut self) {
        self.halted = true;
    }
}

pub const OP_TABLE: [OP; 256] = {
    let mut t: [OP; 256] = [op_xxx; 256];

    // 0x00-0x0F
    t[0x00] = op_nop; // NOP
    t[0x01] = op_ld_bc_d16; // LD BC, d16
    t[0x02] = op_ld_bc_a; // LD (BC), A
    t[0x03] = op_inc_bc; // INC BC
    t[0x04] = op_inc_b; // INC B
    t[0x05] = op_dec_b; // DEC B
    t[0x06] = op_ld_b_d8; // LD B, d8
    t[0x07] = op_rlca; // RLCA
    t[0x08] = op_ld_a16_sp; // LD (a16), SP
    t[0x09] = op_add_hl_bc; // ADD HL, BC
    t[0x0A] = op_ld_a_bc; // LD A, (BC)
    t[0x0B] = op_dec_bc; // DEC BC
    t[0x0C] = op_inc_c; // INC C
    t[0x0D] = op_dec_c; // DEC C
    t[0x0E] = op_ld_c_d8; // LD C, d8
    t[0x0F] = op_rrca; // RRCA

    // 0x10-0x1F
    t[0x10] = op_stop; // STOP
    t[0x11] = op_ld_de_d16; // LD DE, d16
    t[0x12] = op_ld_de_a; // LD (DE), A
    t[0x13] = op_inc_de; // INC DE
    t[0x14] = op_inc_d; // INC D
    t[0x15] = op_dec_d; // DEC D
    t[0x16] = op_ld_d_d8; // LD D, d8
    t[0x17] = op_rla; // RLA
    t[0x18] = op_jr; // JR r8
    t[0x19] = op_add_hl_de; // ADD HL, DE
    t[0x1A] = op_ld_a_de; // LD A, (DE)
    t[0x1B] = op_dec_de; // DEC DE
    t[0x1C] = op_inc_e; // INC E
    t[0x1D] = op_dec_e; // DEC E
    t[0x1E] = op_ld_e_d8; // LD E, d8
    t[0x1F] = op_rra; // RRA

    // 0x20-0x2F
    t[0x20] = op_jr_nz; // JR NZ, r8
    t[0x21] = op_ld_hl_d16; // LD HL, d16
    t[0x22] = op_ld_hli_a; // LD (HL+), A
    t[0x23] = op_inc_hl; // INC HL
    t[0x24] = op_inc_h; // INC H
    t[0x25] = op_dec_h; // DEC H
    t[0x26] = op_ld_h_d8; // LD H, d8
    t[0x27] = op_daa; // DAA
    t[0x28] = op_jr_z; // JR Z, r8
    t[0x29] = op_add_hl_hl; // ADD HL, HL
    t[0x2A] = op_ld_a_hli; // LD A, (HL+)
    t[0x2B] = op_dec_hl; // DEC HL
    t[0x2C] = op_inc_l; // INC L
    t[0x2D] = op_dec_l; // DEC L
    t[0x2E] = op_ld_l_d8; // LD L, d8
    t[0x2F] = op_cpl; // CPL

    // 0x30-0x3F
    t[0x30] = op_jr_nc; // JR NC, r8
    t[0x31] = op_ld_sp_d16; // LD SP, d16
    t[0x32] = op_ld_hld_a; // LD (HL-), A
    t[0x33] = op_inc_sp; // INC SP
    t[0x34] = op_inc_hl_ptr; // INC (HL)
    t[0x35] = op_dec_hl_ptr; // DEC (HL)
    t[0x36] = op_ld_hl_d8; // LD (HL), d8
    t[0x37] = op_scf; // SCF
    t[0x38] = op_jr_c; // JR C, r8
    t[0x39] = op_add_hl_sp; // ADD HL, SP
    t[0x3A] = op_ld_a_hld; // LD A, (HL-)
    t[0x3B] = op_dec_sp; // DEC SP
    t[0x3C] = op_inc_a; // INC A
    t[0x3D] = op_dec_a; // DEC A
    t[0x3E] = op_ld_a_d8; // LD A, d8
    t[0x3F] = op_ccf; // CCF

    // 0x40-0x4F
    t[0x40] = op_ld_b_b; // LD B, B
    t[0x41] = op_ld_b_c; // LD B, C
    t[0x42] = op_ld_b_d; // LD B, D
    t[0x43] = op_ld_b_e; // LD B, E
    t[0x44] = op_ld_b_h; // LD B, H
    t[0x45] = op_ld_b_l; // LD B, L
    t[0x46] = op_ld_b_hl; // LD B, (HL)
    t[0x47] = op_ld_b_a; // LD B, A
    t[0x48] = op_ld_c_b; // LD C, B
    t[0x49] = op_ld_c_c; // LD C, C
    t[0x4A] = op_ld_c_d; // LD C, D
    t[0x4B] = op_ld_c_e; // LD C, E
    t[0x4C] = op_ld_c_h; // LD C, H
    t[0x4D] = op_ld_c_l; // LD C, L
    t[0x4E] = op_ld_c_hl; // LD C, (HL)
    t[0x4F] = op_ld_c_a; // LD C, A

    // 0x50-0x5F
    t[0x50] = op_ld_d_b; // LD D, B
    t[0x51] = op_ld_d_c; // LD D, C
    t[0x52] = op_ld_d_d; // LD D, D
    t[0x53] = op_ld_d_e; // LD D, E
    t[0x54] = op_ld_d_h; // LD D, H
    t[0x55] = op_ld_d_l; // LD D, L
    t[0x56] = op_ld_d_hl; // LD D, (HL)
    t[0x57] = op_ld_d_a; // LD D, A
    t[0x58] = op_ld_e_b; // LD E, B
    t[0x59] = op_ld_e_c; // LD E, C
    t[0x5A] = op_ld_e_d; // LD E, D
    t[0x5B] = op_ld_e_e; // LD E, E
    t[0x5C] = op_ld_e_h; // LD E, H
    t[0x5D] = op_ld_e_l; // LD E, L
    t[0x5E] = op_ld_e_hl; // LD E, (HL)
    t[0x5F] = op_ld_e_a; // LD E, A

    // 0x60-0x6F
    t[0x60] = op_ld_h_b; // LD H, B
    t[0x61] = op_ld_h_c; // LD H, C
    t[0x62] = op_ld_h_d; // LD H, D
    t[0x63] = op_ld_h_e; // LD H, E
    t[0x64] = op_ld_h_h; // LD H, H
    t[0x65] = op_ld_h_l; // LD H, L
    t[0x66] = op_ld_h_hl; // LD H, (HL)
    t[0x67] = op_ld_h_a; // LD H, A
    t[0x68] = op_ld_l_b; // LD L, B
    t[0x69] = op_ld_l_c; // LD L, C
    t[0x6A] = op_ld_l_d; // LD L, D
    t[0x6B] = op_ld_l_e; // LD L, E
    t[0x6C] = op_ld_l_h; // LD L, H
    t[0x6D] = op_ld_l_l; // LD L, L
    t[0x6E] = op_ld_l_hl; // LD L, (HL)
    t[0x6F] = op_ld_l_a; // LD L, A

    // 0x70-0x7F
    t[0x70] = op_ld_hl_b; // LD (HL), B
    t[0x71] = op_ld_hl_c; // LD (HL), C
    t[0x72] = op_ld_hl_d; // LD (HL), D
    t[0x73] = op_ld_hl_e; // LD (HL), E
    t[0x74] = op_ld_hl_h; // LD (HL), H
    t[0x75] = op_ld_hl_l; // LD (HL), L
    t[0x76] = op_halt; // HALT
    t[0x77] = op_ld_hl_a; // LD (HL), A
    t[0x78] = op_ld_a_b; // LD A, B
    t[0x79] = op_ld_a_c; // LD A, C
    t[0x7A] = op_ld_a_d; // LD A, D
    t[0x7B] = op_ld_a_e; // LD A, E
    t[0x7C] = op_ld_a_h; // LD A, H
    t[0x7D] = op_ld_a_l; // LD A, L
    t[0x7E] = op_ld_a_hl; // LD A, (HL)
    t[0x7F] = op_ld_a_a; // LD A, A

    // 0x80-0x8F
    t[0x80] = op_add_a_b; // ADD A, B
    t[0x81] = op_add_a_c; // ADD A, C
    t[0x82] = op_add_a_d; // ADD A, D
    t[0x83] = op_add_a_e; // ADD A, E
    t[0x84] = op_add_a_h; // ADD A, H
    t[0x85] = op_add_a_l; // ADD A, L
    t[0x86] = op_add_a_hl; // ADD A, (HL)
    t[0x87] = op_add_a_a; // ADD A, A
    t[0x88] = op_adc_a_b; // ADC A, B
    t[0x89] = op_adc_a_c; // ADC A, C
    t[0x8A] = op_adc_a_d; // ADC A, D
    t[0x8B] = op_adc_a_e; // ADC A, E
    t[0x8C] = op_adc_a_h; // ADC A, H
    t[0x8D] = op_adc_a_l; // ADC A, L
    t[0x8E] = op_adc_a_hl; // ADC A, (HL)
    t[0x8F] = op_adc_a_a; // ADC A, A

    // 0x90-0x9F
    t[0x90] = op_sub_b; // SUB B
    t[0x91] = op_sub_c; // SUB C
    t[0x92] = op_sub_d; // SUB D
    t[0x93] = op_sub_e; // SUB E
    t[0x94] = op_sub_h; // SUB H
    t[0x95] = op_sub_l; // SUB L
    t[0x96] = op_sub_hl; // SUB (HL)
    t[0x97] = op_sub_a; // SUB A
    t[0x98] = op_sbc_a_b; // SBC A, B
    t[0x99] = op_sbc_a_c; // SBC A, C
    t[0x9A] = op_sbc_a_d; // SBC A, D
    t[0x9B] = op_sbc_a_e; // SBC A, E
    t[0x9C] = op_sbc_a_h; // SBC A, H
    t[0x9D] = op_sbc_a_l; // SBC A, L
    t[0x9E] = op_sbc_a_hl; // SBC A, (HL)
    t[0x9F] = op_sbc_a_a; // SBC A, A

    // 0xA0-0xAF
    t[0xA0] = op_and_b; // AND B
    t[0xA1] = op_and_c; // AND C
    t[0xA2] = op_and_d; // AND D
    t[0xA3] = op_and_e; // AND E
    t[0xA4] = op_and_h; // AND H
    t[0xA5] = op_and_l; // AND L
    t[0xA6] = op_and_hl; // AND (HL)
    t[0xA7] = op_and_a; // AND A
    t[0xA8] = op_xor_b; // XOR B
    t[0xA9] = op_xor_c; // XOR C
    t[0xAA] = op_xor_d; // XOR D
    t[0xAB] = op_xor_e; // XOR E
    t[0xAC] = op_xor_h; // XOR H
    t[0xAD] = op_xor_l; // XOR L
    t[0xAE] = op_xor_hl; // XOR (HL)
    t[0xAF] = op_xor_a; // XOR A

    // 0xB0-0xBF
    t[0xB0] = op_or_b; // OR B
    t[0xB1] = op_or_c; // OR C
    t[0xB2] = op_or_d; // OR D
    t[0xB3] = op_or_e; // OR E
    t[0xB4] = op_or_h; // OR H
    t[0xB5] = op_or_l; // OR L
    t[0xB6] = op_or_hl; // OR (HL)
    t[0xB7] = op_or_a; // OR A
    t[0xB8] = op_cp_b; // CP B
    t[0xB9] = op_cp_c; // CP C
    t[0xBA] = op_cp_d; // CP D
    t[0xBB] = op_cp_e; // CP E
    t[0xBC] = op_cp_h; // CP H
    t[0xBD] = op_cp_l; // CP L
    t[0xBE] = op_cp_hl; // CP (HL)
    t[0xBF] = op_cp_a; // CP A

    // 0xC0-0xCF
    t[0xC0] = op_ret_nz; // RET NZ
    t[0xC1] = op_pop_bc; // POP BC
    t[0xC2] = op_jp_nz; // JP NZ, a16
    t[0xC3] = op_jp_a16; // JP a16
    t[0xC4] = op_call_nz; // CALL NZ, a16
    t[0xC5] = op_push_bc; // PUSH BC
    t[0xC6] = op_add_a_d8; // ADD A, d8
    t[0xC7] = op_rst_00; // RST 00H
    t[0xC8] = op_ret_z; // RET Z
    t[0xC9] = op_ret; // RET
    t[0xCA] = op_jp_z; // JP Z, a16
    t[0xCB] = |_, _| panic!("Trying to call 0xCB as OP."); // This should never be called
    t[0xCC] = op_call_z; // CALL Z, a16
    t[0xCD] = op_call_a16; // CALL a16
    t[0xCE] = op_adc_a_d8; // ADC A, d8
    t[0xCF] = op_rst_08; // RST 08H

    // 0xD0-0xDF
    t[0xD0] = op_ret_nc; // RET NC
    t[0xD1] = op_pop_de; // POP DE
    t[0xD2] = op_jp_nc; // JP NC, a16
    t[0xD3] = op_xxx; // UNUSED
    t[0xD4] = op_call_nc; // CALL NC, a16
    t[0xD5] = op_push_de; // PUSH DE
    t[0xD6] = op_sub_d8; // SUB d8
    t[0xD7] = op_rst_10; // RST 10H
    t[0xD8] = op_ret_c; // RET C
    t[0xD9] = op_reti; // RETI
    t[0xDA] = op_jp_c; // JP C, a16
    t[0xDB] = op_xxx; // UNUSED
    t[0xDC] = op_call_c; // CALL C, a16
    t[0xDD] = op_xxx; // UNUSED
    t[0xDE] = op_sbc_a_d8; // SBC A, d8
    t[0xDF] = op_rst_18; // RST 18H

    // 0xE0-0xEF
    t[0xE0] = op_ldh_a8_a; // LDH (a8), A
    t[0xE1] = op_pop_hl; // POP HL
    t[0xE2] = op_ld_ff00_c_a; // LD (C), A
    t[0xE3] = op_xxx; // UNUSED
    t[0xE4] = op_xxx; // UNUSED
    t[0xE5] = op_push_hl; // PUSH HL
    t[0xE6] = op_and_d8; // AND d8
    t[0xE7] = op_rst_20; // RST 20H
    t[0xE8] = op_add_sp_r8; // ADD SP, r8
    t[0xE9] = op_jp_hl; // JP (HL)
    t[0xEA] = op_ld_a16_a; // LD (a16), A
    t[0xEB] = op_xxx; // UNUSED
    t[0xEC] = op_xxx; // UNUSED
    t[0xED] = op_xxx; // UNUSED
    t[0xEE] = op_xor_d8; // XOR d8
    t[0xEF] = op_rst_28; // RST 28H

    // 0xF0-0xFF
    t[0xF0] = op_ldh_a_a8; // LDH A, (a8)
    t[0xF1] = op_pop_af; // POP AF
    t[0xF2] = op_ld_a_ff00_c; // LD A, (C)
    t[0xF3] = op_di; // DI
    t[0xF4] = op_xxx; // UNUSED
    t[0xF5] = op_push_af; // PUSH AF
    t[0xF6] = op_or_d8; // OR d8
    t[0xF7] = op_rst_30; // RST 30H
    t[0xF8] = op_ld_hl_sp_r8; // LD HL, SP+r8
    t[0xF9] = op_ld_sp_hl; // LD SP, HL
    t[0xFA] = op_ld_a_a16; // LD A, (a16)
    t[0xFB] = op_ei; // EI
    t[0xFC] = op_xxx; // UNUSED
    t[0xFD] = op_xxx; // UNUSED
    t[0xFE] = op_cp_d8; // CP d8
    t[0xFF] = op_rst_38; // RST 38H

    t
};

pub const CB_TABLE: [OP; 256] = {
    let mut t: [OP; 256] = [op_xxx; 256];

    // 0x00-0x0F
    t[0x00] = cb_rlc_b; // RLC B
    t[0x01] = cb_rlc_c; // RLC C
    t[0x02] = cb_rlc_d; // RLC D
    t[0x03] = cb_rlc_e; // RLC E
    t[0x04] = cb_rlc_h; // RLC H
    t[0x05] = cb_rlc_l; // RLC L
    t[0x06] = cb_rlc_hl; // RLC (HL)
    t[0x07] = cb_rlc_a; // RLC A
    t[0x08] = cb_rrc_b; // RRC B
    t[0x09] = cb_rrc_c; // RRC C
    t[0x0A] = cb_rrc_d; // RRC D
    t[0x0B] = cb_rrc_e; // RRC E
    t[0x0C] = cb_rrc_h; // RRC H
    t[0x0D] = cb_rrc_l; // RRC L
    t[0x0E] = cb_rrc_hl; // RRC (HL)
    t[0x0F] = cb_rrc_a; // RRC A

    // 0x10-0x1F
    t[0x10] = cb_rl_b; // RL B
    t[0x11] = cb_rl_c; // RL C
    t[0x12] = cb_rl_d; // RL D
    t[0x13] = cb_rl_e; // RL E
    t[0x14] = cb_rl_h; // RL H
    t[0x15] = cb_rl_l; // RL L
    t[0x16] = cb_rl_hl; // RL (HL)
    t[0x17] = cb_rl_a; // RL A
    t[0x18] = cb_rr_b; // RR B
    t[0x19] = cb_rr_c; // RR C
    t[0x1A] = cb_rr_d; // RR D
    t[0x1B] = cb_rr_e; // RR E
    t[0x1C] = cb_rr_h; // RR H
    t[0x1D] = cb_rr_l; // RR L
    t[0x1E] = cb_rr_hl; // RR (HL)
    t[0x1F] = cb_rr_a; // RR A

    // 0x20-0x2F
    t[0x20] = cb_sla_b; // SLA B
    t[0x21] = cb_sla_c; // SLA C
    t[0x22] = cb_sla_d; // SLA D
    t[0x23] = cb_sla_e; // SLA E
    t[0x24] = cb_sla_h; // SLA H
    t[0x25] = cb_sla_l; // SLA L
    t[0x26] = cb_sla_hl; // SLA (HL)
    t[0x27] = cb_sla_a; // SLA A
    t[0x28] = cb_sra_b; // SRA B
    t[0x29] = cb_sra_c; // SRA C
    t[0x2A] = cb_sra_d; // SRA D
    t[0x2B] = cb_sra_e; // SRA E
    t[0x2C] = cb_sra_h; // SRA H
    t[0x2D] = cb_sra_l; // SRA L
    t[0x2E] = cb_sra_hl; // SRA (HL)
    t[0x2F] = cb_sra_a; // SRA A

    // 0x30-0x3F
    t[0x30] = cb_swap_b; // SWAP B
    t[0x31] = cb_swap_c; // SWAP C
    t[0x32] = cb_swap_d; // SWAP D
    t[0x33] = cb_swap_e; // SWAP E
    t[0x34] = cb_swap_h; // SWAP H
    t[0x35] = cb_swap_l; // SWAP L
    t[0x36] = cb_swap_hl; // SWAP (HL)
    t[0x37] = cb_swap_a; // SWAP A
    t[0x38] = cb_srl_b; // SRL B
    t[0x39] = cb_srl_c; // SRL C
    t[0x3A] = cb_srl_d; // SRL D
    t[0x3B] = cb_srl_e; // SRL E
    t[0x3C] = cb_srl_h; // SRL H
    t[0x3D] = cb_srl_l; // SRL L
    t[0x3E] = cb_srl_hl; // SRL (HL)
    t[0x3F] = cb_srl_a; // SRL A

    // 0x40-0x4F
    t[0x40] = cb_bit_0_b; // BIT 0, B
    t[0x41] = cb_bit_0_c; // BIT 0, C
    t[0x42] = cb_bit_0_d; // BIT 0, D
    t[0x43] = cb_bit_0_e; // BIT 0, E
    t[0x44] = cb_bit_0_h; // BIT 0, H
    t[0x45] = cb_bit_0_l; // BIT 0, L
    t[0x46] = cb_bit_0_hl; // BIT 0, (HL)
    t[0x47] = cb_bit_0_a; // BIT 0, A
    t[0x48] = cb_bit_1_b; // BIT 1, B
    t[0x49] = cb_bit_1_c; // BIT 1, C
    t[0x4A] = cb_bit_1_d; // BIT 1, D
    t[0x4B] = cb_bit_1_e; // BIT 1, E
    t[0x4C] = cb_bit_1_h; // BIT 1, H
    t[0x4D] = cb_bit_1_l; // BIT 1, L
    t[0x4E] = cb_bit_1_hl; // BIT 1, (HL)
    t[0x4F] = cb_bit_1_a; // BIT 1, A

    // 0x50-0x5F
    t[0x50] = cb_bit_2_b; // BIT 2, B
    t[0x51] = cb_bit_2_c; // BIT 2, C
    t[0x52] = cb_bit_2_d; // BIT 2, D
    t[0x53] = cb_bit_2_e; // BIT 2, E
    t[0x54] = cb_bit_2_h; // BIT 2, H
    t[0x55] = cb_bit_2_l; // BIT 2, L
    t[0x56] = cb_bit_2_hl; // BIT 2, (HL)
    t[0x57] = cb_bit_2_a; // BIT 2, A
    t[0x58] = cb_bit_3_b; // BIT 3, B
    t[0x59] = cb_bit_3_c; // BIT 3, C
    t[0x5A] = cb_bit_3_d; // BIT 3, D
    t[0x5B] = cb_bit_3_e; // BIT 3, E
    t[0x5C] = cb_bit_3_h; // BIT 3, H
    t[0x5D] = cb_bit_3_l; // BIT 3, L
    t[0x5E] = cb_bit_3_hl; // BIT 3, (HL)
    t[0x5F] = cb_bit_3_a; // BIT 3, A

    // 0x60-0x6F
    t[0x60] = cb_bit_4_b; // BIT 4, B
    t[0x61] = cb_bit_4_c; // BIT 4, C
    t[0x62] = cb_bit_4_d; // BIT 4, D
    t[0x63] = cb_bit_4_e; // BIT 4, E
    t[0x64] = cb_bit_4_h; // BIT 4, H
    t[0x65] = cb_bit_4_l; // BIT 4, L
    t[0x66] = cb_bit_4_hl; // BIT 4, (HL)
    t[0x67] = cb_bit_4_a; // BIT 4, A
    t[0x68] = cb_bit_5_b; // BIT 5, B
    t[0x69] = cb_bit_5_c; // BIT 5, C
    t[0x6A] = cb_bit_5_d; // BIT 5, D
    t[0x6B] = cb_bit_5_e; // BIT 5, E
    t[0x6C] = cb_bit_5_h; // BIT 5, H
    t[0x6D] = cb_bit_5_l; // BIT 5, L
    t[0x6E] = cb_bit_5_hl; // BIT 5, (HL)
    t[0x6F] = cb_bit_5_a; // BIT 5, A

    // 0x70-0x7F
    t[0x70] = cb_bit_6_b; // BIT 6, B
    t[0x71] = cb_bit_6_c; // BIT 6, C
    t[0x72] = cb_bit_6_d; // BIT 6, D
    t[0x73] = cb_bit_6_e; // BIT 6, E
    t[0x74] = cb_bit_6_h; // BIT 6, H
    t[0x75] = cb_bit_6_l; // BIT 6, L
    t[0x76] = cb_bit_6_hl; // BIT 6, (HL)
    t[0x77] = cb_bit_6_a; // BIT 6, A
    t[0x78] = cb_bit_7_b; // BIT 7, B
    t[0x79] = cb_bit_7_c; // BIT 7, C
    t[0x7A] = cb_bit_7_d; // BIT 7, D
    t[0x7B] = cb_bit_7_e; // BIT 7, E
    t[0x7C] = cb_bit_7_h; // BIT 7, H
    t[0x7D] = cb_bit_7_l; // BIT 7, L
    t[0x7E] = cb_bit_7_hl; // BIT 7, (HL)
    t[0x7F] = cb_bit_7_a; // BIT 7, A

    // 0x80-0x8F
    t[0x80] = cb_res_0_b; // RES 0, B
    t[0x81] = cb_res_0_c; // RES 0, C
    t[0x82] = cb_res_0_d; // RES 0, D
    t[0x83] = cb_res_0_e; // RES 0, E
    t[0x84] = cb_res_0_h; // RES 0, H
    t[0x85] = cb_res_0_l; // RES 0, L
    t[0x86] = cb_res_0_hl; // RES 0, (HL)
    t[0x87] = cb_res_0_a; // RES 0, A
    t[0x88] = cb_res_1_b; // RES 1, B
    t[0x89] = cb_res_1_c; // RES 1, C
    t[0x8A] = cb_res_1_d; // RES 1, D
    t[0x8B] = cb_res_1_e; // RES 1, E
    t[0x8C] = cb_res_1_h; // RES 1, H
    t[0x8D] = cb_res_1_l; // RES 1, L
    t[0x8E] = cb_res_1_hl; // RES 1, (HL)
    t[0x8F] = cb_res_1_a; // RES 1, A

    // 0x90-0x9F
    t[0x90] = cb_res_2_b; // RES 2, B
    t[0x91] = cb_res_2_c; // RES 2, C
    t[0x92] = cb_res_2_d; // RES 2, D
    t[0x93] = cb_res_2_e; // RES 2, E
    t[0x94] = cb_res_2_h; // RES 2, H
    t[0x95] = cb_res_2_l; // RES 2, L
    t[0x96] = cb_res_2_hl; // RES 2, (HL)
    t[0x97] = cb_res_2_a; // RES 2, A
    t[0x98] = cb_res_3_b; // RES 3, B
    t[0x99] = cb_res_3_c; // RES 3, C
    t[0x9A] = cb_res_3_d; // RES 3, D
    t[0x9B] = cb_res_3_e; // RES 3, E
    t[0x9C] = cb_res_3_h; // RES 3, H
    t[0x9D] = cb_res_3_l; // RES 3, L
    t[0x9E] = cb_res_3_hl; // RES 3, (HL)
    t[0x9F] = cb_res_3_a; // RES 3, A

    // 0xA0-0xAF
    t[0xA0] = cb_res_4_b; // RES 4, B
    t[0xA1] = cb_res_4_c; // RES 4, C
    t[0xA2] = cb_res_4_d; // RES 4, D
    t[0xA3] = cb_res_4_e; // RES 4, E
    t[0xA4] = cb_res_4_h; // RES 4, H
    t[0xA5] = cb_res_4_l; // RES 4, L
    t[0xA6] = cb_res_4_hl; // RES 4, (HL)
    t[0xA7] = cb_res_4_a; // RES 4, A
    t[0xA8] = cb_res_5_b; // RES 5, B
    t[0xA9] = cb_res_5_c; // RES 5, C
    t[0xAA] = cb_res_5_d; // RES 5, D
    t[0xAB] = cb_res_5_e; // RES 5, E
    t[0xAC] = cb_res_5_h; // RES 5, H
    t[0xAD] = cb_res_5_l; // RES 5, L
    t[0xAE] = cb_res_5_hl; // RES 5, (HL)
    t[0xAF] = cb_res_5_a; // RES 5, A

    // 0xB0-0xBF
    t[0xB0] = cb_res_6_b; // RES 6, B
    t[0xB1] = cb_res_6_c; // RES 6, C
    t[0xB2] = cb_res_6_d; // RES 6, D
    t[0xB3] = cb_res_6_e; // RES 6, E
    t[0xB4] = cb_res_6_h; // RES 6, H
    t[0xB5] = cb_res_6_l; // RES 6, L
    t[0xB6] = cb_res_6_hl; // RES 6, (HL)
    t[0xB7] = cb_res_6_a; // RES 6, A
    t[0xB8] = cb_res_7_b; // RES 7, B
    t[0xB9] = cb_res_7_c; // RES 7, C
    t[0xBA] = cb_res_7_d; // RES 7, D
    t[0xBB] = cb_res_7_e; // RES 7, E
    t[0xBC] = cb_res_7_h; // RES 7, H
    t[0xBD] = cb_res_7_l; // RES 7, L
    t[0xBE] = cb_res_7_hl; // RES 7, (HL)
    t[0xBF] = cb_res_7_a; // RES 7, A

    // 0xC0-0xCF
    t[0xC0] = cb_set_0_b; // SET 0, B
    t[0xC1] = cb_set_0_c; // SET 0, C
    t[0xC2] = cb_set_0_d; // SET 0, D
    t[0xC3] = cb_set_0_e; // SET 0, E
    t[0xC4] = cb_set_0_h; // SET 0, H
    t[0xC5] = cb_set_0_l; // SET 0, L
    t[0xC6] = cb_set_0_hl; // SET 0, (HL)
    t[0xC7] = cb_set_0_a; // SET 0, A
    t[0xC8] = cb_set_1_b; // SET 1, B
    t[0xC9] = cb_set_1_c; // SET 1, C
    t[0xCA] = cb_set_1_d; // SET 1, D
    t[0xCB] = cb_set_1_e; // SET 1, E
    t[0xCC] = cb_set_1_h; // SET 1, H
    t[0xCD] = cb_set_1_l; // SET 1, L
    t[0xCE] = cb_set_1_hl; // SET 1, (HL)
    t[0xCF] = cb_set_1_a; // SET 1, A

    // 0xD0-0xDF
    t[0xD0] = cb_set_2_b; // SET 2, B
    t[0xD1] = cb_set_2_c; // SET 2, C
    t[0xD2] = cb_set_2_d; // SET 2, D
    t[0xD3] = cb_set_2_e; // SET 2, E
    t[0xD4] = cb_set_2_h; // SET 2, H
    t[0xD5] = cb_set_2_l; // SET 2, L
    t[0xD6] = cb_set_2_hl; // SET 2, (HL)
    t[0xD7] = cb_set_2_a; // SET 2, A
    t[0xD8] = cb_set_3_b; // SET 3, B
    t[0xD9] = cb_set_3_c; // SET 3, C
    t[0xDA] = cb_set_3_d; // SET 3, D
    t[0xDB] = cb_set_3_e; // SET 3, E
    t[0xDC] = cb_set_3_h; // SET 3, H
    t[0xDD] = cb_set_3_l; // SET 3, L
    t[0xDE] = cb_set_3_hl; // SET 3, (HL)
    t[0xDF] = cb_set_3_a; // SET 3, A

    // 0xE0-0xEF
    t[0xE0] = cb_set_4_b; // SET 4, B
    t[0xE1] = cb_set_4_c; // SET 4, C
    t[0xE2] = cb_set_4_d; // SET 4, D
    t[0xE3] = cb_set_4_e; // SET 4, E
    t[0xE4] = cb_set_4_h; // SET 4, H
    t[0xE5] = cb_set_4_l; // SET 4, L
    t[0xE6] = cb_set_4_hl; // SET 4, (HL)
    t[0xE7] = cb_set_4_a; // SET 4, A
    t[0xE8] = cb_set_5_b; // SET 5, B
    t[0xE9] = cb_set_5_c; // SET 5, C
    t[0xEA] = cb_set_5_d; // SET 5, D
    t[0xEB] = cb_set_5_e; // SET 5, E
    t[0xEC] = cb_set_5_h; // SET 5, H
    t[0xED] = cb_set_5_l; // SET 5, L
    t[0xEE] = cb_set_5_hl; // SET 5, (HL)
    t[0xEF] = cb_set_5_a; // SET 5, A

    // 0xF0-0xFF
    t[0xF0] = cb_set_6_b; // SET 6, B
    t[0xF1] = cb_set_6_c; // SET 6, C
    t[0xF2] = cb_set_6_d; // SET 6, D
    t[0xF3] = cb_set_6_e; // SET 6, E
    t[0xF4] = cb_set_6_h; // SET 6, H
    t[0xF5] = cb_set_6_l; // SET 6, L
    t[0xF6] = cb_set_6_hl; // SET 6, (HL)
    t[0xF7] = cb_set_6_a; // SET 6, A
    t[0xF8] = cb_set_7_b; // SET 7, B
    t[0xF9] = cb_set_7_c; // SET 7, C
    t[0xFA] = cb_set_7_d; // SET 7, D
    t[0xFB] = cb_set_7_e; // SET 7, E
    t[0xFC] = cb_set_7_h; // SET 7, H
    t[0xFD] = cb_set_7_l; // SET 7, L
    t[0xFE] = cb_set_7_hl; // SET 7, (HL)
    t[0xFF] = cb_set_7_a; // SET 7, A

    t
};

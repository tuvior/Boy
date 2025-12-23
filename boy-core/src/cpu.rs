use crate::{mmu::MMU, registers::Registers};

pub struct CPU {
    pub r: Registers,
    pub ime: bool, // IME: Interrupt master enable flag
    ime_pending: bool,
    halted: bool,
    stopped: bool,
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

        Self {
            r,
            ime: false,
            ime_pending: false,
            halted: false,
            stopped: false,
        }
    }

    pub fn step(&mut self, mmu: &mut MMU) -> Cycles {
        // No interrupt handling for now
        if self.stopped || self.halted {
            return 1;
        }

        let apply_ime = self.ime_pending;
        self.ime_pending = false;

        let op = self.rb(mmu);

        let cycles = if op == 0xCB {
            let cb = self.rb(mmu);
            (CB_TABLE[cb as usize])(self, mmu)
        } else {
            (OP_TABLE[op as usize])(self, mmu)
        };

        if apply_ime {
            self.ime = true;
        }

        cycles
    }

    #[inline]
    fn pc_inc(&mut self, val: u16) {
        self.r.pc = self.r.pc.wrapping_add(val)
    }

    #[inline]
    fn sp_inc(&mut self, val: u16) {
        self.r.sp = self.r.sp.wrapping_add(val)
    }

    #[inline]
    fn sp_sub(&mut self, val: u16) {
        self.r.sp = self.r.sp.wrapping_sub(val)
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

    #[inline]
    fn push(&mut self, mmu: &mut MMU, val: u16) {
        self.sp_sub(2);
        mmu.ww(self.r.sp, val);
    }

    #[inline]
    fn pop(&mut self, mmu: &mut MMU) -> u16 {
        let val = mmu.rw(self.r.sp);
        self.sp_inc(2);
        val
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

fn set_flags(cpu: &mut CPU, z: bool, n: bool, h: bool, c: bool) {
    cpu.r.set_z(z);
    cpu.r.set_n(n);
    cpu.r.set_h(h);
    cpu.r.set_c(c);
}

fn add8(cpu: &mut CPU, a: u8, b: u8, carry: bool) -> u8 {
    let carry_val = if carry { 1 } else { 0 };
    let sum = (a as u16) + (b as u16) + (carry_val as u16);
    let res = sum as u8;
    let h = ((a & 0x0F) + (b & 0x0F) + carry_val) > 0x0F;
    let c = sum > 0xFF;
    set_flags(cpu, res == 0, false, h, c);
    res
}

fn sub8(cpu: &mut CPU, a: u8, b: u8, carry: bool) -> u8 {
    let carry_val = if carry { 1 } else { 0 };
    let res = a.wrapping_sub(b).wrapping_sub(carry_val);
    let h = (a & 0x0F) < ((b & 0x0F) + carry_val);
    let c = (a as u16) < (b as u16 + carry_val as u16);
    set_flags(cpu, res == 0, true, h, c);
    res
}

fn inc8(cpu: &mut CPU, v: u8) -> u8 {
    let res = v.wrapping_add(1);
    cpu.r.set_z(res == 0);
    cpu.r.set_n(false);
    cpu.r.set_h((v & 0x0F) == 0x0F);
    res
}

fn dec8(cpu: &mut CPU, v: u8) -> u8 {
    let res = v.wrapping_sub(1);
    cpu.r.set_z(res == 0);
    cpu.r.set_n(true);
    cpu.r.set_h((v & 0x0F) == 0x00);
    res
}

fn add_hl(cpu: &mut CPU, v: u16) {
    let hl = cpu.r.hl();
    let res = hl.wrapping_add(v);
    cpu.r.set_n(false);
    cpu.r.set_h(((hl & 0x0FFF) + (v & 0x0FFF)) > 0x0FFF);
    cpu.r.set_c((hl as u32 + v as u32) > 0xFFFF);
    cpu.r.set_hl(res);
}

fn add_sp(cpu: &mut CPU, v: i8) -> u16 {
    let sp = cpu.r.sp;
    let v_u16 = v as u16;
    let res = sp.wrapping_add(v_u16);
    cpu.r.set_z(false);
    cpu.r.set_n(false);
    cpu.r.set_h(((sp & 0x0F) + (v_u16 & 0x0F)) > 0x0F);
    cpu.r.set_c(((sp & 0xFF) + (v_u16 & 0xFF)) > 0xFF);
    res
}

fn rlc(cpu: &mut CPU, v: u8) -> u8 {
    let c = v & 0x80 != 0;
    let res = v.rotate_left(1);
    set_flags(cpu, res == 0, false, false, c);
    res
}

fn rrc(cpu: &mut CPU, v: u8) -> u8 {
    let c = v & 0x01 != 0;
    let res = v.rotate_right(1);
    set_flags(cpu, res == 0, false, false, c);
    res
}

fn rl(cpu: &mut CPU, v: u8) -> u8 {
    let carry = cpu.r.c();
    let c = v & 0x80 != 0;
    let res = (v << 1) | if carry { 1 } else { 0 };
    set_flags(cpu, res == 0, false, false, c);
    res
}

fn rr(cpu: &mut CPU, v: u8) -> u8 {
    let carry = cpu.r.c();
    let c = v & 0x01 != 0;
    let res = (v >> 1) | if carry { 0x80 } else { 0 };
    set_flags(cpu, res == 0, false, false, c);
    res
}

fn sla(cpu: &mut CPU, v: u8) -> u8 {
    let c = v & 0x80 != 0;
    let res = v << 1;
    set_flags(cpu, res == 0, false, false, c);
    res
}

fn sra(cpu: &mut CPU, v: u8) -> u8 {
    let c = v & 0x01 != 0;
    let res = (v >> 1) | (v & 0x80);
    set_flags(cpu, res == 0, false, false, c);
    res
}

fn srl(cpu: &mut CPU, v: u8) -> u8 {
    let c = v & 0x01 != 0;
    let res = v >> 1;
    set_flags(cpu, res == 0, false, false, c);
    res
}

fn swap(cpu: &mut CPU, v: u8) -> u8 {
    let res = (v << 4) | (v >> 4);
    set_flags(cpu, res == 0, false, false, false);
    res
}

fn bit(cpu: &mut CPU, bit: u8, v: u8) {
    cpu.r.set_z(v & (1 << bit) == 0);
    cpu.r.set_n(false);
    cpu.r.set_h(true);
}

fn res(bit: u8, v: u8) -> u8 {
    v & !(1 << bit)
}

fn set(bit: u8, v: u8) -> u8 {
    v | (1 << bit)
}

fn add_a(cpu: &mut CPU, v: u8) {
    let res = add8(cpu, cpu.r.a, v, false);
    cpu.r.a = res;
}

fn adc_a(cpu: &mut CPU, v: u8) {
    let res = add8(cpu, cpu.r.a, v, cpu.r.c());
    cpu.r.a = res;
}

fn sub_a(cpu: &mut CPU, v: u8) {
    let res = sub8(cpu, cpu.r.a, v, false);
    cpu.r.a = res;
}

fn sbc_a(cpu: &mut CPU, v: u8) {
    let res = sub8(cpu, cpu.r.a, v, cpu.r.c());
    cpu.r.a = res;
}

fn and_a(cpu: &mut CPU, v: u8) {
    let res = cpu.r.a & v;
    cpu.r.a = res;
    set_flags(cpu, res == 0, false, true, false);
}

fn xor_a(cpu: &mut CPU, v: u8) {
    let res = cpu.r.a ^ v;
    cpu.r.a = res;
    set_flags(cpu, res == 0, false, false, false);
}

fn or_a(cpu: &mut CPU, v: u8) {
    let res = cpu.r.a | v;
    cpu.r.a = res;
    set_flags(cpu, res == 0, false, false, false);
}

fn cp_a(cpu: &mut CPU, v: u8) {
    let _ = sub8(cpu, cpu.r.a, v, false);
}

fn jr(cpu: &mut CPU, offset: i8) {
    cpu.r.pc = cpu.r.pc.wrapping_add(offset as i16 as u16);
}

fn call(cpu: &mut CPU, mmu: &mut MMU, addr: u16) {
    cpu.push(mmu, cpu.r.pc);
    cpu.r.pc = addr;
}

fn op_nop(_: &mut CPU, _: &mut MMU) -> Cycles {
    1
}

fn op_stop(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let _ = cpu.rb(mmu);
    cpu.stopped = true;
    1
}

fn op_halt(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.halted = true;
    1
}

fn op_prefix_cb(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let cb = cpu.rb(mmu);
    (CB_TABLE[cb as usize])(cpu, mmu)
}

// 16-bit loads
fn op_ld_bc_d16(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let v = cpu.rw(mmu);
    cpu.r.set_bc(v);
    3
}

fn op_ld_de_d16(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let v = cpu.rw(mmu);
    cpu.r.set_de(v);
    3
}

fn op_ld_hl_d16(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let v = cpu.rw(mmu);
    cpu.r.set_hl(v);
    3
}

fn op_ld_sp_d16(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let v = cpu.rw(mmu);
    cpu.r.sp = v;
    3
}

fn op_ld_a16_sp(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let addr = cpu.rw(mmu);
    mmu.ww(addr, cpu.r.sp);
    5
}

fn op_ld_sp_hl(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.sp = cpu.r.hl();
    2
}

fn op_ld_hl_sp_r8(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let v = cpu.rb(mmu) as i8;
    let res = add_sp(cpu, v);
    cpu.r.set_hl(res);
    3
}

fn op_add_sp_r8(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let v = cpu.rb(mmu) as i8;
    let res = add_sp(cpu, v);
    cpu.r.sp = res;
    4
}

// 16-bit inc/dec
fn op_inc_bc(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.set_bc(cpu.r.bc().wrapping_add(1));
    2
}

fn op_inc_de(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.set_de(cpu.r.de().wrapping_add(1));
    2
}

fn op_inc_hl(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.set_hl(cpu.r.hl().wrapping_add(1));
    2
}

fn op_inc_sp(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.sp = cpu.r.sp.wrapping_add(1);
    2
}

fn op_dec_bc(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.set_bc(cpu.r.bc().wrapping_sub(1));
    2
}

fn op_dec_de(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.set_de(cpu.r.de().wrapping_sub(1));
    2
}

fn op_dec_hl(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.set_hl(cpu.r.hl().wrapping_sub(1));
    2
}

fn op_dec_sp(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.sp = cpu.r.sp.wrapping_sub(1);
    2
}

// 16-bit add
fn op_add_hl_bc(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    add_hl(cpu, cpu.r.bc());
    2
}

fn op_add_hl_de(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    add_hl(cpu, cpu.r.de());
    2
}

fn op_add_hl_hl(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    add_hl(cpu, cpu.r.hl());
    2
}

fn op_add_hl_sp(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    add_hl(cpu, cpu.r.sp);
    2
}

// 8-bit loads (special forms)
fn op_ld_bc_a(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    mmu.wb(cpu.r.bc(), cpu.r.a);
    2
}

fn op_ld_de_a(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    mmu.wb(cpu.r.de(), cpu.r.a);
    2
}

fn op_ld_hli_a(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let addr = cpu.r.hl();
    mmu.wb(addr, cpu.r.a);
    cpu.r.set_hl(addr.wrapping_add(1));
    2
}

fn op_ld_hld_a(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let addr = cpu.r.hl();
    mmu.wb(addr, cpu.r.a);
    cpu.r.set_hl(addr.wrapping_sub(1));
    2
}

fn op_ld_a_bc(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    cpu.r.a = mmu.rb(cpu.r.bc());
    2
}

fn op_ld_a_de(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    cpu.r.a = mmu.rb(cpu.r.de());
    2
}

fn op_ld_a_hli(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let addr = cpu.r.hl();
    cpu.r.a = mmu.rb(addr);
    cpu.r.set_hl(addr.wrapping_add(1));
    2
}

fn op_ld_a_hld(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let addr = cpu.r.hl();
    cpu.r.a = mmu.rb(addr);
    cpu.r.set_hl(addr.wrapping_sub(1));
    2
}

fn op_ld_a16_a(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let addr = cpu.rw(mmu);
    mmu.wb(addr, cpu.r.a);
    4
}

fn op_ld_a_a16(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let addr = cpu.rw(mmu);
    cpu.r.a = mmu.rb(addr);
    4
}

fn op_ld_ff00_c_a(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let addr = 0xFF00u16 | cpu.r.c as u16;
    mmu.wb(addr, cpu.r.a);
    2
}

fn op_ld_a_ff00_c(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let addr = 0xFF00u16 | cpu.r.c as u16;
    cpu.r.a = mmu.rb(addr);
    2
}

fn op_ldh_a8_a(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let addr = 0xFF00u16 | cpu.rb(mmu) as u16;
    mmu.wb(addr, cpu.r.a);
    3
}

fn op_ldh_a_a8(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let addr = 0xFF00u16 | cpu.rb(mmu) as u16;
    cpu.r.a = mmu.rb(addr);
    3
}

// 8-bit inc/dec (HL)
fn op_inc_hl_ptr(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let addr = cpu.r.hl();
    let v = mmu.rb(addr);
    let res = inc8(cpu, v);
    mmu.wb(addr, res);
    3
}

fn op_dec_hl_ptr(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let addr = cpu.r.hl();
    let v = mmu.rb(addr);
    let res = dec8(cpu, v);
    mmu.wb(addr, res);
    3
}

// Jumps
fn op_jr(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let offset = cpu.rb(mmu) as i8;
    jr(cpu, offset);
    3
}

fn op_jr_nz(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let offset = cpu.rb(mmu) as i8;
    if !cpu.r.z() {
        jr(cpu, offset);
        3
    } else {
        2
    }
}

fn op_jr_z(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let offset = cpu.rb(mmu) as i8;
    if cpu.r.z() {
        jr(cpu, offset);
        3
    } else {
        2
    }
}

fn op_jr_nc(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let offset = cpu.rb(mmu) as i8;
    if !cpu.r.c() {
        jr(cpu, offset);
        3
    } else {
        2
    }
}

fn op_jr_c(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let offset = cpu.rb(mmu) as i8;
    if cpu.r.c() {
        jr(cpu, offset);
        3
    } else {
        2
    }
}

fn op_jp_a16(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let addr = cpu.rw(mmu);
    cpu.r.pc = addr;
    4
}

fn op_jp_hl(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.pc = cpu.r.hl();
    1
}

fn op_jp_nz(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let addr = cpu.rw(mmu);
    if !cpu.r.z() {
        cpu.r.pc = addr;
        4
    } else {
        3
    }
}

fn op_jp_z(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let addr = cpu.rw(mmu);
    if cpu.r.z() {
        cpu.r.pc = addr;
        4
    } else {
        3
    }
}

fn op_jp_nc(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let addr = cpu.rw(mmu);
    if !cpu.r.c() {
        cpu.r.pc = addr;
        4
    } else {
        3
    }
}

fn op_jp_c(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let addr = cpu.rw(mmu);
    if cpu.r.c() {
        cpu.r.pc = addr;
        4
    } else {
        3
    }
}

// Calls/returns
fn op_call_a16(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let addr = cpu.rw(mmu);
    call(cpu, mmu, addr);
    6
}

fn op_call_nz(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let addr = cpu.rw(mmu);
    if !cpu.r.z() {
        call(cpu, mmu, addr);
        6
    } else {
        3
    }
}

fn op_call_z(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let addr = cpu.rw(mmu);
    if cpu.r.z() {
        call(cpu, mmu, addr);
        6
    } else {
        3
    }
}

fn op_call_nc(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let addr = cpu.rw(mmu);
    if !cpu.r.c() {
        call(cpu, mmu, addr);
        6
    } else {
        3
    }
}

fn op_call_c(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let addr = cpu.rw(mmu);
    if cpu.r.c() {
        call(cpu, mmu, addr);
        6
    } else {
        3
    }
}

fn op_ret(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    cpu.r.pc = cpu.pop(mmu);
    4
}

fn op_ret_nz(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    if !cpu.r.z() {
        cpu.r.pc = cpu.pop(mmu);
        5
    } else {
        2
    }
}

fn op_ret_z(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    if cpu.r.z() {
        cpu.r.pc = cpu.pop(mmu);
        5
    } else {
        2
    }
}

fn op_ret_nc(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    if !cpu.r.c() {
        cpu.r.pc = cpu.pop(mmu);
        5
    } else {
        2
    }
}

fn op_ret_c(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    if cpu.r.c() {
        cpu.r.pc = cpu.pop(mmu);
        5
    } else {
        2
    }
}

fn op_reti(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    cpu.r.pc = cpu.pop(mmu);
    cpu.ime = true;
    4
}

// RST
fn op_rst_00(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    call(cpu, mmu, 0x0000);
    4
}

fn op_rst_08(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    call(cpu, mmu, 0x0008);
    4
}

fn op_rst_10(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    call(cpu, mmu, 0x0010);
    4
}

fn op_rst_18(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    call(cpu, mmu, 0x0018);
    4
}

fn op_rst_20(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    call(cpu, mmu, 0x0020);
    4
}

fn op_rst_28(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    call(cpu, mmu, 0x0028);
    4
}

fn op_rst_30(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    call(cpu, mmu, 0x0030);
    4
}

fn op_rst_38(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    call(cpu, mmu, 0x0038);
    4
}

// Stack
fn op_push_bc(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    cpu.push(mmu, cpu.r.bc());
    4
}

fn op_push_de(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    cpu.push(mmu, cpu.r.de());
    4
}

fn op_push_hl(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    cpu.push(mmu, cpu.r.hl());
    4
}

fn op_push_af(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    cpu.push(mmu, cpu.r.af());
    4
}

fn op_pop_bc(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let v = cpu.pop(mmu);
    cpu.r.set_bc(v);
    3
}

fn op_pop_de(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let v = cpu.pop(mmu);
    cpu.r.set_de(v);
    3
}

fn op_pop_hl(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let v = cpu.pop(mmu);
    cpu.r.set_hl(v);
    3
}

fn op_pop_af(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let v = cpu.pop(mmu);
    cpu.r.set_af(v);
    3
}

// CPU control
fn op_di(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.ime = false;
    cpu.ime_pending = false;
    1
}

fn op_ei(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.ime_pending = true;
    1
}

// Misc
fn op_rlca(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    let res = rlc(cpu, cpu.r.a);
    cpu.r.a = res;
    cpu.r.set_z(false);
    1
}

fn op_rrca(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    let res = rrc(cpu, cpu.r.a);
    cpu.r.a = res;
    cpu.r.set_z(false);
    1
}

fn op_rla(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    let res = rl(cpu, cpu.r.a);
    cpu.r.a = res;
    cpu.r.set_z(false);
    1
}

fn op_rra(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    let res = rr(cpu, cpu.r.a);
    cpu.r.a = res;
    cpu.r.set_z(false);
    1
}

fn op_daa(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    let mut a = cpu.r.a;
    let mut adjust = 0u8;
    let mut carry = cpu.r.c();

    if !cpu.r.n() {
        if cpu.r.h() || (a & 0x0F) > 0x09 {
            adjust |= 0x06;
        }
        if cpu.r.c() || a > 0x99 {
            adjust |= 0x60;
            carry = true;
        }
        a = a.wrapping_add(adjust);
    } else {
        if cpu.r.h() {
            adjust |= 0x06;
        }
        if cpu.r.c() {
            adjust |= 0x60;
        }
        a = a.wrapping_sub(adjust);
    }

    cpu.r.a = a;
    cpu.r.set_z(a == 0);
    cpu.r.set_h(false);
    cpu.r.set_c(carry);

    1
}

fn op_cpl(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.a = !cpu.r.a;
    cpu.r.set_n(true);
    cpu.r.set_h(true);
    1
}

fn op_scf(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.set_n(false);
    cpu.r.set_h(false);
    cpu.r.set_c(true);
    1
}

fn op_ccf(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    let c = cpu.r.c();
    cpu.r.set_n(false);
    cpu.r.set_h(false);
    cpu.r.set_c(!c);
    1
}
fn op_ld_b_d8(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let v = cpu.rb(mmu);
    cpu.r.b = v;
    2
}

fn op_ld_c_d8(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let v = cpu.rb(mmu);
    cpu.r.c = v;
    2
}

fn op_ld_d_d8(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let v = cpu.rb(mmu);
    cpu.r.d = v;
    2
}

fn op_ld_e_d8(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let v = cpu.rb(mmu);
    cpu.r.e = v;
    2
}

fn op_ld_h_d8(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let v = cpu.rb(mmu);
    cpu.r.h = v;
    2
}

fn op_ld_l_d8(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let v = cpu.rb(mmu);
    cpu.r.l = v;
    2
}

fn op_ld_a_d8(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let v = cpu.rb(mmu);
    cpu.r.a = v;
    2
}

fn op_ld_hl_d8(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let v = cpu.rb(mmu);
    let addr = cpu.r.hl();
    mmu.wb(addr, v);
    3
}
fn op_inc_b(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.b = inc8(cpu, cpu.r.b);
    1
}

fn op_dec_b(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.b = dec8(cpu, cpu.r.b);
    1
}

fn op_inc_c(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.c = inc8(cpu, cpu.r.c);
    1
}

fn op_dec_c(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.c = dec8(cpu, cpu.r.c);
    1
}

fn op_inc_d(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.d = inc8(cpu, cpu.r.d);
    1
}

fn op_dec_d(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.d = dec8(cpu, cpu.r.d);
    1
}

fn op_inc_e(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.e = inc8(cpu, cpu.r.e);
    1
}

fn op_dec_e(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.e = dec8(cpu, cpu.r.e);
    1
}

fn op_inc_h(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.h = inc8(cpu, cpu.r.h);
    1
}

fn op_dec_h(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.h = dec8(cpu, cpu.r.h);
    1
}

fn op_inc_l(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.l = inc8(cpu, cpu.r.l);
    1
}

fn op_dec_l(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.l = dec8(cpu, cpu.r.l);
    1
}

fn op_inc_a(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.a = inc8(cpu, cpu.r.a);
    1
}

fn op_dec_a(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.a = dec8(cpu, cpu.r.a);
    1
}

#[allow(clippy::self_assignment, dead_code)]
fn op_ld_b_b(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.b = cpu.r.b;
    1
}

fn op_ld_b_c(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.b = cpu.r.c;
    1
}

fn op_ld_b_d(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.b = cpu.r.d;
    1
}

fn op_ld_b_e(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.b = cpu.r.e;
    1
}

fn op_ld_b_h(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.b = cpu.r.h;
    1
}

fn op_ld_b_l(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.b = cpu.r.l;
    1
}

fn op_ld_b_hl(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    cpu.r.b = mmu.rb(cpu.r.hl());
    2
}

fn op_ld_b_a(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.b = cpu.r.a;
    1
}

fn op_ld_c_b(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.c = cpu.r.b;
    1
}

#[allow(clippy::self_assignment, dead_code)]
fn op_ld_c_c(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.c = cpu.r.c;
    1
}

fn op_ld_c_d(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.c = cpu.r.d;
    1
}

fn op_ld_c_e(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.c = cpu.r.e;
    1
}

fn op_ld_c_h(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.c = cpu.r.h;
    1
}

fn op_ld_c_l(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.c = cpu.r.l;
    1
}

fn op_ld_c_hl(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    cpu.r.c = mmu.rb(cpu.r.hl());
    2
}

fn op_ld_c_a(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.c = cpu.r.a;
    1
}

fn op_ld_d_b(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.d = cpu.r.b;
    1
}

fn op_ld_d_c(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.d = cpu.r.c;
    1
}

#[allow(clippy::self_assignment, dead_code)]
fn op_ld_d_d(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.d = cpu.r.d;
    1
}

fn op_ld_d_e(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.d = cpu.r.e;
    1
}

fn op_ld_d_h(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.d = cpu.r.h;
    1
}

fn op_ld_d_l(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.d = cpu.r.l;
    1
}

fn op_ld_d_hl(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    cpu.r.d = mmu.rb(cpu.r.hl());
    2
}

fn op_ld_d_a(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.d = cpu.r.a;
    1
}

fn op_ld_e_b(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.e = cpu.r.b;
    1
}

fn op_ld_e_c(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.e = cpu.r.c;
    1
}

fn op_ld_e_d(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.e = cpu.r.d;
    1
}

#[allow(clippy::self_assignment, dead_code)]
fn op_ld_e_e(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.e = cpu.r.e;
    1
}

fn op_ld_e_h(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.e = cpu.r.h;
    1
}

fn op_ld_e_l(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.e = cpu.r.l;
    1
}

fn op_ld_e_hl(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    cpu.r.e = mmu.rb(cpu.r.hl());
    2
}

fn op_ld_e_a(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.e = cpu.r.a;
    1
}

fn op_ld_h_b(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.h = cpu.r.b;
    1
}

fn op_ld_h_c(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.h = cpu.r.c;
    1
}

fn op_ld_h_d(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.h = cpu.r.d;
    1
}

fn op_ld_h_e(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.h = cpu.r.e;
    1
}

#[allow(clippy::self_assignment, dead_code)]
fn op_ld_h_h(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.h = cpu.r.h;
    1
}

fn op_ld_h_l(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.h = cpu.r.l;
    1
}

fn op_ld_h_hl(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    cpu.r.h = mmu.rb(cpu.r.hl());
    2
}

fn op_ld_h_a(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.h = cpu.r.a;
    1
}

fn op_ld_l_b(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.l = cpu.r.b;
    1
}

fn op_ld_l_c(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.l = cpu.r.c;
    1
}

fn op_ld_l_d(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.l = cpu.r.d;
    1
}

fn op_ld_l_e(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.l = cpu.r.e;
    1
}

fn op_ld_l_h(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.l = cpu.r.h;
    1
}

#[allow(clippy::self_assignment, dead_code)]
fn op_ld_l_l(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.l = cpu.r.l;
    1
}

fn op_ld_l_hl(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    cpu.r.l = mmu.rb(cpu.r.hl());
    2
}

fn op_ld_l_a(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.l = cpu.r.a;
    1
}

fn op_ld_hl_b(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let addr = cpu.r.hl();
    mmu.wb(addr, cpu.r.b);
    2
}

fn op_ld_hl_c(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let addr = cpu.r.hl();
    mmu.wb(addr, cpu.r.c);
    2
}

fn op_ld_hl_d(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let addr = cpu.r.hl();
    mmu.wb(addr, cpu.r.d);
    2
}

fn op_ld_hl_e(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let addr = cpu.r.hl();
    mmu.wb(addr, cpu.r.e);
    2
}

fn op_ld_hl_h(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let addr = cpu.r.hl();
    mmu.wb(addr, cpu.r.h);
    2
}

fn op_ld_hl_l(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let addr = cpu.r.hl();
    mmu.wb(addr, cpu.r.l);
    2
}

fn op_ld_hl_a(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let addr = cpu.r.hl();
    mmu.wb(addr, cpu.r.a);
    2
}

fn op_ld_a_b(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.a = cpu.r.b;
    1
}

fn op_ld_a_c(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.a = cpu.r.c;
    1
}

fn op_ld_a_d(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.a = cpu.r.d;
    1
}

fn op_ld_a_e(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.a = cpu.r.e;
    1
}

fn op_ld_a_h(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.a = cpu.r.h;
    1
}

fn op_ld_a_l(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.a = cpu.r.l;
    1
}

fn op_ld_a_hl(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    cpu.r.a = mmu.rb(cpu.r.hl());
    2
}

#[allow(clippy::self_assignment, dead_code)]
fn op_ld_a_a(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.a = cpu.r.a;
    1
}

fn op_add_a_b(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    add_a(cpu, cpu.r.b);
    1
}

fn op_add_a_c(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    add_a(cpu, cpu.r.c);
    1
}

fn op_add_a_d(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    add_a(cpu, cpu.r.d);
    1
}

fn op_add_a_e(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    add_a(cpu, cpu.r.e);
    1
}

fn op_add_a_h(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    add_a(cpu, cpu.r.h);
    1
}

fn op_add_a_l(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    add_a(cpu, cpu.r.l);
    1
}

fn op_add_a_hl(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let v = mmu.rb(cpu.r.hl());
    add_a(cpu, v);
    2
}

fn op_add_a_a(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    add_a(cpu, cpu.r.a);
    1
}

fn op_adc_a_b(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    adc_a(cpu, cpu.r.b);
    1
}

fn op_adc_a_c(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    adc_a(cpu, cpu.r.c);
    1
}

fn op_adc_a_d(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    adc_a(cpu, cpu.r.d);
    1
}

fn op_adc_a_e(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    adc_a(cpu, cpu.r.e);
    1
}

fn op_adc_a_h(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    adc_a(cpu, cpu.r.h);
    1
}

fn op_adc_a_l(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    adc_a(cpu, cpu.r.l);
    1
}

fn op_adc_a_hl(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let v = mmu.rb(cpu.r.hl());
    adc_a(cpu, v);
    2
}

fn op_adc_a_a(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    adc_a(cpu, cpu.r.a);
    1
}

fn op_sub_b(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    sub_a(cpu, cpu.r.b);
    1
}

fn op_sub_c(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    sub_a(cpu, cpu.r.c);
    1
}

fn op_sub_d(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    sub_a(cpu, cpu.r.d);
    1
}

fn op_sub_e(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    sub_a(cpu, cpu.r.e);
    1
}

fn op_sub_h(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    sub_a(cpu, cpu.r.h);
    1
}

fn op_sub_l(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    sub_a(cpu, cpu.r.l);
    1
}

fn op_sub_hl(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let v = mmu.rb(cpu.r.hl());
    sub_a(cpu, v);
    2
}

fn op_sub_a(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    sub_a(cpu, cpu.r.a);
    1
}

fn op_sbc_a_b(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    sbc_a(cpu, cpu.r.b);
    1
}

fn op_sbc_a_c(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    sbc_a(cpu, cpu.r.c);
    1
}

fn op_sbc_a_d(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    sbc_a(cpu, cpu.r.d);
    1
}

fn op_sbc_a_e(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    sbc_a(cpu, cpu.r.e);
    1
}

fn op_sbc_a_h(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    sbc_a(cpu, cpu.r.h);
    1
}

fn op_sbc_a_l(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    sbc_a(cpu, cpu.r.l);
    1
}

fn op_sbc_a_hl(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let v = mmu.rb(cpu.r.hl());
    sbc_a(cpu, v);
    2
}

fn op_sbc_a_a(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    sbc_a(cpu, cpu.r.a);
    1
}

fn op_and_b(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    and_a(cpu, cpu.r.b);
    1
}

fn op_and_c(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    and_a(cpu, cpu.r.c);
    1
}

fn op_and_d(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    and_a(cpu, cpu.r.d);
    1
}

fn op_and_e(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    and_a(cpu, cpu.r.e);
    1
}

fn op_and_h(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    and_a(cpu, cpu.r.h);
    1
}

fn op_and_l(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    and_a(cpu, cpu.r.l);
    1
}

fn op_and_hl(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let v = mmu.rb(cpu.r.hl());
    and_a(cpu, v);
    2
}

fn op_and_a(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    and_a(cpu, cpu.r.a);
    1
}

fn op_xor_b(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    xor_a(cpu, cpu.r.b);
    1
}

fn op_xor_c(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    xor_a(cpu, cpu.r.c);
    1
}

fn op_xor_d(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    xor_a(cpu, cpu.r.d);
    1
}

fn op_xor_e(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    xor_a(cpu, cpu.r.e);
    1
}

fn op_xor_h(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    xor_a(cpu, cpu.r.h);
    1
}

fn op_xor_l(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    xor_a(cpu, cpu.r.l);
    1
}

fn op_xor_hl(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let v = mmu.rb(cpu.r.hl());
    xor_a(cpu, v);
    2
}

fn op_xor_a(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    xor_a(cpu, cpu.r.a);
    1
}

fn op_or_b(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    or_a(cpu, cpu.r.b);
    1
}

fn op_or_c(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    or_a(cpu, cpu.r.c);
    1
}

fn op_or_d(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    or_a(cpu, cpu.r.d);
    1
}

fn op_or_e(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    or_a(cpu, cpu.r.e);
    1
}

fn op_or_h(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    or_a(cpu, cpu.r.h);
    1
}

fn op_or_l(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    or_a(cpu, cpu.r.l);
    1
}

fn op_or_hl(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let v = mmu.rb(cpu.r.hl());
    or_a(cpu, v);
    2
}

fn op_or_a(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    or_a(cpu, cpu.r.a);
    1
}

fn op_cp_b(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cp_a(cpu, cpu.r.b);
    1
}

fn op_cp_c(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cp_a(cpu, cpu.r.c);
    1
}

fn op_cp_d(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cp_a(cpu, cpu.r.d);
    1
}

fn op_cp_e(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cp_a(cpu, cpu.r.e);
    1
}

fn op_cp_h(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cp_a(cpu, cpu.r.h);
    1
}

fn op_cp_l(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cp_a(cpu, cpu.r.l);
    1
}

fn op_cp_hl(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let v = mmu.rb(cpu.r.hl());
    cp_a(cpu, v);
    2
}

fn op_cp_a(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cp_a(cpu, cpu.r.a);
    1
}

fn op_add_a_d8(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let v = cpu.rb(mmu);
    add_a(cpu, v);
    2
}

fn op_adc_a_d8(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let v = cpu.rb(mmu);
    adc_a(cpu, v);
    2
}

fn op_sub_d8(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let v = cpu.rb(mmu);
    sub_a(cpu, v);
    2
}

fn op_sbc_a_d8(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let v = cpu.rb(mmu);
    sbc_a(cpu, v);
    2
}

fn op_and_d8(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let v = cpu.rb(mmu);
    and_a(cpu, v);
    2
}

fn op_xor_d8(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let v = cpu.rb(mmu);
    xor_a(cpu, v);
    2
}

fn op_or_d8(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let v = cpu.rb(mmu);
    or_a(cpu, v);
    2
}

fn op_cp_d8(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let v = cpu.rb(mmu);
    cp_a(cpu, v);
    2
}

fn cb_rlc_b(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.b = rlc(cpu, cpu.r.b);
    2
}

fn cb_rlc_c(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.c = rlc(cpu, cpu.r.c);
    2
}

fn cb_rlc_d(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.d = rlc(cpu, cpu.r.d);
    2
}

fn cb_rlc_e(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.e = rlc(cpu, cpu.r.e);
    2
}

fn cb_rlc_h(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.h = rlc(cpu, cpu.r.h);
    2
}

fn cb_rlc_l(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.l = rlc(cpu, cpu.r.l);
    2
}

fn cb_rlc_hl(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let addr = cpu.r.hl();
    let v = mmu.rb(addr);
    let res = rlc(cpu, v);
    mmu.wb(addr, res);
    4
}

fn cb_rlc_a(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.a = rlc(cpu, cpu.r.a);
    2
}

fn cb_rrc_b(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.b = rrc(cpu, cpu.r.b);
    2
}

fn cb_rrc_c(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.c = rrc(cpu, cpu.r.c);
    2
}

fn cb_rrc_d(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.d = rrc(cpu, cpu.r.d);
    2
}

fn cb_rrc_e(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.e = rrc(cpu, cpu.r.e);
    2
}

fn cb_rrc_h(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.h = rrc(cpu, cpu.r.h);
    2
}

fn cb_rrc_l(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.l = rrc(cpu, cpu.r.l);
    2
}

fn cb_rrc_hl(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let addr = cpu.r.hl();
    let v = mmu.rb(addr);
    let res = rrc(cpu, v);
    mmu.wb(addr, res);
    4
}

fn cb_rrc_a(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.a = rrc(cpu, cpu.r.a);
    2
}

fn cb_rl_b(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.b = rl(cpu, cpu.r.b);
    2
}

fn cb_rl_c(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.c = rl(cpu, cpu.r.c);
    2
}

fn cb_rl_d(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.d = rl(cpu, cpu.r.d);
    2
}

fn cb_rl_e(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.e = rl(cpu, cpu.r.e);
    2
}

fn cb_rl_h(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.h = rl(cpu, cpu.r.h);
    2
}

fn cb_rl_l(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.l = rl(cpu, cpu.r.l);
    2
}

fn cb_rl_hl(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let addr = cpu.r.hl();
    let v = mmu.rb(addr);
    let res = rl(cpu, v);
    mmu.wb(addr, res);
    4
}

fn cb_rl_a(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.a = rl(cpu, cpu.r.a);
    2
}

fn cb_rr_b(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.b = rr(cpu, cpu.r.b);
    2
}

fn cb_rr_c(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.c = rr(cpu, cpu.r.c);
    2
}

fn cb_rr_d(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.d = rr(cpu, cpu.r.d);
    2
}

fn cb_rr_e(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.e = rr(cpu, cpu.r.e);
    2
}

fn cb_rr_h(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.h = rr(cpu, cpu.r.h);
    2
}

fn cb_rr_l(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.l = rr(cpu, cpu.r.l);
    2
}

fn cb_rr_hl(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let addr = cpu.r.hl();
    let v = mmu.rb(addr);
    let res = rr(cpu, v);
    mmu.wb(addr, res);
    4
}

fn cb_rr_a(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.a = rr(cpu, cpu.r.a);
    2
}

fn cb_sla_b(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.b = sla(cpu, cpu.r.b);
    2
}

fn cb_sla_c(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.c = sla(cpu, cpu.r.c);
    2
}

fn cb_sla_d(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.d = sla(cpu, cpu.r.d);
    2
}

fn cb_sla_e(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.e = sla(cpu, cpu.r.e);
    2
}

fn cb_sla_h(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.h = sla(cpu, cpu.r.h);
    2
}

fn cb_sla_l(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.l = sla(cpu, cpu.r.l);
    2
}

fn cb_sla_hl(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let addr = cpu.r.hl();
    let v = mmu.rb(addr);
    let res = sla(cpu, v);
    mmu.wb(addr, res);
    4
}

fn cb_sla_a(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.a = sla(cpu, cpu.r.a);
    2
}

fn cb_sra_b(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.b = sra(cpu, cpu.r.b);
    2
}

fn cb_sra_c(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.c = sra(cpu, cpu.r.c);
    2
}

fn cb_sra_d(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.d = sra(cpu, cpu.r.d);
    2
}

fn cb_sra_e(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.e = sra(cpu, cpu.r.e);
    2
}

fn cb_sra_h(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.h = sra(cpu, cpu.r.h);
    2
}

fn cb_sra_l(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.l = sra(cpu, cpu.r.l);
    2
}

fn cb_sra_hl(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let addr = cpu.r.hl();
    let v = mmu.rb(addr);
    let res = sra(cpu, v);
    mmu.wb(addr, res);
    4
}

fn cb_sra_a(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.a = sra(cpu, cpu.r.a);
    2
}

fn cb_swap_b(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.b = swap(cpu, cpu.r.b);
    2
}

fn cb_swap_c(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.c = swap(cpu, cpu.r.c);
    2
}

fn cb_swap_d(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.d = swap(cpu, cpu.r.d);
    2
}

fn cb_swap_e(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.e = swap(cpu, cpu.r.e);
    2
}

fn cb_swap_h(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.h = swap(cpu, cpu.r.h);
    2
}

fn cb_swap_l(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.l = swap(cpu, cpu.r.l);
    2
}

fn cb_swap_hl(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let addr = cpu.r.hl();
    let v = mmu.rb(addr);
    let res = swap(cpu, v);
    mmu.wb(addr, res);
    4
}

fn cb_swap_a(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.a = swap(cpu, cpu.r.a);
    2
}

fn cb_srl_b(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.b = srl(cpu, cpu.r.b);
    2
}

fn cb_srl_c(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.c = srl(cpu, cpu.r.c);
    2
}

fn cb_srl_d(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.d = srl(cpu, cpu.r.d);
    2
}

fn cb_srl_e(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.e = srl(cpu, cpu.r.e);
    2
}

fn cb_srl_h(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.h = srl(cpu, cpu.r.h);
    2
}

fn cb_srl_l(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.l = srl(cpu, cpu.r.l);
    2
}

fn cb_srl_hl(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let addr = cpu.r.hl();
    let v = mmu.rb(addr);
    let res = srl(cpu, v);
    mmu.wb(addr, res);
    4
}

fn cb_srl_a(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.a = srl(cpu, cpu.r.a);
    2
}

fn cb_bit_0_b(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    bit(cpu, 0, cpu.r.b);
    2
}

fn cb_bit_0_c(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    bit(cpu, 0, cpu.r.c);
    2
}

fn cb_bit_0_d(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    bit(cpu, 0, cpu.r.d);
    2
}

fn cb_bit_0_e(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    bit(cpu, 0, cpu.r.e);
    2
}

fn cb_bit_0_h(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    bit(cpu, 0, cpu.r.h);
    2
}

fn cb_bit_0_l(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    bit(cpu, 0, cpu.r.l);
    2
}

fn cb_bit_0_hl(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let v = mmu.rb(cpu.r.hl());
    bit(cpu, 0, v);
    3
}

fn cb_bit_0_a(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    bit(cpu, 0, cpu.r.a);
    2
}

fn cb_bit_1_b(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    bit(cpu, 1, cpu.r.b);
    2
}

fn cb_bit_1_c(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    bit(cpu, 1, cpu.r.c);
    2
}

fn cb_bit_1_d(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    bit(cpu, 1, cpu.r.d);
    2
}

fn cb_bit_1_e(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    bit(cpu, 1, cpu.r.e);
    2
}

fn cb_bit_1_h(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    bit(cpu, 1, cpu.r.h);
    2
}

fn cb_bit_1_l(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    bit(cpu, 1, cpu.r.l);
    2
}

fn cb_bit_1_hl(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let v = mmu.rb(cpu.r.hl());
    bit(cpu, 1, v);
    3
}

fn cb_bit_1_a(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    bit(cpu, 1, cpu.r.a);
    2
}

fn cb_bit_2_b(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    bit(cpu, 2, cpu.r.b);
    2
}

fn cb_bit_2_c(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    bit(cpu, 2, cpu.r.c);
    2
}

fn cb_bit_2_d(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    bit(cpu, 2, cpu.r.d);
    2
}

fn cb_bit_2_e(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    bit(cpu, 2, cpu.r.e);
    2
}

fn cb_bit_2_h(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    bit(cpu, 2, cpu.r.h);
    2
}

fn cb_bit_2_l(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    bit(cpu, 2, cpu.r.l);
    2
}

fn cb_bit_2_hl(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let v = mmu.rb(cpu.r.hl());
    bit(cpu, 2, v);
    3
}

fn cb_bit_2_a(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    bit(cpu, 2, cpu.r.a);
    2
}

fn cb_bit_3_b(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    bit(cpu, 3, cpu.r.b);
    2
}

fn cb_bit_3_c(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    bit(cpu, 3, cpu.r.c);
    2
}

fn cb_bit_3_d(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    bit(cpu, 3, cpu.r.d);
    2
}

fn cb_bit_3_e(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    bit(cpu, 3, cpu.r.e);
    2
}

fn cb_bit_3_h(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    bit(cpu, 3, cpu.r.h);
    2
}

fn cb_bit_3_l(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    bit(cpu, 3, cpu.r.l);
    2
}

fn cb_bit_3_hl(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let v = mmu.rb(cpu.r.hl());
    bit(cpu, 3, v);
    3
}

fn cb_bit_3_a(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    bit(cpu, 3, cpu.r.a);
    2
}

fn cb_bit_4_b(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    bit(cpu, 4, cpu.r.b);
    2
}

fn cb_bit_4_c(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    bit(cpu, 4, cpu.r.c);
    2
}

fn cb_bit_4_d(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    bit(cpu, 4, cpu.r.d);
    2
}

fn cb_bit_4_e(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    bit(cpu, 4, cpu.r.e);
    2
}

fn cb_bit_4_h(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    bit(cpu, 4, cpu.r.h);
    2
}

fn cb_bit_4_l(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    bit(cpu, 4, cpu.r.l);
    2
}

fn cb_bit_4_hl(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let v = mmu.rb(cpu.r.hl());
    bit(cpu, 4, v);
    3
}

fn cb_bit_4_a(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    bit(cpu, 4, cpu.r.a);
    2
}

fn cb_bit_5_b(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    bit(cpu, 5, cpu.r.b);
    2
}

fn cb_bit_5_c(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    bit(cpu, 5, cpu.r.c);
    2
}

fn cb_bit_5_d(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    bit(cpu, 5, cpu.r.d);
    2
}

fn cb_bit_5_e(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    bit(cpu, 5, cpu.r.e);
    2
}

fn cb_bit_5_h(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    bit(cpu, 5, cpu.r.h);
    2
}

fn cb_bit_5_l(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    bit(cpu, 5, cpu.r.l);
    2
}

fn cb_bit_5_hl(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let v = mmu.rb(cpu.r.hl());
    bit(cpu, 5, v);
    3
}

fn cb_bit_5_a(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    bit(cpu, 5, cpu.r.a);
    2
}

fn cb_bit_6_b(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    bit(cpu, 6, cpu.r.b);
    2
}

fn cb_bit_6_c(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    bit(cpu, 6, cpu.r.c);
    2
}

fn cb_bit_6_d(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    bit(cpu, 6, cpu.r.d);
    2
}

fn cb_bit_6_e(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    bit(cpu, 6, cpu.r.e);
    2
}

fn cb_bit_6_h(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    bit(cpu, 6, cpu.r.h);
    2
}

fn cb_bit_6_l(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    bit(cpu, 6, cpu.r.l);
    2
}

fn cb_bit_6_hl(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let v = mmu.rb(cpu.r.hl());
    bit(cpu, 6, v);
    3
}

fn cb_bit_6_a(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    bit(cpu, 6, cpu.r.a);
    2
}

fn cb_bit_7_b(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    bit(cpu, 7, cpu.r.b);
    2
}

fn cb_bit_7_c(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    bit(cpu, 7, cpu.r.c);
    2
}

fn cb_bit_7_d(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    bit(cpu, 7, cpu.r.d);
    2
}

fn cb_bit_7_e(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    bit(cpu, 7, cpu.r.e);
    2
}

fn cb_bit_7_h(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    bit(cpu, 7, cpu.r.h);
    2
}

fn cb_bit_7_l(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    bit(cpu, 7, cpu.r.l);
    2
}

fn cb_bit_7_hl(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let v = mmu.rb(cpu.r.hl());
    bit(cpu, 7, v);
    3
}

fn cb_bit_7_a(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    bit(cpu, 7, cpu.r.a);
    2
}

fn cb_res_0_b(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.b = res(0, cpu.r.b);
    2
}

fn cb_res_0_c(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.c = res(0, cpu.r.c);
    2
}

fn cb_res_0_d(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.d = res(0, cpu.r.d);
    2
}

fn cb_res_0_e(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.e = res(0, cpu.r.e);
    2
}

fn cb_res_0_h(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.h = res(0, cpu.r.h);
    2
}

fn cb_res_0_l(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.l = res(0, cpu.r.l);
    2
}

fn cb_res_0_hl(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let addr = cpu.r.hl();
    let v = mmu.rb(addr);
    let res = res(0, v);
    mmu.wb(addr, res);
    4
}

fn cb_res_0_a(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.a = res(0, cpu.r.a);
    2
}

fn cb_res_1_b(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.b = res(1, cpu.r.b);
    2
}

fn cb_res_1_c(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.c = res(1, cpu.r.c);
    2
}

fn cb_res_1_d(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.d = res(1, cpu.r.d);
    2
}

fn cb_res_1_e(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.e = res(1, cpu.r.e);
    2
}

fn cb_res_1_h(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.h = res(1, cpu.r.h);
    2
}

fn cb_res_1_l(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.l = res(1, cpu.r.l);
    2
}

fn cb_res_1_hl(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let addr = cpu.r.hl();
    let v = mmu.rb(addr);
    let res = res(1, v);
    mmu.wb(addr, res);
    4
}

fn cb_res_1_a(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.a = res(1, cpu.r.a);
    2
}

fn cb_res_2_b(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.b = res(2, cpu.r.b);
    2
}

fn cb_res_2_c(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.c = res(2, cpu.r.c);
    2
}

fn cb_res_2_d(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.d = res(2, cpu.r.d);
    2
}

fn cb_res_2_e(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.e = res(2, cpu.r.e);
    2
}

fn cb_res_2_h(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.h = res(2, cpu.r.h);
    2
}

fn cb_res_2_l(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.l = res(2, cpu.r.l);
    2
}

fn cb_res_2_hl(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let addr = cpu.r.hl();
    let v = mmu.rb(addr);
    let res = res(2, v);
    mmu.wb(addr, res);
    4
}

fn cb_res_2_a(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.a = res(2, cpu.r.a);
    2
}

fn cb_res_3_b(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.b = res(3, cpu.r.b);
    2
}

fn cb_res_3_c(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.c = res(3, cpu.r.c);
    2
}

fn cb_res_3_d(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.d = res(3, cpu.r.d);
    2
}

fn cb_res_3_e(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.e = res(3, cpu.r.e);
    2
}

fn cb_res_3_h(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.h = res(3, cpu.r.h);
    2
}

fn cb_res_3_l(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.l = res(3, cpu.r.l);
    2
}

fn cb_res_3_hl(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let addr = cpu.r.hl();
    let v = mmu.rb(addr);
    let res = res(3, v);
    mmu.wb(addr, res);
    4
}

fn cb_res_3_a(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.a = res(3, cpu.r.a);
    2
}

fn cb_res_4_b(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.b = res(4, cpu.r.b);
    2
}

fn cb_res_4_c(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.c = res(4, cpu.r.c);
    2
}

fn cb_res_4_d(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.d = res(4, cpu.r.d);
    2
}

fn cb_res_4_e(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.e = res(4, cpu.r.e);
    2
}

fn cb_res_4_h(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.h = res(4, cpu.r.h);
    2
}

fn cb_res_4_l(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.l = res(4, cpu.r.l);
    2
}

fn cb_res_4_hl(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let addr = cpu.r.hl();
    let v = mmu.rb(addr);
    let res = res(4, v);
    mmu.wb(addr, res);
    4
}

fn cb_res_4_a(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.a = res(4, cpu.r.a);
    2
}

fn cb_res_5_b(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.b = res(5, cpu.r.b);
    2
}

fn cb_res_5_c(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.c = res(5, cpu.r.c);
    2
}

fn cb_res_5_d(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.d = res(5, cpu.r.d);
    2
}

fn cb_res_5_e(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.e = res(5, cpu.r.e);
    2
}

fn cb_res_5_h(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.h = res(5, cpu.r.h);
    2
}

fn cb_res_5_l(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.l = res(5, cpu.r.l);
    2
}

fn cb_res_5_hl(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let addr = cpu.r.hl();
    let v = mmu.rb(addr);
    let res = res(5, v);
    mmu.wb(addr, res);
    4
}

fn cb_res_5_a(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.a = res(5, cpu.r.a);
    2
}

fn cb_res_6_b(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.b = res(6, cpu.r.b);
    2
}

fn cb_res_6_c(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.c = res(6, cpu.r.c);
    2
}

fn cb_res_6_d(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.d = res(6, cpu.r.d);
    2
}

fn cb_res_6_e(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.e = res(6, cpu.r.e);
    2
}

fn cb_res_6_h(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.h = res(6, cpu.r.h);
    2
}

fn cb_res_6_l(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.l = res(6, cpu.r.l);
    2
}

fn cb_res_6_hl(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let addr = cpu.r.hl();
    let v = mmu.rb(addr);
    let res = res(6, v);
    mmu.wb(addr, res);
    4
}

fn cb_res_6_a(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.a = res(6, cpu.r.a);
    2
}

fn cb_res_7_b(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.b = res(7, cpu.r.b);
    2
}

fn cb_res_7_c(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.c = res(7, cpu.r.c);
    2
}

fn cb_res_7_d(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.d = res(7, cpu.r.d);
    2
}

fn cb_res_7_e(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.e = res(7, cpu.r.e);
    2
}

fn cb_res_7_h(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.h = res(7, cpu.r.h);
    2
}

fn cb_res_7_l(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.l = res(7, cpu.r.l);
    2
}

fn cb_res_7_hl(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let addr = cpu.r.hl();
    let v = mmu.rb(addr);
    let res = res(7, v);
    mmu.wb(addr, res);
    4
}

fn cb_res_7_a(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.a = res(7, cpu.r.a);
    2
}

fn cb_set_0_b(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.b = set(0, cpu.r.b);
    2
}

fn cb_set_0_c(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.c = set(0, cpu.r.c);
    2
}

fn cb_set_0_d(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.d = set(0, cpu.r.d);
    2
}

fn cb_set_0_e(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.e = set(0, cpu.r.e);
    2
}

fn cb_set_0_h(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.h = set(0, cpu.r.h);
    2
}

fn cb_set_0_l(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.l = set(0, cpu.r.l);
    2
}

fn cb_set_0_hl(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let addr = cpu.r.hl();
    let v = mmu.rb(addr);
    let res = set(0, v);
    mmu.wb(addr, res);
    4
}

fn cb_set_0_a(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.a = set(0, cpu.r.a);
    2
}

fn cb_set_1_b(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.b = set(1, cpu.r.b);
    2
}

fn cb_set_1_c(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.c = set(1, cpu.r.c);
    2
}

fn cb_set_1_d(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.d = set(1, cpu.r.d);
    2
}

fn cb_set_1_e(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.e = set(1, cpu.r.e);
    2
}

fn cb_set_1_h(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.h = set(1, cpu.r.h);
    2
}

fn cb_set_1_l(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.l = set(1, cpu.r.l);
    2
}

fn cb_set_1_hl(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let addr = cpu.r.hl();
    let v = mmu.rb(addr);
    let res = set(1, v);
    mmu.wb(addr, res);
    4
}

fn cb_set_1_a(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.a = set(1, cpu.r.a);
    2
}

fn cb_set_2_b(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.b = set(2, cpu.r.b);
    2
}

fn cb_set_2_c(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.c = set(2, cpu.r.c);
    2
}

fn cb_set_2_d(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.d = set(2, cpu.r.d);
    2
}

fn cb_set_2_e(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.e = set(2, cpu.r.e);
    2
}

fn cb_set_2_h(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.h = set(2, cpu.r.h);
    2
}

fn cb_set_2_l(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.l = set(2, cpu.r.l);
    2
}

fn cb_set_2_hl(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let addr = cpu.r.hl();
    let v = mmu.rb(addr);
    let res = set(2, v);
    mmu.wb(addr, res);
    4
}

fn cb_set_2_a(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.a = set(2, cpu.r.a);
    2
}

fn cb_set_3_b(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.b = set(3, cpu.r.b);
    2
}

fn cb_set_3_c(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.c = set(3, cpu.r.c);
    2
}

fn cb_set_3_d(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.d = set(3, cpu.r.d);
    2
}

fn cb_set_3_e(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.e = set(3, cpu.r.e);
    2
}

fn cb_set_3_h(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.h = set(3, cpu.r.h);
    2
}

fn cb_set_3_l(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.l = set(3, cpu.r.l);
    2
}

fn cb_set_3_hl(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let addr = cpu.r.hl();
    let v = mmu.rb(addr);
    let res = set(3, v);
    mmu.wb(addr, res);
    4
}

fn cb_set_3_a(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.a = set(3, cpu.r.a);
    2
}

fn cb_set_4_b(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.b = set(4, cpu.r.b);
    2
}

fn cb_set_4_c(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.c = set(4, cpu.r.c);
    2
}

fn cb_set_4_d(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.d = set(4, cpu.r.d);
    2
}

fn cb_set_4_e(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.e = set(4, cpu.r.e);
    2
}

fn cb_set_4_h(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.h = set(4, cpu.r.h);
    2
}

fn cb_set_4_l(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.l = set(4, cpu.r.l);
    2
}

fn cb_set_4_hl(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let addr = cpu.r.hl();
    let v = mmu.rb(addr);
    let res = set(4, v);
    mmu.wb(addr, res);
    4
}

fn cb_set_4_a(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.a = set(4, cpu.r.a);
    2
}

fn cb_set_5_b(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.b = set(5, cpu.r.b);
    2
}

fn cb_set_5_c(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.c = set(5, cpu.r.c);
    2
}

fn cb_set_5_d(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.d = set(5, cpu.r.d);
    2
}

fn cb_set_5_e(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.e = set(5, cpu.r.e);
    2
}

fn cb_set_5_h(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.h = set(5, cpu.r.h);
    2
}

fn cb_set_5_l(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.l = set(5, cpu.r.l);
    2
}

fn cb_set_5_hl(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let addr = cpu.r.hl();
    let v = mmu.rb(addr);
    let res = set(5, v);
    mmu.wb(addr, res);
    4
}

fn cb_set_5_a(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.a = set(5, cpu.r.a);
    2
}

fn cb_set_6_b(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.b = set(6, cpu.r.b);
    2
}

fn cb_set_6_c(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.c = set(6, cpu.r.c);
    2
}

fn cb_set_6_d(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.d = set(6, cpu.r.d);
    2
}

fn cb_set_6_e(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.e = set(6, cpu.r.e);
    2
}

fn cb_set_6_h(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.h = set(6, cpu.r.h);
    2
}

fn cb_set_6_l(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.l = set(6, cpu.r.l);
    2
}

fn cb_set_6_hl(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let addr = cpu.r.hl();
    let v = mmu.rb(addr);
    let res = set(6, v);
    mmu.wb(addr, res);
    4
}

fn cb_set_6_a(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.a = set(6, cpu.r.a);
    2
}

fn cb_set_7_b(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.b = set(7, cpu.r.b);
    2
}

fn cb_set_7_c(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.c = set(7, cpu.r.c);
    2
}

fn cb_set_7_d(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.d = set(7, cpu.r.d);
    2
}

fn cb_set_7_e(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.e = set(7, cpu.r.e);
    2
}

fn cb_set_7_h(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.h = set(7, cpu.r.h);
    2
}

fn cb_set_7_l(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.l = set(7, cpu.r.l);
    2
}

fn cb_set_7_hl(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let addr = cpu.r.hl();
    let v = mmu.rb(addr);
    let res = set(7, v);
    mmu.wb(addr, res);
    4
}

fn cb_set_7_a(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.a = set(7, cpu.r.a);
    2
}

pub const OP_TABLE: [OP; 256] = {
    let mut t: [OP; 256] = [op_unimp; 256];

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
    t[0xCB] = op_prefix_cb; // PREFIX CB
    t[0xCC] = op_call_z; // CALL Z, a16
    t[0xCD] = op_call_a16; // CALL a16
    t[0xCE] = op_adc_a_d8; // ADC A, d8
    t[0xCF] = op_rst_08; // RST 08H

    // 0xD0-0xDF
    t[0xD0] = op_ret_nc; // RET NC
    t[0xD1] = op_pop_de; // POP DE
    t[0xD2] = op_jp_nc; // JP NC, a16
    t[0xD3] = op_unimp; // UNUSED
    t[0xD4] = op_call_nc; // CALL NC, a16
    t[0xD5] = op_push_de; // PUSH DE
    t[0xD6] = op_sub_d8; // SUB d8
    t[0xD7] = op_rst_10; // RST 10H
    t[0xD8] = op_ret_c; // RET C
    t[0xD9] = op_reti; // RETI
    t[0xDA] = op_jp_c; // JP C, a16
    t[0xDB] = op_unimp; // UNUSED
    t[0xDC] = op_call_c; // CALL C, a16
    t[0xDD] = op_unimp; // UNUSED
    t[0xDE] = op_sbc_a_d8; // SBC A, d8
    t[0xDF] = op_rst_18; // RST 18H

    // 0xE0-0xEF
    t[0xE0] = op_ldh_a8_a; // LDH (a8), A
    t[0xE1] = op_pop_hl; // POP HL
    t[0xE2] = op_ld_ff00_c_a; // LD (C), A
    t[0xE3] = op_unimp; // UNUSED
    t[0xE4] = op_unimp; // UNUSED
    t[0xE5] = op_push_hl; // PUSH HL
    t[0xE6] = op_and_d8; // AND d8
    t[0xE7] = op_rst_20; // RST 20H
    t[0xE8] = op_add_sp_r8; // ADD SP, r8
    t[0xE9] = op_jp_hl; // JP (HL)
    t[0xEA] = op_ld_a16_a; // LD (a16), A
    t[0xEB] = op_unimp; // UNUSED
    t[0xEC] = op_unimp; // UNUSED
    t[0xED] = op_unimp; // UNUSED
    t[0xEE] = op_xor_d8; // XOR d8
    t[0xEF] = op_rst_28; // RST 28H

    // 0xF0-0xFF
    t[0xF0] = op_ldh_a_a8; // LDH A, (a8)
    t[0xF1] = op_pop_af; // POP AF
    t[0xF2] = op_ld_a_ff00_c; // LD A, (C)
    t[0xF3] = op_di; // DI
    t[0xF4] = op_unimp; // UNUSED
    t[0xF5] = op_push_af; // PUSH AF
    t[0xF6] = op_or_d8; // OR d8
    t[0xF7] = op_rst_30; // RST 30H
    t[0xF8] = op_ld_hl_sp_r8; // LD HL, SP+r8
    t[0xF9] = op_ld_sp_hl; // LD SP, HL
    t[0xFA] = op_ld_a_a16; // LD A, (a16)
    t[0xFB] = op_ei; // EI
    t[0xFC] = op_unimp; // UNUSED
    t[0xFD] = op_unimp; // UNUSED
    t[0xFE] = op_cp_d8; // CP d8
    t[0xFF] = op_rst_38; // RST 38H

    t
};

pub const CB_TABLE: [OP; 256] = {
    let mut t: [OP; 256] = [cb_unimp; 256];

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

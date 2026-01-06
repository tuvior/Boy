use crate::{
    cpu::cpu::{CPU, Cycles},
    mmu::MMU,
};

pub fn op_xxx(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let pc = cpu.r.pc.wrapping_sub(1);
    let op = mmu.rb(pc);
    panic!("Illegal opcode: 0x{op:02X} at PC=0x{pc:04X}")
}

// ALU

pub fn add8(cpu: &mut CPU, a: u8, b: u8, carry: bool) -> u8 {
    let carry_val = if carry { 1 } else { 0 };
    let sum = (a as u16) + (b as u16) + (carry_val as u16);
    let res = sum as u8;
    let h = ((a & 0x0F) + (b & 0x0F) + carry_val) > 0x0F;
    let c = sum > 0xFF;
    cpu.set_flags(res == 0, false, h, c);
    res
}

pub fn sub8(cpu: &mut CPU, a: u8, b: u8, carry: bool) -> u8 {
    let carry_val = if carry { 1 } else { 0 };
    let res = a.wrapping_sub(b).wrapping_sub(carry_val);
    let h = (a & 0x0F) < ((b & 0x0F) + carry_val);
    let c = (a as u16) < (b as u16 + carry_val as u16);
    cpu.set_flags(res == 0, true, h, c);
    res
}

pub fn inc8(cpu: &mut CPU, v: u8) -> u8 {
    let res = v.wrapping_add(1);
    cpu.r.set_z(res == 0);
    cpu.r.set_n(false);
    cpu.r.set_h((v & 0x0F) == 0x0F);
    res
}

pub fn dec8(cpu: &mut CPU, v: u8) -> u8 {
    let res = v.wrapping_sub(1);
    cpu.r.set_z(res == 0);
    cpu.r.set_n(true);
    cpu.r.set_h((v & 0x0F) == 0x00);
    res
}

pub fn add_hl(cpu: &mut CPU, v: u16) {
    let hl = cpu.r.hl();
    let res = hl.wrapping_add(v);
    cpu.r.set_n(false);
    cpu.r.set_h(((hl & 0x0FFF) + (v & 0x0FFF)) > 0x0FFF);
    cpu.r.set_c((hl as u32 + v as u32) > 0xFFFF);
    cpu.r.set_hl(res);
}

pub fn add_sp(cpu: &mut CPU, v: i8) -> u16 {
    let sp = cpu.r.sp;
    let v_u16 = v as u16;
    let res = sp.wrapping_add(v_u16);
    cpu.r.set_z(false);
    cpu.r.set_n(false);
    cpu.r.set_h(((sp & 0x0F) + (v_u16 & 0x0F)) > 0x0F);
    cpu.r.set_c(((sp & 0xFF) + (v_u16 & 0xFF)) > 0xFF);
    res
}

pub fn rlc(cpu: &mut CPU, v: u8) -> u8 {
    let c = v & 0x80 != 0;
    let res = v.rotate_left(1);
    cpu.set_flags(res == 0, false, false, c);
    res
}

pub fn rrc(cpu: &mut CPU, v: u8) -> u8 {
    let c = v & 0x01 != 0;
    let res = v.rotate_right(1);
    cpu.set_flags(res == 0, false, false, c);
    res
}

pub fn rl(cpu: &mut CPU, v: u8) -> u8 {
    let carry = cpu.r.c();
    let c = v & 0x80 != 0;
    let res = (v << 1) | if carry { 1 } else { 0 };
    cpu.set_flags(res == 0, false, false, c);
    res
}

pub fn rr(cpu: &mut CPU, v: u8) -> u8 {
    let carry = cpu.r.c();
    let c = v & 0x01 != 0;
    let res = (v >> 1) | if carry { 0x80 } else { 0 };
    cpu.set_flags(res == 0, false, false, c);
    res
}

pub fn sla(cpu: &mut CPU, v: u8) -> u8 {
    let c = v & 0x80 != 0;
    let res = v << 1;
    cpu.set_flags(res == 0, false, false, c);
    res
}

pub fn sra(cpu: &mut CPU, v: u8) -> u8 {
    let c = v & 0x01 != 0;
    let res = (v >> 1) | (v & 0x80);
    cpu.set_flags(res == 0, false, false, c);
    res
}

pub fn srl(cpu: &mut CPU, v: u8) -> u8 {
    let c = v & 0x01 != 0;
    let res = v >> 1;
    cpu.set_flags(res == 0, false, false, c);
    res
}

pub fn swap(cpu: &mut CPU, v: u8) -> u8 {
    let res = v.rotate_right(4);
    cpu.set_flags(res == 0, false, false, false);
    res
}

// Utilities

pub fn bit(cpu: &mut CPU, bit: u8, v: u8) {
    cpu.r.set_z(v & (1 << bit) == 0);
    cpu.r.set_n(false);
    cpu.r.set_h(true);
}

pub fn res(bit: u8, v: u8) -> u8 {
    v & !(1 << bit)
}

pub fn set(bit: u8, v: u8) -> u8 {
    v | (1 << bit)
}

pub fn add_a(cpu: &mut CPU, v: u8) {
    let res = add8(cpu, cpu.r.a, v, false);
    cpu.r.a = res;
}

pub fn adc_a(cpu: &mut CPU, v: u8) {
    let res = add8(cpu, cpu.r.a, v, cpu.r.c());
    cpu.r.a = res;
}

pub fn sub_a(cpu: &mut CPU, v: u8) {
    let res = sub8(cpu, cpu.r.a, v, false);
    cpu.r.a = res;
}

pub fn sbc_a(cpu: &mut CPU, v: u8) {
    let res = sub8(cpu, cpu.r.a, v, cpu.r.c());
    cpu.r.a = res;
}

pub fn and_a(cpu: &mut CPU, v: u8) {
    let res = cpu.r.a & v;
    cpu.r.a = res;
    cpu.set_flags(res == 0, false, true, false);
}

pub fn xor_a(cpu: &mut CPU, v: u8) {
    let res = cpu.r.a ^ v;
    cpu.r.a = res;
    cpu.set_flags(res == 0, false, false, false);
}

pub fn or_a(cpu: &mut CPU, v: u8) {
    let res = cpu.r.a | v;
    cpu.r.a = res;
    cpu.set_flags(res == 0, false, false, false);
}

pub fn cp_a(cpu: &mut CPU, v: u8) {
    let _ = sub8(cpu, cpu.r.a, v, false);
}

pub fn jr(cpu: &mut CPU, offset: i8) {
    cpu.r.pc = cpu.r.pc.wrapping_add(offset as i16 as u16);
}

pub fn call(cpu: &mut CPU, mmu: &mut MMU, addr: u16) {
    cpu.push(mmu, cpu.r.pc);
    cpu.r.pc = addr;
}

pub fn op_nop(_: &mut CPU, _: &mut MMU) -> Cycles {
    1
}

pub fn op_stop(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    // STOP is a 2 byte instruction
    cpu.rb(mmu);
    cpu.stop();
    1
}

pub fn op_halt(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.halt();
    1
}

// 16-bit loads
pub fn op_ld_bc_d16(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let v = cpu.rw(mmu);
    cpu.r.set_bc(v);
    3
}

pub fn op_ld_de_d16(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let v = cpu.rw(mmu);
    cpu.r.set_de(v);
    3
}

pub fn op_ld_hl_d16(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let v = cpu.rw(mmu);
    cpu.r.set_hl(v);
    3
}

pub fn op_ld_sp_d16(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let v = cpu.rw(mmu);
    cpu.r.sp = v;
    3
}

pub fn op_ld_a16_sp(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let addr = cpu.rw(mmu);
    mmu.ww(addr, cpu.r.sp);
    5
}

pub fn op_ld_sp_hl(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.sp = cpu.r.hl();
    2
}

pub fn op_ld_hl_sp_r8(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let v = cpu.rb(mmu) as i8;
    let res = add_sp(cpu, v);
    cpu.r.set_hl(res);
    3
}

pub fn op_add_sp_r8(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let v = cpu.rb(mmu) as i8;
    let res = add_sp(cpu, v);
    cpu.r.sp = res;
    4
}

// 16-bit inc/dec
pub fn op_inc_bc(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.set_bc(cpu.r.bc().wrapping_add(1));
    2
}

pub fn op_inc_de(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.set_de(cpu.r.de().wrapping_add(1));
    2
}

pub fn op_inc_hl(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.set_hl(cpu.r.hl().wrapping_add(1));
    2
}

pub fn op_inc_sp(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.sp = cpu.r.sp.wrapping_add(1);
    2
}

pub fn op_dec_bc(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.set_bc(cpu.r.bc().wrapping_sub(1));
    2
}

pub fn op_dec_de(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.set_de(cpu.r.de().wrapping_sub(1));
    2
}

pub fn op_dec_hl(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.set_hl(cpu.r.hl().wrapping_sub(1));
    2
}

pub fn op_dec_sp(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.sp = cpu.r.sp.wrapping_sub(1);
    2
}

// 16-bit add
pub fn op_add_hl_bc(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    add_hl(cpu, cpu.r.bc());
    2
}

pub fn op_add_hl_de(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    add_hl(cpu, cpu.r.de());
    2
}

pub fn op_add_hl_hl(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    add_hl(cpu, cpu.r.hl());
    2
}

pub fn op_add_hl_sp(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    add_hl(cpu, cpu.r.sp);
    2
}

// 8-bit loads (special forms)
pub fn op_ld_bc_a(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    mmu.wb(cpu.r.bc(), cpu.r.a);
    2
}

pub fn op_ld_de_a(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    mmu.wb(cpu.r.de(), cpu.r.a);
    2
}

pub fn op_ld_hli_a(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let addr = cpu.r.hl();
    mmu.wb(addr, cpu.r.a);
    cpu.r.set_hl(addr.wrapping_add(1));
    2
}

pub fn op_ld_hld_a(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let addr = cpu.r.hl();
    mmu.wb(addr, cpu.r.a);
    cpu.r.set_hl(addr.wrapping_sub(1));
    2
}

pub fn op_ld_a_bc(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    cpu.r.a = mmu.rb(cpu.r.bc());
    2
}

pub fn op_ld_a_de(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    cpu.r.a = mmu.rb(cpu.r.de());
    2
}

pub fn op_ld_a_hli(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let addr = cpu.r.hl();
    cpu.r.a = mmu.rb(addr);
    cpu.r.set_hl(addr.wrapping_add(1));
    2
}

pub fn op_ld_a_hld(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let addr = cpu.r.hl();
    cpu.r.a = mmu.rb(addr);
    cpu.r.set_hl(addr.wrapping_sub(1));
    2
}

pub fn op_ld_a16_a(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let addr = cpu.rw(mmu);
    mmu.wb(addr, cpu.r.a);
    4
}

pub fn op_ld_a_a16(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let addr = cpu.rw(mmu);
    cpu.r.a = mmu.rb(addr);
    4
}

pub fn op_ld_ff00_c_a(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let addr = 0xFF00u16 | cpu.r.c as u16;
    mmu.wb(addr, cpu.r.a);
    2
}

pub fn op_ld_a_ff00_c(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let addr = 0xFF00u16 | cpu.r.c as u16;
    cpu.r.a = mmu.rb(addr);
    2
}

pub fn op_ldh_a8_a(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let addr = 0xFF00u16 | cpu.rb(mmu) as u16;
    mmu.wb(addr, cpu.r.a);
    3
}

pub fn op_ldh_a_a8(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let addr = 0xFF00u16 | cpu.rb(mmu) as u16;
    cpu.r.a = mmu.rb(addr);
    3
}

// 8-bit inc/dec (HL)
pub fn op_inc_hl_ptr(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let addr = cpu.r.hl();
    let v = mmu.rb(addr);
    let res = inc8(cpu, v);
    mmu.wb(addr, res);
    3
}

pub fn op_dec_hl_ptr(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let addr = cpu.r.hl();
    let v = mmu.rb(addr);
    let res = dec8(cpu, v);
    mmu.wb(addr, res);
    3
}

// Jumps
pub fn op_jr(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let offset = cpu.rb(mmu) as i8;
    jr(cpu, offset);
    3
}

pub fn op_jr_nz(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let offset = cpu.rb(mmu) as i8;
    if !cpu.r.z() {
        jr(cpu, offset);
        3
    } else {
        2
    }
}

pub fn op_jr_z(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let offset = cpu.rb(mmu) as i8;
    if cpu.r.z() {
        jr(cpu, offset);
        3
    } else {
        2
    }
}

pub fn op_jr_nc(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let offset = cpu.rb(mmu) as i8;
    if !cpu.r.c() {
        jr(cpu, offset);
        3
    } else {
        2
    }
}

pub fn op_jr_c(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let offset = cpu.rb(mmu) as i8;
    if cpu.r.c() {
        jr(cpu, offset);
        3
    } else {
        2
    }
}

pub fn op_jp_a16(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let addr = cpu.rw(mmu);
    cpu.r.pc = addr;
    4
}

pub fn op_jp_hl(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.pc = cpu.r.hl();
    1
}

pub fn op_jp_nz(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let addr = cpu.rw(mmu);
    if !cpu.r.z() {
        cpu.r.pc = addr;
        4
    } else {
        3
    }
}

pub fn op_jp_z(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let addr = cpu.rw(mmu);
    if cpu.r.z() {
        cpu.r.pc = addr;
        4
    } else {
        3
    }
}

pub fn op_jp_nc(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let addr = cpu.rw(mmu);
    if !cpu.r.c() {
        cpu.r.pc = addr;
        4
    } else {
        3
    }
}

pub fn op_jp_c(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let addr = cpu.rw(mmu);
    if cpu.r.c() {
        cpu.r.pc = addr;
        4
    } else {
        3
    }
}

// Calls/returns
pub fn op_call_a16(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let addr = cpu.rw(mmu);
    call(cpu, mmu, addr);
    6
}

pub fn op_call_nz(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let addr = cpu.rw(mmu);
    if !cpu.r.z() {
        call(cpu, mmu, addr);
        6
    } else {
        3
    }
}

pub fn op_call_z(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let addr = cpu.rw(mmu);
    if cpu.r.z() {
        call(cpu, mmu, addr);
        6
    } else {
        3
    }
}

pub fn op_call_nc(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let addr = cpu.rw(mmu);
    if !cpu.r.c() {
        call(cpu, mmu, addr);
        6
    } else {
        3
    }
}

pub fn op_call_c(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let addr = cpu.rw(mmu);
    if cpu.r.c() {
        call(cpu, mmu, addr);
        6
    } else {
        3
    }
}

pub fn op_ret(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    cpu.r.pc = cpu.pop(mmu);
    4
}

pub fn op_ret_nz(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    if !cpu.r.z() {
        cpu.r.pc = cpu.pop(mmu);
        5
    } else {
        2
    }
}

pub fn op_ret_z(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    if cpu.r.z() {
        cpu.r.pc = cpu.pop(mmu);
        5
    } else {
        2
    }
}

pub fn op_ret_nc(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    if !cpu.r.c() {
        cpu.r.pc = cpu.pop(mmu);
        5
    } else {
        2
    }
}

pub fn op_ret_c(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    if cpu.r.c() {
        cpu.r.pc = cpu.pop(mmu);
        5
    } else {
        2
    }
}

pub fn op_reti(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    cpu.r.pc = cpu.pop(mmu);
    cpu.ime = true;
    4
}

// RST
pub fn op_rst_00(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    call(cpu, mmu, 0x0000);
    4
}

pub fn op_rst_08(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    call(cpu, mmu, 0x0008);
    4
}

pub fn op_rst_10(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    call(cpu, mmu, 0x0010);
    4
}

pub fn op_rst_18(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    call(cpu, mmu, 0x0018);
    4
}

pub fn op_rst_20(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    call(cpu, mmu, 0x0020);
    4
}

pub fn op_rst_28(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    call(cpu, mmu, 0x0028);
    4
}

pub fn op_rst_30(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    call(cpu, mmu, 0x0030);
    4
}

pub fn op_rst_38(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    call(cpu, mmu, 0x0038);
    4
}

// Stack
pub fn op_push_bc(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    cpu.push(mmu, cpu.r.bc());
    4
}

pub fn op_push_de(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    cpu.push(mmu, cpu.r.de());
    4
}

pub fn op_push_hl(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    cpu.push(mmu, cpu.r.hl());
    4
}

pub fn op_push_af(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    cpu.push(mmu, cpu.r.af());
    4
}

pub fn op_pop_bc(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let v = cpu.pop(mmu);
    cpu.r.set_bc(v);
    3
}

pub fn op_pop_de(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let v = cpu.pop(mmu);
    cpu.r.set_de(v);
    3
}

pub fn op_pop_hl(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let v = cpu.pop(mmu);
    cpu.r.set_hl(v);
    3
}

pub fn op_pop_af(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let v = cpu.pop(mmu);
    cpu.r.set_af(v);
    3
}

// CPU control
pub fn op_di(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.ime = false;
    cpu.set_ime_pending(false);
    1
}

pub fn op_ei(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.set_ime_pending(true);
    1
}

// Misc
pub fn op_rlca(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    let res = rlc(cpu, cpu.r.a);
    cpu.r.a = res;
    cpu.r.set_z(false);
    1
}

pub fn op_rrca(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    let res = rrc(cpu, cpu.r.a);
    cpu.r.a = res;
    cpu.r.set_z(false);
    1
}

pub fn op_rla(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    let res = rl(cpu, cpu.r.a);
    cpu.r.a = res;
    cpu.r.set_z(false);
    1
}

pub fn op_rra(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    let res = rr(cpu, cpu.r.a);
    cpu.r.a = res;
    cpu.r.set_z(false);
    1
}

pub fn op_daa(cpu: &mut CPU, _: &mut MMU) -> Cycles {
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

pub fn op_cpl(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.a = !cpu.r.a;
    cpu.r.set_n(true);
    cpu.r.set_h(true);
    1
}

pub fn op_scf(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.set_n(false);
    cpu.r.set_h(false);
    cpu.r.set_c(true);
    1
}

pub fn op_ccf(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    let c = cpu.r.c();
    cpu.r.set_n(false);
    cpu.r.set_h(false);
    cpu.r.set_c(!c);
    1
}
pub fn op_ld_b_d8(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let v = cpu.rb(mmu);
    cpu.r.b = v;
    2
}

pub fn op_ld_c_d8(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let v = cpu.rb(mmu);
    cpu.r.c = v;
    2
}

pub fn op_ld_d_d8(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let v = cpu.rb(mmu);
    cpu.r.d = v;
    2
}

pub fn op_ld_e_d8(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let v = cpu.rb(mmu);
    cpu.r.e = v;
    2
}

pub fn op_ld_h_d8(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let v = cpu.rb(mmu);
    cpu.r.h = v;
    2
}

pub fn op_ld_l_d8(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let v = cpu.rb(mmu);
    cpu.r.l = v;
    2
}

pub fn op_ld_a_d8(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let v = cpu.rb(mmu);
    cpu.r.a = v;
    2
}

pub fn op_ld_hl_d8(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let v = cpu.rb(mmu);
    let addr = cpu.r.hl();
    mmu.wb(addr, v);
    3
}
pub fn op_inc_b(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.b = inc8(cpu, cpu.r.b);
    1
}

pub fn op_dec_b(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.b = dec8(cpu, cpu.r.b);
    1
}

pub fn op_inc_c(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.c = inc8(cpu, cpu.r.c);
    1
}

pub fn op_dec_c(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.c = dec8(cpu, cpu.r.c);
    1
}

pub fn op_inc_d(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.d = inc8(cpu, cpu.r.d);
    1
}

pub fn op_dec_d(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.d = dec8(cpu, cpu.r.d);
    1
}

pub fn op_inc_e(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.e = inc8(cpu, cpu.r.e);
    1
}

pub fn op_dec_e(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.e = dec8(cpu, cpu.r.e);
    1
}

pub fn op_inc_h(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.h = inc8(cpu, cpu.r.h);
    1
}

pub fn op_dec_h(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.h = dec8(cpu, cpu.r.h);
    1
}

pub fn op_inc_l(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.l = inc8(cpu, cpu.r.l);
    1
}

pub fn op_dec_l(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.l = dec8(cpu, cpu.r.l);
    1
}

pub fn op_inc_a(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.a = inc8(cpu, cpu.r.a);
    1
}

pub fn op_dec_a(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.a = dec8(cpu, cpu.r.a);
    1
}

#[allow(clippy::self_assignment, dead_code)]
pub fn op_ld_b_b(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.b = cpu.r.b;
    1
}

pub fn op_ld_b_c(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.b = cpu.r.c;
    1
}

pub fn op_ld_b_d(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.b = cpu.r.d;
    1
}

pub fn op_ld_b_e(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.b = cpu.r.e;
    1
}

pub fn op_ld_b_h(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.b = cpu.r.h;
    1
}

pub fn op_ld_b_l(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.b = cpu.r.l;
    1
}

pub fn op_ld_b_hl(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    cpu.r.b = mmu.rb(cpu.r.hl());
    2
}

pub fn op_ld_b_a(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.b = cpu.r.a;
    1
}

pub fn op_ld_c_b(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.c = cpu.r.b;
    1
}

#[allow(clippy::self_assignment, dead_code)]
pub fn op_ld_c_c(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.c = cpu.r.c;
    1
}

pub fn op_ld_c_d(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.c = cpu.r.d;
    1
}

pub fn op_ld_c_e(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.c = cpu.r.e;
    1
}

pub fn op_ld_c_h(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.c = cpu.r.h;
    1
}

pub fn op_ld_c_l(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.c = cpu.r.l;
    1
}

pub fn op_ld_c_hl(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    cpu.r.c = mmu.rb(cpu.r.hl());
    2
}

pub fn op_ld_c_a(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.c = cpu.r.a;
    1
}

pub fn op_ld_d_b(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.d = cpu.r.b;
    1
}

pub fn op_ld_d_c(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.d = cpu.r.c;
    1
}

#[allow(clippy::self_assignment, dead_code)]
pub fn op_ld_d_d(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.d = cpu.r.d;
    1
}

pub fn op_ld_d_e(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.d = cpu.r.e;
    1
}

pub fn op_ld_d_h(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.d = cpu.r.h;
    1
}

pub fn op_ld_d_l(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.d = cpu.r.l;
    1
}

pub fn op_ld_d_hl(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    cpu.r.d = mmu.rb(cpu.r.hl());
    2
}

pub fn op_ld_d_a(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.d = cpu.r.a;
    1
}

pub fn op_ld_e_b(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.e = cpu.r.b;
    1
}

pub fn op_ld_e_c(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.e = cpu.r.c;
    1
}

pub fn op_ld_e_d(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.e = cpu.r.d;
    1
}

#[allow(clippy::self_assignment, dead_code)]
pub fn op_ld_e_e(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.e = cpu.r.e;
    1
}

pub fn op_ld_e_h(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.e = cpu.r.h;
    1
}

pub fn op_ld_e_l(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.e = cpu.r.l;
    1
}

pub fn op_ld_e_hl(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    cpu.r.e = mmu.rb(cpu.r.hl());
    2
}

pub fn op_ld_e_a(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.e = cpu.r.a;
    1
}

pub fn op_ld_h_b(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.h = cpu.r.b;
    1
}

pub fn op_ld_h_c(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.h = cpu.r.c;
    1
}

pub fn op_ld_h_d(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.h = cpu.r.d;
    1
}

pub fn op_ld_h_e(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.h = cpu.r.e;
    1
}

#[allow(clippy::self_assignment, dead_code)]
pub fn op_ld_h_h(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.h = cpu.r.h;
    1
}

pub fn op_ld_h_l(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.h = cpu.r.l;
    1
}

pub fn op_ld_h_hl(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    cpu.r.h = mmu.rb(cpu.r.hl());
    2
}

pub fn op_ld_h_a(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.h = cpu.r.a;
    1
}

pub fn op_ld_l_b(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.l = cpu.r.b;
    1
}

pub fn op_ld_l_c(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.l = cpu.r.c;
    1
}

pub fn op_ld_l_d(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.l = cpu.r.d;
    1
}

pub fn op_ld_l_e(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.l = cpu.r.e;
    1
}

pub fn op_ld_l_h(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.l = cpu.r.h;
    1
}

#[allow(clippy::self_assignment, dead_code)]
pub fn op_ld_l_l(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.l = cpu.r.l;
    1
}

pub fn op_ld_l_hl(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    cpu.r.l = mmu.rb(cpu.r.hl());
    2
}

pub fn op_ld_l_a(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.l = cpu.r.a;
    1
}

pub fn op_ld_hl_b(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let addr = cpu.r.hl();
    mmu.wb(addr, cpu.r.b);
    2
}

pub fn op_ld_hl_c(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let addr = cpu.r.hl();
    mmu.wb(addr, cpu.r.c);
    2
}

pub fn op_ld_hl_d(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let addr = cpu.r.hl();
    mmu.wb(addr, cpu.r.d);
    2
}

pub fn op_ld_hl_e(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let addr = cpu.r.hl();
    mmu.wb(addr, cpu.r.e);
    2
}

pub fn op_ld_hl_h(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let addr = cpu.r.hl();
    mmu.wb(addr, cpu.r.h);
    2
}

pub fn op_ld_hl_l(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let addr = cpu.r.hl();
    mmu.wb(addr, cpu.r.l);
    2
}

pub fn op_ld_hl_a(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let addr = cpu.r.hl();
    mmu.wb(addr, cpu.r.a);
    2
}

pub fn op_ld_a_b(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.a = cpu.r.b;
    1
}

pub fn op_ld_a_c(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.a = cpu.r.c;
    1
}

pub fn op_ld_a_d(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.a = cpu.r.d;
    1
}

pub fn op_ld_a_e(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.a = cpu.r.e;
    1
}

pub fn op_ld_a_h(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.a = cpu.r.h;
    1
}

pub fn op_ld_a_l(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.a = cpu.r.l;
    1
}

pub fn op_ld_a_hl(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    cpu.r.a = mmu.rb(cpu.r.hl());
    2
}

#[allow(clippy::self_assignment, dead_code)]
pub fn op_ld_a_a(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.a = cpu.r.a;
    1
}

pub fn op_add_a_b(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    add_a(cpu, cpu.r.b);
    1
}

pub fn op_add_a_c(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    add_a(cpu, cpu.r.c);
    1
}

pub fn op_add_a_d(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    add_a(cpu, cpu.r.d);
    1
}

pub fn op_add_a_e(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    add_a(cpu, cpu.r.e);
    1
}

pub fn op_add_a_h(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    add_a(cpu, cpu.r.h);
    1
}

pub fn op_add_a_l(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    add_a(cpu, cpu.r.l);
    1
}

pub fn op_add_a_hl(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let v = mmu.rb(cpu.r.hl());
    add_a(cpu, v);
    2
}

pub fn op_add_a_a(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    add_a(cpu, cpu.r.a);
    1
}

pub fn op_adc_a_b(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    adc_a(cpu, cpu.r.b);
    1
}

pub fn op_adc_a_c(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    adc_a(cpu, cpu.r.c);
    1
}

pub fn op_adc_a_d(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    adc_a(cpu, cpu.r.d);
    1
}

pub fn op_adc_a_e(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    adc_a(cpu, cpu.r.e);
    1
}

pub fn op_adc_a_h(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    adc_a(cpu, cpu.r.h);
    1
}

pub fn op_adc_a_l(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    adc_a(cpu, cpu.r.l);
    1
}

pub fn op_adc_a_hl(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let v = mmu.rb(cpu.r.hl());
    adc_a(cpu, v);
    2
}

pub fn op_adc_a_a(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    adc_a(cpu, cpu.r.a);
    1
}

pub fn op_sub_b(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    sub_a(cpu, cpu.r.b);
    1
}

pub fn op_sub_c(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    sub_a(cpu, cpu.r.c);
    1
}

pub fn op_sub_d(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    sub_a(cpu, cpu.r.d);
    1
}

pub fn op_sub_e(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    sub_a(cpu, cpu.r.e);
    1
}

pub fn op_sub_h(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    sub_a(cpu, cpu.r.h);
    1
}

pub fn op_sub_l(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    sub_a(cpu, cpu.r.l);
    1
}

pub fn op_sub_hl(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let v = mmu.rb(cpu.r.hl());
    sub_a(cpu, v);
    2
}

pub fn op_sub_a(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    sub_a(cpu, cpu.r.a);
    1
}

pub fn op_sbc_a_b(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    sbc_a(cpu, cpu.r.b);
    1
}

pub fn op_sbc_a_c(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    sbc_a(cpu, cpu.r.c);
    1
}

pub fn op_sbc_a_d(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    sbc_a(cpu, cpu.r.d);
    1
}

pub fn op_sbc_a_e(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    sbc_a(cpu, cpu.r.e);
    1
}

pub fn op_sbc_a_h(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    sbc_a(cpu, cpu.r.h);
    1
}

pub fn op_sbc_a_l(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    sbc_a(cpu, cpu.r.l);
    1
}

pub fn op_sbc_a_hl(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let v = mmu.rb(cpu.r.hl());
    sbc_a(cpu, v);
    2
}

pub fn op_sbc_a_a(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    sbc_a(cpu, cpu.r.a);
    1
}

pub fn op_and_b(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    and_a(cpu, cpu.r.b);
    1
}

pub fn op_and_c(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    and_a(cpu, cpu.r.c);
    1
}

pub fn op_and_d(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    and_a(cpu, cpu.r.d);
    1
}

pub fn op_and_e(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    and_a(cpu, cpu.r.e);
    1
}

pub fn op_and_h(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    and_a(cpu, cpu.r.h);
    1
}

pub fn op_and_l(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    and_a(cpu, cpu.r.l);
    1
}

pub fn op_and_hl(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let v = mmu.rb(cpu.r.hl());
    and_a(cpu, v);
    2
}

pub fn op_and_a(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    and_a(cpu, cpu.r.a);
    1
}

pub fn op_xor_b(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    xor_a(cpu, cpu.r.b);
    1
}

pub fn op_xor_c(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    xor_a(cpu, cpu.r.c);
    1
}

pub fn op_xor_d(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    xor_a(cpu, cpu.r.d);
    1
}

pub fn op_xor_e(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    xor_a(cpu, cpu.r.e);
    1
}

pub fn op_xor_h(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    xor_a(cpu, cpu.r.h);
    1
}

pub fn op_xor_l(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    xor_a(cpu, cpu.r.l);
    1
}

pub fn op_xor_hl(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let v = mmu.rb(cpu.r.hl());
    xor_a(cpu, v);
    2
}

pub fn op_xor_a(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    xor_a(cpu, cpu.r.a);
    1
}

pub fn op_or_b(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    or_a(cpu, cpu.r.b);
    1
}

pub fn op_or_c(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    or_a(cpu, cpu.r.c);
    1
}

pub fn op_or_d(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    or_a(cpu, cpu.r.d);
    1
}

pub fn op_or_e(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    or_a(cpu, cpu.r.e);
    1
}

pub fn op_or_h(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    or_a(cpu, cpu.r.h);
    1
}

pub fn op_or_l(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    or_a(cpu, cpu.r.l);
    1
}

pub fn op_or_hl(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let v = mmu.rb(cpu.r.hl());
    or_a(cpu, v);
    2
}

pub fn op_or_a(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    or_a(cpu, cpu.r.a);
    1
}

pub fn op_cp_b(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cp_a(cpu, cpu.r.b);
    1
}

pub fn op_cp_c(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cp_a(cpu, cpu.r.c);
    1
}

pub fn op_cp_d(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cp_a(cpu, cpu.r.d);
    1
}

pub fn op_cp_e(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cp_a(cpu, cpu.r.e);
    1
}

pub fn op_cp_h(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cp_a(cpu, cpu.r.h);
    1
}

pub fn op_cp_l(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cp_a(cpu, cpu.r.l);
    1
}

pub fn op_cp_hl(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let v = mmu.rb(cpu.r.hl());
    cp_a(cpu, v);
    2
}

pub fn op_cp_a(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cp_a(cpu, cpu.r.a);
    1
}

pub fn op_add_a_d8(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let v = cpu.rb(mmu);
    add_a(cpu, v);
    2
}

pub fn op_adc_a_d8(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let v = cpu.rb(mmu);
    adc_a(cpu, v);
    2
}

pub fn op_sub_d8(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let v = cpu.rb(mmu);
    sub_a(cpu, v);
    2
}

pub fn op_sbc_a_d8(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let v = cpu.rb(mmu);
    sbc_a(cpu, v);
    2
}

pub fn op_and_d8(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let v = cpu.rb(mmu);
    and_a(cpu, v);
    2
}

pub fn op_xor_d8(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let v = cpu.rb(mmu);
    xor_a(cpu, v);
    2
}

pub fn op_or_d8(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let v = cpu.rb(mmu);
    or_a(cpu, v);
    2
}

pub fn op_cp_d8(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let v = cpu.rb(mmu);
    cp_a(cpu, v);
    2
}

pub fn cb_rlc_b(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.b = rlc(cpu, cpu.r.b);
    2
}

pub fn cb_rlc_c(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.c = rlc(cpu, cpu.r.c);
    2
}

pub fn cb_rlc_d(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.d = rlc(cpu, cpu.r.d);
    2
}

pub fn cb_rlc_e(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.e = rlc(cpu, cpu.r.e);
    2
}

pub fn cb_rlc_h(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.h = rlc(cpu, cpu.r.h);
    2
}

pub fn cb_rlc_l(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.l = rlc(cpu, cpu.r.l);
    2
}

pub fn cb_rlc_hl(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let addr = cpu.r.hl();
    let v = mmu.rb(addr);
    let res = rlc(cpu, v);
    mmu.wb(addr, res);
    4
}

pub fn cb_rlc_a(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.a = rlc(cpu, cpu.r.a);
    2
}

pub fn cb_rrc_b(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.b = rrc(cpu, cpu.r.b);
    2
}

pub fn cb_rrc_c(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.c = rrc(cpu, cpu.r.c);
    2
}

pub fn cb_rrc_d(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.d = rrc(cpu, cpu.r.d);
    2
}

pub fn cb_rrc_e(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.e = rrc(cpu, cpu.r.e);
    2
}

pub fn cb_rrc_h(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.h = rrc(cpu, cpu.r.h);
    2
}

pub fn cb_rrc_l(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.l = rrc(cpu, cpu.r.l);
    2
}

pub fn cb_rrc_hl(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let addr = cpu.r.hl();
    let v = mmu.rb(addr);
    let res = rrc(cpu, v);
    mmu.wb(addr, res);
    4
}

pub fn cb_rrc_a(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.a = rrc(cpu, cpu.r.a);
    2
}

pub fn cb_rl_b(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.b = rl(cpu, cpu.r.b);
    2
}

pub fn cb_rl_c(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.c = rl(cpu, cpu.r.c);
    2
}

pub fn cb_rl_d(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.d = rl(cpu, cpu.r.d);
    2
}

pub fn cb_rl_e(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.e = rl(cpu, cpu.r.e);
    2
}

pub fn cb_rl_h(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.h = rl(cpu, cpu.r.h);
    2
}

pub fn cb_rl_l(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.l = rl(cpu, cpu.r.l);
    2
}

pub fn cb_rl_hl(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let addr = cpu.r.hl();
    let v = mmu.rb(addr);
    let res = rl(cpu, v);
    mmu.wb(addr, res);
    4
}

pub fn cb_rl_a(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.a = rl(cpu, cpu.r.a);
    2
}

pub fn cb_rr_b(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.b = rr(cpu, cpu.r.b);
    2
}

pub fn cb_rr_c(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.c = rr(cpu, cpu.r.c);
    2
}

pub fn cb_rr_d(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.d = rr(cpu, cpu.r.d);
    2
}

pub fn cb_rr_e(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.e = rr(cpu, cpu.r.e);
    2
}

pub fn cb_rr_h(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.h = rr(cpu, cpu.r.h);
    2
}

pub fn cb_rr_l(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.l = rr(cpu, cpu.r.l);
    2
}

pub fn cb_rr_hl(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let addr = cpu.r.hl();
    let v = mmu.rb(addr);
    let res = rr(cpu, v);
    mmu.wb(addr, res);
    4
}

pub fn cb_rr_a(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.a = rr(cpu, cpu.r.a);
    2
}

pub fn cb_sla_b(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.b = sla(cpu, cpu.r.b);
    2
}

pub fn cb_sla_c(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.c = sla(cpu, cpu.r.c);
    2
}

pub fn cb_sla_d(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.d = sla(cpu, cpu.r.d);
    2
}

pub fn cb_sla_e(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.e = sla(cpu, cpu.r.e);
    2
}

pub fn cb_sla_h(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.h = sla(cpu, cpu.r.h);
    2
}

pub fn cb_sla_l(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.l = sla(cpu, cpu.r.l);
    2
}

pub fn cb_sla_hl(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let addr = cpu.r.hl();
    let v = mmu.rb(addr);
    let res = sla(cpu, v);
    mmu.wb(addr, res);
    4
}

pub fn cb_sla_a(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.a = sla(cpu, cpu.r.a);
    2
}

pub fn cb_sra_b(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.b = sra(cpu, cpu.r.b);
    2
}

pub fn cb_sra_c(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.c = sra(cpu, cpu.r.c);
    2
}

pub fn cb_sra_d(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.d = sra(cpu, cpu.r.d);
    2
}

pub fn cb_sra_e(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.e = sra(cpu, cpu.r.e);
    2
}

pub fn cb_sra_h(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.h = sra(cpu, cpu.r.h);
    2
}

pub fn cb_sra_l(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.l = sra(cpu, cpu.r.l);
    2
}

pub fn cb_sra_hl(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let addr = cpu.r.hl();
    let v = mmu.rb(addr);
    let res = sra(cpu, v);
    mmu.wb(addr, res);
    4
}

pub fn cb_sra_a(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.a = sra(cpu, cpu.r.a);
    2
}

pub fn cb_swap_b(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.b = swap(cpu, cpu.r.b);
    2
}

pub fn cb_swap_c(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.c = swap(cpu, cpu.r.c);
    2
}

pub fn cb_swap_d(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.d = swap(cpu, cpu.r.d);
    2
}

pub fn cb_swap_e(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.e = swap(cpu, cpu.r.e);
    2
}

pub fn cb_swap_h(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.h = swap(cpu, cpu.r.h);
    2
}

pub fn cb_swap_l(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.l = swap(cpu, cpu.r.l);
    2
}

pub fn cb_swap_hl(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let addr = cpu.r.hl();
    let v = mmu.rb(addr);
    let res = swap(cpu, v);
    mmu.wb(addr, res);
    4
}

pub fn cb_swap_a(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.a = swap(cpu, cpu.r.a);
    2
}

pub fn cb_srl_b(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.b = srl(cpu, cpu.r.b);
    2
}

pub fn cb_srl_c(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.c = srl(cpu, cpu.r.c);
    2
}

pub fn cb_srl_d(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.d = srl(cpu, cpu.r.d);
    2
}

pub fn cb_srl_e(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.e = srl(cpu, cpu.r.e);
    2
}

pub fn cb_srl_h(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.h = srl(cpu, cpu.r.h);
    2
}

pub fn cb_srl_l(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.l = srl(cpu, cpu.r.l);
    2
}

pub fn cb_srl_hl(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let addr = cpu.r.hl();
    let v = mmu.rb(addr);
    let res = srl(cpu, v);
    mmu.wb(addr, res);
    4
}

pub fn cb_srl_a(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.a = srl(cpu, cpu.r.a);
    2
}

pub fn cb_bit_0_b(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    bit(cpu, 0, cpu.r.b);
    2
}

pub fn cb_bit_0_c(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    bit(cpu, 0, cpu.r.c);
    2
}

pub fn cb_bit_0_d(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    bit(cpu, 0, cpu.r.d);
    2
}

pub fn cb_bit_0_e(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    bit(cpu, 0, cpu.r.e);
    2
}

pub fn cb_bit_0_h(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    bit(cpu, 0, cpu.r.h);
    2
}

pub fn cb_bit_0_l(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    bit(cpu, 0, cpu.r.l);
    2
}

pub fn cb_bit_0_hl(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let v = mmu.rb(cpu.r.hl());
    bit(cpu, 0, v);
    3
}

pub fn cb_bit_0_a(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    bit(cpu, 0, cpu.r.a);
    2
}

pub fn cb_bit_1_b(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    bit(cpu, 1, cpu.r.b);
    2
}

pub fn cb_bit_1_c(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    bit(cpu, 1, cpu.r.c);
    2
}

pub fn cb_bit_1_d(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    bit(cpu, 1, cpu.r.d);
    2
}

pub fn cb_bit_1_e(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    bit(cpu, 1, cpu.r.e);
    2
}

pub fn cb_bit_1_h(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    bit(cpu, 1, cpu.r.h);
    2
}

pub fn cb_bit_1_l(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    bit(cpu, 1, cpu.r.l);
    2
}

pub fn cb_bit_1_hl(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let v = mmu.rb(cpu.r.hl());
    bit(cpu, 1, v);
    3
}

pub fn cb_bit_1_a(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    bit(cpu, 1, cpu.r.a);
    2
}

pub fn cb_bit_2_b(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    bit(cpu, 2, cpu.r.b);
    2
}

pub fn cb_bit_2_c(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    bit(cpu, 2, cpu.r.c);
    2
}

pub fn cb_bit_2_d(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    bit(cpu, 2, cpu.r.d);
    2
}

pub fn cb_bit_2_e(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    bit(cpu, 2, cpu.r.e);
    2
}

pub fn cb_bit_2_h(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    bit(cpu, 2, cpu.r.h);
    2
}

pub fn cb_bit_2_l(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    bit(cpu, 2, cpu.r.l);
    2
}

pub fn cb_bit_2_hl(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let v = mmu.rb(cpu.r.hl());
    bit(cpu, 2, v);
    3
}

pub fn cb_bit_2_a(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    bit(cpu, 2, cpu.r.a);
    2
}

pub fn cb_bit_3_b(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    bit(cpu, 3, cpu.r.b);
    2
}

pub fn cb_bit_3_c(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    bit(cpu, 3, cpu.r.c);
    2
}

pub fn cb_bit_3_d(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    bit(cpu, 3, cpu.r.d);
    2
}

pub fn cb_bit_3_e(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    bit(cpu, 3, cpu.r.e);
    2
}

pub fn cb_bit_3_h(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    bit(cpu, 3, cpu.r.h);
    2
}

pub fn cb_bit_3_l(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    bit(cpu, 3, cpu.r.l);
    2
}

pub fn cb_bit_3_hl(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let v = mmu.rb(cpu.r.hl());
    bit(cpu, 3, v);
    3
}

pub fn cb_bit_3_a(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    bit(cpu, 3, cpu.r.a);
    2
}

pub fn cb_bit_4_b(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    bit(cpu, 4, cpu.r.b);
    2
}

pub fn cb_bit_4_c(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    bit(cpu, 4, cpu.r.c);
    2
}

pub fn cb_bit_4_d(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    bit(cpu, 4, cpu.r.d);
    2
}

pub fn cb_bit_4_e(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    bit(cpu, 4, cpu.r.e);
    2
}

pub fn cb_bit_4_h(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    bit(cpu, 4, cpu.r.h);
    2
}

pub fn cb_bit_4_l(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    bit(cpu, 4, cpu.r.l);
    2
}

pub fn cb_bit_4_hl(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let v = mmu.rb(cpu.r.hl());
    bit(cpu, 4, v);
    3
}

pub fn cb_bit_4_a(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    bit(cpu, 4, cpu.r.a);
    2
}

pub fn cb_bit_5_b(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    bit(cpu, 5, cpu.r.b);
    2
}

pub fn cb_bit_5_c(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    bit(cpu, 5, cpu.r.c);
    2
}

pub fn cb_bit_5_d(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    bit(cpu, 5, cpu.r.d);
    2
}

pub fn cb_bit_5_e(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    bit(cpu, 5, cpu.r.e);
    2
}

pub fn cb_bit_5_h(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    bit(cpu, 5, cpu.r.h);
    2
}

pub fn cb_bit_5_l(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    bit(cpu, 5, cpu.r.l);
    2
}

pub fn cb_bit_5_hl(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let v = mmu.rb(cpu.r.hl());
    bit(cpu, 5, v);
    3
}

pub fn cb_bit_5_a(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    bit(cpu, 5, cpu.r.a);
    2
}

pub fn cb_bit_6_b(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    bit(cpu, 6, cpu.r.b);
    2
}

pub fn cb_bit_6_c(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    bit(cpu, 6, cpu.r.c);
    2
}

pub fn cb_bit_6_d(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    bit(cpu, 6, cpu.r.d);
    2
}

pub fn cb_bit_6_e(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    bit(cpu, 6, cpu.r.e);
    2
}

pub fn cb_bit_6_h(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    bit(cpu, 6, cpu.r.h);
    2
}

pub fn cb_bit_6_l(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    bit(cpu, 6, cpu.r.l);
    2
}

pub fn cb_bit_6_hl(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let v = mmu.rb(cpu.r.hl());
    bit(cpu, 6, v);
    3
}

pub fn cb_bit_6_a(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    bit(cpu, 6, cpu.r.a);
    2
}

pub fn cb_bit_7_b(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    bit(cpu, 7, cpu.r.b);
    2
}

pub fn cb_bit_7_c(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    bit(cpu, 7, cpu.r.c);
    2
}

pub fn cb_bit_7_d(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    bit(cpu, 7, cpu.r.d);
    2
}

pub fn cb_bit_7_e(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    bit(cpu, 7, cpu.r.e);
    2
}

pub fn cb_bit_7_h(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    bit(cpu, 7, cpu.r.h);
    2
}

pub fn cb_bit_7_l(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    bit(cpu, 7, cpu.r.l);
    2
}

pub fn cb_bit_7_hl(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let v = mmu.rb(cpu.r.hl());
    bit(cpu, 7, v);
    3
}

pub fn cb_bit_7_a(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    bit(cpu, 7, cpu.r.a);
    2
}

pub fn cb_res_0_b(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.b = res(0, cpu.r.b);
    2
}

pub fn cb_res_0_c(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.c = res(0, cpu.r.c);
    2
}

pub fn cb_res_0_d(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.d = res(0, cpu.r.d);
    2
}

pub fn cb_res_0_e(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.e = res(0, cpu.r.e);
    2
}

pub fn cb_res_0_h(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.h = res(0, cpu.r.h);
    2
}

pub fn cb_res_0_l(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.l = res(0, cpu.r.l);
    2
}

pub fn cb_res_0_hl(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let addr = cpu.r.hl();
    let v = mmu.rb(addr);
    let res = res(0, v);
    mmu.wb(addr, res);
    4
}

pub fn cb_res_0_a(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.a = res(0, cpu.r.a);
    2
}

pub fn cb_res_1_b(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.b = res(1, cpu.r.b);
    2
}

pub fn cb_res_1_c(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.c = res(1, cpu.r.c);
    2
}

pub fn cb_res_1_d(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.d = res(1, cpu.r.d);
    2
}

pub fn cb_res_1_e(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.e = res(1, cpu.r.e);
    2
}

pub fn cb_res_1_h(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.h = res(1, cpu.r.h);
    2
}

pub fn cb_res_1_l(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.l = res(1, cpu.r.l);
    2
}

pub fn cb_res_1_hl(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let addr = cpu.r.hl();
    let v = mmu.rb(addr);
    let res = res(1, v);
    mmu.wb(addr, res);
    4
}

pub fn cb_res_1_a(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.a = res(1, cpu.r.a);
    2
}

pub fn cb_res_2_b(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.b = res(2, cpu.r.b);
    2
}

pub fn cb_res_2_c(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.c = res(2, cpu.r.c);
    2
}

pub fn cb_res_2_d(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.d = res(2, cpu.r.d);
    2
}

pub fn cb_res_2_e(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.e = res(2, cpu.r.e);
    2
}

pub fn cb_res_2_h(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.h = res(2, cpu.r.h);
    2
}

pub fn cb_res_2_l(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.l = res(2, cpu.r.l);
    2
}

pub fn cb_res_2_hl(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let addr = cpu.r.hl();
    let v = mmu.rb(addr);
    let res = res(2, v);
    mmu.wb(addr, res);
    4
}

pub fn cb_res_2_a(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.a = res(2, cpu.r.a);
    2
}

pub fn cb_res_3_b(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.b = res(3, cpu.r.b);
    2
}

pub fn cb_res_3_c(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.c = res(3, cpu.r.c);
    2
}

pub fn cb_res_3_d(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.d = res(3, cpu.r.d);
    2
}

pub fn cb_res_3_e(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.e = res(3, cpu.r.e);
    2
}

pub fn cb_res_3_h(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.h = res(3, cpu.r.h);
    2
}

pub fn cb_res_3_l(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.l = res(3, cpu.r.l);
    2
}

pub fn cb_res_3_hl(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let addr = cpu.r.hl();
    let v = mmu.rb(addr);
    let res = res(3, v);
    mmu.wb(addr, res);
    4
}

pub fn cb_res_3_a(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.a = res(3, cpu.r.a);
    2
}

pub fn cb_res_4_b(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.b = res(4, cpu.r.b);
    2
}

pub fn cb_res_4_c(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.c = res(4, cpu.r.c);
    2
}

pub fn cb_res_4_d(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.d = res(4, cpu.r.d);
    2
}

pub fn cb_res_4_e(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.e = res(4, cpu.r.e);
    2
}

pub fn cb_res_4_h(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.h = res(4, cpu.r.h);
    2
}

pub fn cb_res_4_l(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.l = res(4, cpu.r.l);
    2
}

pub fn cb_res_4_hl(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let addr = cpu.r.hl();
    let v = mmu.rb(addr);
    let res = res(4, v);
    mmu.wb(addr, res);
    4
}

pub fn cb_res_4_a(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.a = res(4, cpu.r.a);
    2
}

pub fn cb_res_5_b(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.b = res(5, cpu.r.b);
    2
}

pub fn cb_res_5_c(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.c = res(5, cpu.r.c);
    2
}

pub fn cb_res_5_d(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.d = res(5, cpu.r.d);
    2
}

pub fn cb_res_5_e(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.e = res(5, cpu.r.e);
    2
}

pub fn cb_res_5_h(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.h = res(5, cpu.r.h);
    2
}

pub fn cb_res_5_l(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.l = res(5, cpu.r.l);
    2
}

pub fn cb_res_5_hl(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let addr = cpu.r.hl();
    let v = mmu.rb(addr);
    let res = res(5, v);
    mmu.wb(addr, res);
    4
}

pub fn cb_res_5_a(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.a = res(5, cpu.r.a);
    2
}

pub fn cb_res_6_b(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.b = res(6, cpu.r.b);
    2
}

pub fn cb_res_6_c(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.c = res(6, cpu.r.c);
    2
}

pub fn cb_res_6_d(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.d = res(6, cpu.r.d);
    2
}

pub fn cb_res_6_e(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.e = res(6, cpu.r.e);
    2
}

pub fn cb_res_6_h(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.h = res(6, cpu.r.h);
    2
}

pub fn cb_res_6_l(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.l = res(6, cpu.r.l);
    2
}

pub fn cb_res_6_hl(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let addr = cpu.r.hl();
    let v = mmu.rb(addr);
    let res = res(6, v);
    mmu.wb(addr, res);
    4
}

pub fn cb_res_6_a(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.a = res(6, cpu.r.a);
    2
}

pub fn cb_res_7_b(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.b = res(7, cpu.r.b);
    2
}

pub fn cb_res_7_c(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.c = res(7, cpu.r.c);
    2
}

pub fn cb_res_7_d(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.d = res(7, cpu.r.d);
    2
}

pub fn cb_res_7_e(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.e = res(7, cpu.r.e);
    2
}

pub fn cb_res_7_h(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.h = res(7, cpu.r.h);
    2
}

pub fn cb_res_7_l(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.l = res(7, cpu.r.l);
    2
}

pub fn cb_res_7_hl(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let addr = cpu.r.hl();
    let v = mmu.rb(addr);
    let res = res(7, v);
    mmu.wb(addr, res);
    4
}

pub fn cb_res_7_a(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.a = res(7, cpu.r.a);
    2
}

pub fn cb_set_0_b(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.b = set(0, cpu.r.b);
    2
}

pub fn cb_set_0_c(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.c = set(0, cpu.r.c);
    2
}

pub fn cb_set_0_d(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.d = set(0, cpu.r.d);
    2
}

pub fn cb_set_0_e(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.e = set(0, cpu.r.e);
    2
}

pub fn cb_set_0_h(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.h = set(0, cpu.r.h);
    2
}

pub fn cb_set_0_l(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.l = set(0, cpu.r.l);
    2
}

pub fn cb_set_0_hl(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let addr = cpu.r.hl();
    let v = mmu.rb(addr);
    let res = set(0, v);
    mmu.wb(addr, res);
    4
}

pub fn cb_set_0_a(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.a = set(0, cpu.r.a);
    2
}

pub fn cb_set_1_b(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.b = set(1, cpu.r.b);
    2
}

pub fn cb_set_1_c(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.c = set(1, cpu.r.c);
    2
}

pub fn cb_set_1_d(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.d = set(1, cpu.r.d);
    2
}

pub fn cb_set_1_e(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.e = set(1, cpu.r.e);
    2
}

pub fn cb_set_1_h(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.h = set(1, cpu.r.h);
    2
}

pub fn cb_set_1_l(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.l = set(1, cpu.r.l);
    2
}

pub fn cb_set_1_hl(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let addr = cpu.r.hl();
    let v = mmu.rb(addr);
    let res = set(1, v);
    mmu.wb(addr, res);
    4
}

pub fn cb_set_1_a(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.a = set(1, cpu.r.a);
    2
}

pub fn cb_set_2_b(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.b = set(2, cpu.r.b);
    2
}

pub fn cb_set_2_c(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.c = set(2, cpu.r.c);
    2
}

pub fn cb_set_2_d(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.d = set(2, cpu.r.d);
    2
}

pub fn cb_set_2_e(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.e = set(2, cpu.r.e);
    2
}

pub fn cb_set_2_h(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.h = set(2, cpu.r.h);
    2
}

pub fn cb_set_2_l(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.l = set(2, cpu.r.l);
    2
}

pub fn cb_set_2_hl(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let addr = cpu.r.hl();
    let v = mmu.rb(addr);
    let res = set(2, v);
    mmu.wb(addr, res);
    4
}

pub fn cb_set_2_a(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.a = set(2, cpu.r.a);
    2
}

pub fn cb_set_3_b(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.b = set(3, cpu.r.b);
    2
}

pub fn cb_set_3_c(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.c = set(3, cpu.r.c);
    2
}

pub fn cb_set_3_d(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.d = set(3, cpu.r.d);
    2
}

pub fn cb_set_3_e(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.e = set(3, cpu.r.e);
    2
}

pub fn cb_set_3_h(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.h = set(3, cpu.r.h);
    2
}

pub fn cb_set_3_l(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.l = set(3, cpu.r.l);
    2
}

pub fn cb_set_3_hl(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let addr = cpu.r.hl();
    let v = mmu.rb(addr);
    let res = set(3, v);
    mmu.wb(addr, res);
    4
}

pub fn cb_set_3_a(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.a = set(3, cpu.r.a);
    2
}

pub fn cb_set_4_b(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.b = set(4, cpu.r.b);
    2
}

pub fn cb_set_4_c(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.c = set(4, cpu.r.c);
    2
}

pub fn cb_set_4_d(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.d = set(4, cpu.r.d);
    2
}

pub fn cb_set_4_e(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.e = set(4, cpu.r.e);
    2
}

pub fn cb_set_4_h(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.h = set(4, cpu.r.h);
    2
}

pub fn cb_set_4_l(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.l = set(4, cpu.r.l);
    2
}

pub fn cb_set_4_hl(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let addr = cpu.r.hl();
    let v = mmu.rb(addr);
    let res = set(4, v);
    mmu.wb(addr, res);
    4
}

pub fn cb_set_4_a(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.a = set(4, cpu.r.a);
    2
}

pub fn cb_set_5_b(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.b = set(5, cpu.r.b);
    2
}

pub fn cb_set_5_c(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.c = set(5, cpu.r.c);
    2
}

pub fn cb_set_5_d(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.d = set(5, cpu.r.d);
    2
}

pub fn cb_set_5_e(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.e = set(5, cpu.r.e);
    2
}

pub fn cb_set_5_h(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.h = set(5, cpu.r.h);
    2
}

pub fn cb_set_5_l(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.l = set(5, cpu.r.l);
    2
}

pub fn cb_set_5_hl(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let addr = cpu.r.hl();
    let v = mmu.rb(addr);
    let res = set(5, v);
    mmu.wb(addr, res);
    4
}

pub fn cb_set_5_a(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.a = set(5, cpu.r.a);
    2
}

pub fn cb_set_6_b(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.b = set(6, cpu.r.b);
    2
}

pub fn cb_set_6_c(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.c = set(6, cpu.r.c);
    2
}

pub fn cb_set_6_d(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.d = set(6, cpu.r.d);
    2
}

pub fn cb_set_6_e(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.e = set(6, cpu.r.e);
    2
}

pub fn cb_set_6_h(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.h = set(6, cpu.r.h);
    2
}

pub fn cb_set_6_l(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.l = set(6, cpu.r.l);
    2
}

pub fn cb_set_6_hl(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let addr = cpu.r.hl();
    let v = mmu.rb(addr);
    let res = set(6, v);
    mmu.wb(addr, res);
    4
}

pub fn cb_set_6_a(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.a = set(6, cpu.r.a);
    2
}

pub fn cb_set_7_b(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.b = set(7, cpu.r.b);
    2
}

pub fn cb_set_7_c(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.c = set(7, cpu.r.c);
    2
}

pub fn cb_set_7_d(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.d = set(7, cpu.r.d);
    2
}

pub fn cb_set_7_e(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.e = set(7, cpu.r.e);
    2
}

pub fn cb_set_7_h(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.h = set(7, cpu.r.h);
    2
}

pub fn cb_set_7_l(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.l = set(7, cpu.r.l);
    2
}

pub fn cb_set_7_hl(cpu: &mut CPU, mmu: &mut MMU) -> Cycles {
    let addr = cpu.r.hl();
    let v = mmu.rb(addr);
    let res = set(7, v);
    mmu.wb(addr, res);
    4
}

pub fn cb_set_7_a(cpu: &mut CPU, _: &mut MMU) -> Cycles {
    cpu.r.a = set(7, cpu.r.a);
    2
}

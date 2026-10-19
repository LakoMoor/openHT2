//! Chassis part of the per-frame vehicle step (`car2e.cpp` and neighbours).

use crate::globals::MAT_IDENTITY;
use crate::math::{vec_len, vec_normalize};
use crate::matrix::{vec_cross, vec_div_scalar, vec_dot, vec_scale};
use crate::mem::Mem;

const SCRATCH: u32 = 0x2fff_f100;

fn r(x: f64) -> f32 {
    x as f32
}

fn v3(m: &Mem, a: u32) -> [f64; 3] {
    [m.f32(a) as f64, m.f32(a + 4) as f64, m.f32(a + 8) as f64]
}

fn set3(m: &mut Mem, a: u32, v: [f32; 3]) {
    m.set_f32(a, v[0]);
    m.set_f32(a + 4, v[1]);
    m.set_f32(a + 8, v[2]);
}

fn copy3(m: &mut Mem, dst: u32, src: u32) {
    for i in 0..3 {
        let w = m.u32(src + 4 * i);
        m.set_u32(dst + 4 * i, w);
    }
}

/// `0x4efc10`: start of a step, once the contact normal `n` (`+0xd0`) is known: remove the part
/// of the velocity (`+0x40`) and of the angular velocity (`+0x4c`) that does not lie along / around
/// the normal, reset the contact state, and derive the axes (`+0xa2c`, `+0xa38`, `+0x9f0`, ...)
/// used by the wheel code.
pub fn fn_004efc10(m: &mut Mem, this: u32) {
    let n = v3(m, this + 0xd0);
    // velocity: keep only its component along the normal
    let vel = v3(m, this + 0x40);
    let d = vel[2] * n[2] + vel[1] * n[1] + vel[0] * n[0];
    set3(m, this + 0x40, [r(d * n[0]), r(d * n[1]), r(d * n[2])]);
    // angular velocity: n x w x n style projection, intermediates stay extended
    let w = v3(m, this + 0x4c);
    let a = w[2] * n[1] - n[2] * w[1];
    let b = n[2] * w[0] - w[2] * n[0];
    let c = w[1] * n[0] - w[0] * n[1];
    set3(m, this + 0x4c, [r(b * n[2] - c * n[1]), r(c * n[0] - a * n[2]), r(a * n[1] - b * n[0])]);
    for i in 0..3 {
        let w = m.u32(0x702650 + 4 * i);
        m.set_u32(this + 0xc24 + 4 * i, w);
    }
    m.set_f32(this + 0xdc0, r(vec_len(m, this + 0x40)));
    m.set_f32(this + 0xdc4, 0.0);
    for i in 0..m.i32(this + 0xdd4).max(0) as u32 {
        m.wr8(this + 0x19c4 + i, 0);
        m.set_i32(this + 0x19d4 + 4 * i, 0);
    }
    crate::carv::fn_004f2490(m, this);
    m.wr8(this + 0x19b4, 0);
    m.wr8(this + 0x19b5, 0);
    m.wr8(this + 0x19b7, 0);
    for i in 0..12 {
        let w = m.u32(MAT_IDENTITY + 4 * i);
        m.set_u32(this + 0x960 + 4 * i, w);
    }
    m.set_f32(this + 0xd7c, 10.0);

    // P = the vehicle axis picked by the constant vector at 0x702620 (stored at +0xa2c)
    let k = v3(m, 0x702620);
    let pick = |m: &Mem, base: u32| r(k[2] * m.f32(this + base + 0x18) as f64 + k[1] * m.f32(this + base + 0xc) as f64 + k[0] * m.f32(this + base) as f64);
    let p = [pick(m, 0x10), pick(m, 0x14), pick(m, 0x18)];
    set3(m, this + 0xa2c, p);
    let p = v3(m, this + 0xa2c);
    // Q = n x P
    let q = [r(n[1] * p[2] - p[1] * n[2]), r(p[0] * n[2] - n[0] * p[2]), r(p[1] * n[0] - n[1] * p[0])];
    let q64 = [q[0] as f64, q[1] as f64, q[2] as f64];
    // R = Q x n
    let rr = [r(q64[1] * n[2] - q64[2] * n[1]), r(q64[2] * n[0] - q64[0] * n[2]), r(q64[0] * n[1] - q64[1] * n[0])];
    set3(m, this + 0xa38, rr);
    vec_normalize(m, this + 0xa38);
    m.set_f32(this + 0xdc0, r(vec_len(m, this + 0x40)));
    // S = V x n, T = n x S
    let v = v3(m, this + 0x40);
    let s = [r(v[1] * n[2] - v[2] * n[1]), r(v[2] * n[0] - v[0] * n[2]), r(v[0] * n[1] - v[1] * n[0])];
    set3(m, SCRATCH, s);
    vec_cross(m, SCRATCH + 16, this + 0xd0, SCRATCH);
    copy3(m, this + 0x9f0, SCRATCH + 16);
    let len = vec_len(m, this + 0x9f0);
    m.set_f32(this + 0xdc4, len as f32);
    if len <= 0.01 {
        copy3(m, this + 0x9fc, this + 0xa38);
    } else {
        vec_div_scalar(m, SCRATCH + 32, this + 0x9f0, len as f32);
        copy3(m, this + 0x9fc, SCRATCH + 32);
    }
    let dot = vec_dot(m, this + 0x9f0, this + 0xa38);
    m.set_f32(this + 0xdc8, dot as f32);
    vec_scale(m, SCRATCH + 48, this + 0xa38, dot as f32);
    copy3(m, this + 0xa20, SCRATCH + 48);
    copy3(m, this + 0xd58, this + 0xd40);
}

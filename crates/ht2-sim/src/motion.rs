//! Derived motion state of `_Car_V` (`0x4f0530`, `0x4f0800`, `0x4f0ce0`).

use crate::globals::{VEC_Y, VEC_ZERO};
use crate::math;
use crate::mem::Mem;

fn r(x: f64) -> f32 {
    x as f32
}

/// `0x4f0800`: refresh the inverse of the body transform (`+0x10` into `+0x900`), the velocity in
/// body axes (`+0x58`), the world axes in body coordinates (`+0xa2c`) and the horizontal axis (`+0xa38`).
pub fn fn_004f0800(m: &mut Mem, this: u32) {
    math::mat_inverse(m, this + 0x900, this + 0x10);
    let g = |m: &Mem, o: u32| m.f32(this + o) as f64;
    let local = [
        r(g(m, 0x4c) * g(m, 0x900) + g(m, 0x918) * g(m, 0x54) + g(m, 0x90c) * g(m, 0x50)),
        r(g(m, 0x91c) * g(m, 0x54) + g(m, 0x904) * g(m, 0x4c) + g(m, 0x910) * g(m, 0x50)),
        r(g(m, 0x920) * g(m, 0x54) + g(m, 0x908) * g(m, 0x4c) + g(m, 0x914) * g(m, 0x50)),
    ];
    math::vec_div(m, this + 0x58, local, this + 0x1f48);
    let y = [m.f32(VEC_Y), m.f32(VEC_Y + 4), m.f32(VEC_Y + 8)].map(|v| v as f64);
    let t2 = r(y[2] * g(m, 0x2c));
    let t4 = r(y[1] * g(m, 0x20));
    let t3 = r(y[0] * g(m, 0x14));
    let t5 = r(y[2] * g(m, 0x30));
    let t7 = r(y[1] * g(m, 0x24));
    let t6 = r(y[0] * g(m, 0x18));
    m.set_f32(this + 0xa2c, r(y[0] * g(m, 0x10) + y[1] * g(m, 0x1c) + y[2] * g(m, 0x28)));
    m.set_f32(this + 0xa30, r(t3 as f64 + t4 as f64 + t2 as f64));
    m.set_f32(this + 0xa34, r(t6 as f64 + t7 as f64 + t5 as f64));
    let (a, b, c) = (g(m, 0xa2c), g(m, 0xa30), g(m, 0xa34));
    let (d0, d1, d2) = (g(m, 0xd0), g(m, 0xd4), g(m, 0xd8));
    let x2 = r(d1 * c - d2 * b);
    let x4 = r(a * d2 - c * d0);
    let x3 = r(b * d0 - a * d1);
    let out0 = r(x4 as f64 * d2 - x3 as f64 * d1);
    let out1 = r(x3 as f64 * d0 - x2 as f64 * d2);
    m.set_f32(this + 0xa38, out0);
    m.set_f32(this + 0xa3c, out1);
    let out2 = r(x2 as f64 * d1 - x4 as f64 * d0);
    m.set_f32(this + 0xa40, out2);
    math::vec_normalize(m, this + 0xa38);
}

/// `0x4f0530`: speed and heading bookkeeping from the velocity at `+0x40`.
pub fn fn_004f0530(m: &mut Mem, this: u32) {
    let len = math::vec_len(m, this + 0x40);
    if 5.0 < (m.f32(this + 0xdc0) as f64 - len).abs() || len < 1.0 {
        for k in 0..3 {
            let z = m.f32(VEC_ZERO + 4 * k);
            m.set_f32(this + 0xc90 + 4 * k, z);
            m.set_f32(this + 0xcd8 + 4 * k, z);
        }
    }
    m.set_f32(this + 0xdc0, len as f32);
    let g = |m: &Mem, o: u32| m.f32(this + o) as f64;
    let f3 = r(g(m, 0x44) * g(m, 0xd8) - g(m, 0x48) * g(m, 0xd4));
    let f5 = r(g(m, 0xd0) * g(m, 0x48) - g(m, 0xd8) * g(m, 0x40));
    let f4 = r(g(m, 0xd4) * g(m, 0x40) - g(m, 0xd0) * g(m, 0x44));
    m.set_f32(this + 0x9f0, r(f4 as f64 * g(m, 0xd4) - f5 as f64 * g(m, 0xd8)));
    m.set_f32(this + 0x9f4, r(f3 as f64 * g(m, 0xd8) - f4 as f64 * g(m, 0xd0)));
    m.set_f32(this + 0x9f8, r(f5 as f64 * g(m, 0xd0) - f3 as f64 * g(m, 0xd4)));
    let l2 = math::vec_len(m, this + 0x9f0);
    m.set_f32(this + 0xdc4, l2 as f32);
    let along = r(g(m, 0x9f0) * g(m, 0xa38) + g(m, 0x9f4) * g(m, 0xa3c) + g(m, 0x9f8) * g(m, 0xa40));
    m.set_f32(this + 0xdc8, along);
    m.set_f32(this + 0xa20, r(along as f64 * g(m, 0xa38)));
    m.set_f32(this + 0xa24, r(along as f64 * g(m, 0xa3c)));
    m.set_f32(this + 0xa28, r(along as f64 * g(m, 0xa40)));
    if 0.001f64 < l2 {
        m.set_f32(this + 0x9fc, r(g(m, 0x9f0) / l2));
        m.set_f32(this + 0xa00, r(g(m, 0x9f4) / l2));
        m.set_f32(this + 0xa04, r(g(m, 0x9f8) / l2));
    } else {
        for k in 0..3 {
            let v = m.u32(this + 0xa38 + 4 * k);
            m.set_u32(this + 0x9fc + 4 * k, v);
        }
    }
}

/// `0x4f0ce0`: refresh the derived motion state and the per-wheel contact summary flags.
pub fn fn_004f0ce0(m: &mut Mem, this: u32) {
    fn_004f0800(m, this);
    fn_004f0530(m, this);
    m.wr8(this + 0x19b4, 1);
    m.wr8(this + 0x19ba, 0);
    for i in 0..m.i32(this + 0xdd4).max(0) as u32 {
        if m.i16(this + 0x110 + 0xf8 * i) == 0 {
            let v = m.u32(this + 0xdd8 + 4 * i + 0x471 * 4);
            m.set_u32(this + 0xdd8 + 4 * i, v);
            m.set_i16(this + 0x19a4 + 2 * i, 1);
        }
        let c = m.i16(this + 0x19a4 + 2 * i) != 0;
        let all = m.rd8(this + 0x19b4) != 0 && c;
        m.wr8(this + 0x19b4, all as u8);
        let any = m.rd8(this + 0x19ba) != 0 || c;
        m.wr8(this + 0x19ba, any as u8);
    }
    if m.i32(this + 0xdd4) > 3 {
        let v = m.i16(this + 0x19a8) != 0 && m.i16(this + 0x19aa) != 0;
        m.wr8(this + 0x19b7, v as u8);
    }
    let front = m.i16(this + 0x19a4) != 0 && m.i16(this + 0x19a6) != 0;
    m.wr8(this + 0x19b5, front as u8);
}

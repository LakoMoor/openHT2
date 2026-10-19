//! Small helpers of `_Car_V` used by the per-frame step (`car*.cpp`, `0x4e28b0..0x4f2490`).

use crate::globals::VEC_UP;
use crate::mem::Mem;

/// `0x4e28b0`: 1 if any component of the vector at `v` lies outside `-limit..=limit`, else 0.
pub fn fn_004e28b0(m: &Mem, v: u32, limit: f32) -> u32 {
    let (x, y, z) = (m.f32(v), m.f32(v + 4), m.f32(v + 8));
    let out = limit < x || limit < y || limit < z || limit < -x || limit < -y || limit < -z;
    out as u32
}

/// `0x4f0000`: 1 if the vehicle is not upright: the up axis of its matrix points less than 0.17
/// upwards, or its second axis is steeper than 0.94.
pub fn fn_004f0000(m: &Mem, this: u32) -> u32 {
    let up = [m.f32(VEC_UP), m.f32(VEC_UP + 4), m.f32(VEC_UP + 8)];
    let dot = up[0] as f64 * m.f32(this + 0x18) as f64 + up[1] as f64 * m.f32(this + 0x24) as f64 + up[2] as f64 * m.f32(this + 0x30) as f64;
    if 0.17f64 as f32 as f64 <= dot && (m.f32(this + 0x24) as f64).abs() <= 0.94f64 as f32 as f64 {
        0
    } else {
        1
    }
}

/// `0x4f0090`: steering angle of the front wheels in radians from the steering position
/// (`+0x3cc8`: steps, `+0x3ccc`: number of steps, `+0x29e0`: analogue input).
pub fn fn_004f0090(m: &Mem, this: u32) -> f32 {
    const MAX: f64 = 0.785398185f32 as f64; // pi/4
    let steps = m.i32(this + 0x3cc8);
    if steps == 0 {
        return (MAX * m.f32(this + 0x29e0) as f64) as f32;
    }
    let abs = steps.wrapping_abs();
    if m.i32(this + 0x3ccc) - 1 <= abs {
        return if steps > 0 { (MAX * 0.5) as f32 } else { (-MAX * 0.5) as f32 };
    }
    let mut v = 0.0f64;
    if abs > 0 {
        v = if steps < 1 { -MAX } else { MAX };
        v *= 0.2;
    }
    v = MAX * m.f32(this + 0x29e0) as f64 + v;
    if MAX < v {
        v = MAX;
    }
    if v < -MAX {
        return (-MAX) as f32;
    }
    v as f32
}

/// `0x4f0450`: remember the state of the previous step (matrices, positions, per-wheel data).
pub fn fn_004f0450(m: &mut Mem, this: u32) {
    let copy = |m: &mut Mem, from: u32, to: u32, words: u32| {
        for i in 0..words {
            let v = m.u32(this + from + 4 * i);
            m.set_u32(this + to + 4 * i, v);
        }
    };
    copy(m, 0x10, 0x8d0, 12);
    copy(m, 0x900, 0x930, 12);
    copy(m, 0xf4, 0x1a10, 1);
    copy(m, 0x40, 0xa08, 3);
    copy(m, 0x4c, 0xc3c, 3);
    for i in 0..m.i32(this + 0xdd4).max(0) as u32 {
        copy(m, 0xf8 + 0xf8 * i, 0x1018 + 0xf8 * i, 0x3e);
        copy(m, 0x64 + 12 * i, 0x17e4 + 12 * i, 3);
        copy(m, 0xdd8 + 4 * i, 0xdf8 + 4 * i, 1);
    }
}

/// `0x4f2490`: combine the per-wheel contact flags (`+0x19c4 + i`) into axle and side flags.
pub fn fn_004f2490(m: &mut Mem, this: u32) {
    let n = m.i32(this + 0xdd4);
    let b = |m: &Mem, o: u32| m.rd8(this + o) != 0;
    m.wr8(this + 0x19bb, 0);
    m.wr8(this + 0x19bc, 1);
    for i in 0..n.max(0) as u32 {
        let c = b(m, 0x19c4 + i);
        let all = b(m, 0x19bc) && c;
        m.wr8(this + 0x19bc, all as u8);
        let any = b(m, 0x19bb) || c;
        m.wr8(this + 0x19bb, any as u8);
    }
    if n > 3 {
        let v = b(m, 0x19c4) && b(m, 0x19c6);
        m.wr8(this + 0x19c0, v as u8);
        let v = b(m, 0x19c5) && b(m, 0x19c7);
        m.wr8(this + 0x19c1, v as u8);
    }
    let v = b(m, 0x19c4) && b(m, 0x19c5);
    m.wr8(this + 0x19be, v as u8);
    if n > 3 {
        let v = b(m, 0x19c6) && b(m, 0x19c7);
        m.wr8(this + 0x19bf, v as u8);
    }
    let all = m.rd8(this + 0x19bc);
    m.wr8(this + 0x19bd, all);
    let front = (b(m, 0x19c4) || m.i16(this + 0x19a4) != 0) && (b(m, 0x19c5) || m.i16(this + 0x19a6) != 0);
    m.wr8(this + 0x19c2, front as u8);
    if n > 3 {
        let rear = (b(m, 0x19c6) || m.i16(this + 0x19a8) != 0) && (b(m, 0x19c7) || m.i16(this + 0x19aa) != 0);
        m.wr8(this + 0x19c3, rear as u8);
    }
    if front {
        m.set_i32(this + 0x19f4, 0);
    }
    if b(m, 0x19c3) {
        m.set_i32(this + 0x19f8, 0);
    }
}

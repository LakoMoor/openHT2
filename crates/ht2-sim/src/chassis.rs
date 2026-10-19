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

fn f(m: &Mem, a: u32) -> f64 {
    m.f32(a) as f64
}

/// `0x4f2670`: per-wheel geometry for the step. With `+0x2638 < 2` every wheel just gets the
/// given axis; otherwise the wheel contact velocity is projected into the wheel plane and the
/// slip direction, slip angle and tyre load limit are derived.
///
/// `p2`, `p3`, `p4`: transforms; `p5`, `p6`, `p8`: vectors; `p7`: scalar (all as in the original).
pub fn fn_004f2670(m: &mut Mem, this: u32, p2: u32, p3: u32, p4: u32, p5: u32, p6: u32, p7: f32, p8: u32) {
    let count = m.i32(this + 0xdd4);
    let n = v3(m, this + 0xd0);
    if m.i32(this + 0x2638) < 2 {
        let sgn: f64 = if p7 > 0.0 { 1.0 } else { -1.0 };
        for i in 0..count.max(0) as u32 {
            let (v3o, f4, w) = (12 * i, 4 * i, this + 0xf8 * i);
            copy3(m, this + 0x18a4 + v3o, p5);
            let a = sgn * p7 as f64;
            m.set_f32(this + 0x1964 + f4, r(a));
            m.set_f32(this + 0x1984 + f4, r(a * f(m, w + 0x10c)));
            let q = v3(m, p8);
            let s = [r(sgn * q[0]), r(sgn * q[1]), r(sgn * q[2])];
            set3(m, this + 0x1904 + v3o, s);
            set3(m, this + 0xa44 + v3o, s);
            copy3(m, this + 0xaa4 + v3o, p8);
            set3(m, this + 0xb64 + v3o, [r(n[1] * q[2] - n[2] * q[1]), r(n[2] * q[0] - q[2] * n[0]), r(q[1] * n[0] - n[1] * q[0])]);
            let s = [s[0] as f64, s[1] as f64, s[2] as f64];
            set3(m, this + 0xb04 + v3o, [r(s[2] * n[1] - n[2] * s[1]), r(n[2] * s[0] - s[2] * n[0]), r(s[1] * n[0] - s[0] * n[1])]);
            m.set_i32(this + 0x1a5c + f4, 0);
            m.set_f32(this + 0x1a7c + f4, 1.0);
            m.set_i32(this + 0x1a30 + f4, 0);
            m.set_f32(this + 0xd00 + f4, r(f(m, this + 0x2670 + f4) * f(m, this + 0xd20 + f4)));
        }
        return;
    }
    // l = (p6 rotated by p4) scaled component-wise by the vector at +0x1f54
    let t = SCRATCH + 0x100;
    crate::matrix::rotate(m, t, p6, p4);
    let l = [
        r(f(m, t) * f(m, this + 0x1f54)),
        r(f(m, t + 4) * f(m, this + 0x1f58)),
        r(f(m, t + 8) * f(m, this + 0x1f5c)),
    ];
    let a4: Vec<f64> = (0..9).map(|k| f(m, p4 + 4 * k)).collect();
    let a3: Vec<f64> = (0..9).map(|k| f(m, p3 + 4 * k)).collect();
    for i in 0..count.max(0) as u32 {
        let (v3o, f4) = (12 * i, 4 * i);
        let wb = this + 0x104 + 0xf8 * i;
        let w = v3(m, wb);
        let ux = a4[3] * w[1] + a4[0] * w[0] + a4[6] * w[2];
        let uy = r(a4[1] * w[0] + w[2] * a4[7] + w[1] * a4[4]) as f64;
        let uz = r(a4[5] * w[1] + a4[8] * w[2] + w[0] * a4[2]) as f64;
        let fl = r(ux * l[0] as f64 + uz * l[2] as f64 + uy * l[1] as f64) as f64;
        let (aa, bb, cc) = (ux * fl, uy * fl, uz * fl);
        let e = v3(m, this + 0x1844 + v3o);
        let d0 = r(bb * e[2] - cc * e[1]) as f64;
        let d1 = r(cc * e[0] - aa * e[2]) as f64;
        let d2 = aa * e[1] - bb * e[0];
        let c = [
            r(d2 * a3[6] + d1 * a3[3] + d0 * a3[0]) as f64,
            r(d2 * a3[7] + d1 * a3[4] + d0 * a3[1]) as f64,
            r(d0 * a3[2] + d2 * a3[8] + d1 * a3[5]) as f64,
        ];
        set3(m, this + 0x18a4 + v3o, [c[0] as f32, c[1] as f32, c[2] as f32]);
        // l1 = w x c, l2 = l1 x w, l3 = l2 + p5
        let l1 = [r(c[2] * w[1] - c[1] * w[2]) as f64, r(c[0] * w[2] - c[2] * w[0]) as f64, r(c[1] * w[0] - c[0] * w[1]) as f64];
        let l2 = [r(l1[1] * w[2] - l1[2] * w[1]) as f64, r(l1[2] * w[0] - l1[0] * w[2]) as f64, r(l1[0] * w[1] - l1[1] * w[0]) as f64];
        let p5v = v3(m, p5);
        set3(m, this + 0x18a4 + v3o, [r(l2[0] + p5v[0]), r(l2[1] + p5v[1]), r(l2[2] + p5v[2])]);
        let len = vec_len(m, this + 0x18a4 + v3o);
        let wz = f(m, wb + 8);
        m.set_f32(this + 0x1964 + f4, len as f32);
        m.set_f32(this + 0x1984 + f4, r(len * wz));
        let lenf = m.f32(this + 0x1964 + f4) as f64;
        if lenf <= 0.001 {
            copy3(m, this + 0x1904 + v3o, p8);
        } else {
            let c = v3(m, this + 0x18a4 + v3o);
            set3(m, this + 0x1904 + v3o, [r(c[0] / lenf), r(c[1] / lenf), r(c[2] / lenf)]);
        }
        // pv: the wheel direction
        let pv = this + 0xa44 + v3o;
        if m.i32(this + 0x1bd4 + f4) == 0 {
            copy3(m, pv, p8);
        } else if m.f32(this + 0x1cc0 + v3o) > 0.0 {
            let q = v3(m, p8);
            let b: Vec<f64> = (0..9).map(|k| f(m, p2 + 4 * k)).collect();
            set3(
                m,
                pv,
                [
                    r(b[6] * q[2] + b[3] * q[1] + q[0] * b[0]),
                    r(b[7] * q[2] + b[1] * q[0] + b[4] * q[1]),
                    r(b[8] * q[2] + b[2] * q[0] + b[5] * q[1]),
                ],
            );
        } else {
            let mat = SCRATCH + 0x400;
            let roll = -m.f32(this + 0x1a20);
            let dir = [m.f32(this + 0xd0), m.f32(this + 0xd4), m.f32(this + 0xd8)];
            let zero = [m.f32(0x702650), m.f32(0x702654), m.f32(0x702658)];
            crate::matrix::mat_look(m, mat, zero, dir, roll);
            crate::matrix::rotate(m, pv, p8, mat);
        }
        copy3(m, this + 0xaa4 + v3o, pv);
        let pp = v3(m, pv);
        set3(m, this + 0xb64 + v3o, [r(n[1] * pp[2] - pp[1] * n[2]), r(pp[0] * n[2] - n[0] * pp[2]), r(pp[1] * n[0] - n[1] * pp[0])]);
        if vec_dot(m, pv, this + 0x1904 + v3o) < 0.0 {
            crate::matrix::vec_neg(m, SCRATCH + 0x440, pv);
            copy3(m, pv, SCRATCH + 0x440);
        }
        vec_cross(m, SCRATCH + 0x450, this + 0xd0, pv);
        copy3(m, this + 0xb04 + v3o, SCRATCH + 0x450);
        vec_cross(m, SCRATCH + 0x460, this + 0x1904 + v3o, pv);
        m.set_f32(this + 0x1a5c + f4, vec_dot(m, SCRATCH + 0x460, this + 0xd0) as f32);
        m.set_f32(this + 0x1a7c + f4, vec_dot(m, this + 0x1904 + v3o, pv) as f32);
        if m.f32(this + 0x1a5c + f4) > 1.0 {
            m.set_f32(this + 0x1a5c + f4, 1.0);
        }
        if m.f32(this + 0x1a5c + f4) < -1.0 {
            m.set_f32(this + 0x1a5c + f4, -1.0);
        }
        if m.f32(this + 0x1a5c + f4) < 0.0 {
            crate::matrix::vec_neg(m, SCRATCH + 0x470, this + 0xb04 + v3o);
            copy3(m, this + 0xb04 + v3o, SCRATCH + 0x470);
        }
        // asin of the sine; the original's CRT routine works in extended precision
        let sn = m.f32(this + 0x1a5c + f4) as f64;
        m.set_f32(this + 0x1a30 + f4, sn.asin() as f32);
        m.set_f32(this + 0xd00 + f4, r(f(m, this + 0xd20 + f4) * f(m, this + 0x2670 + f4)));
    }
}

//! Vector and matrix helpers of the original math library (`0x4bd..`, `0x5ab..`, `0x5ac..`).
//!
//! Vectors are three consecutive `f32`; a transform is a 3x3 matrix (row-major) followed by a
//! translation, 12 `f32` in all.

use crate::mem::Mem;

fn r(x: f64) -> f32 {
    x as f32
}

/// `0x5ab4d0`: length of the vector at `v` (left in extended precision by the original).
pub fn vec_len(m: &Mem, v: u32) -> f64 {
    let (x, y, z) = (m.f32(v) as f64, m.f32(v + 4) as f64, m.f32(v + 8) as f64);
    (z * z + y * y + x * x).sqrt()
}

/// `0x5ab480`: normalise the vector at `v` unless it is shorter than 1e-5.
pub fn vec_normalize(m: &mut Mem, v: u32) {
    let (x, y, z) = (m.f32(v), m.f32(v + 4), m.f32(v + 8));
    let len = r(((z as f64) * z as f64 + (y as f64) * y as f64 + (x as f64) * x as f64).sqrt());
    if 1e-5f64 as f32 <= len {
        m.set_f32(v, r(x as f64 / len as f64));
        m.set_f32(v + 4, r(y as f64 / len as f64));
        m.set_f32(v + 8, r(z as f64 / len as f64));
    }
}

/// `0x4bdef0`: store `(x, y, z)` at `dst`.
pub fn vec_set(m: &mut Mem, dst: u32, x: f32, y: f32, z: f32) {
    m.set_f32(dst, x);
    m.set_f32(dst + 4, y);
    m.set_f32(dst + 8, z);
}

/// `0x4e3132`: `dst = a / b` component-wise (`a` by value, `b` at an address; returns `dst`).
pub fn vec_div(m: &mut Mem, dst: u32, a: [f32; 3], b: u32) -> u32 {
    let (x, y, z) = (
        r(a[0] as f64 / m.f32(b) as f64),
        r(a[1] as f64 / m.f32(b + 4) as f64),
        r(a[2] as f64 / m.f32(b + 8) as f64),
    );
    vec_set(m, dst, x, y, z);
    dst
}

/// `0x5ac2e0`: invert the transform at `src` into `dst` (3x3 inverse, translation `-t * inverse`).
pub fn mat_inverse(m: &mut Mem, dst: u32, src: u32) -> u32 {
    let p: Vec<f32> = (0..12).map(|i| m.f32(src + 4 * i)).collect();
    let q = |i: usize| p[i] as f64;
    let c0 = r(q(1) * q(5) - q(2) * q(4));
    let c1 = r(q(3) * q(2) - q(5) * q(0));
    let c2 = r(q(0) * q(4) - q(1) * q(3));
    let det = r(c1 as f64 * q(7) + c2 as f64 * q(8) + c0 as f64 * q(6));
    let d = det as f64;
    let mut o = [0.0f32; 12];
    o[0] = r((q(8) * q(4) - q(7) * q(5)) / d);
    o[1] = r((q(7) * q(2) - q(8) * q(1)) / d);
    o[2] = r(c0 as f64 / d);
    o[3] = r((q(6) * q(5) - q(8) * q(3)) / d);
    o[4] = r((q(8) * q(0) - q(6) * q(2)) / d);
    o[5] = r(c1 as f64 / d);
    o[6] = r((q(7) * q(3) - q(6) * q(4)) / d);
    o[7] = r((q(6) * q(1) - q(7) * q(0)) / d);
    o[8] = r(c2 as f64 / d);
    let (t0, t1, t2) = (r(-q(9)) as f64, r(-q(10)) as f64, r(-q(11)) as f64);
    o[9] = r(t0 * o[0] as f64 + t1 * o[3] as f64 + t2 * o[6] as f64);
    o[10] = r(t0 * o[1] as f64 + t1 * o[4] as f64 + t2 * o[7] as f64);
    o[11] = r(t0 * o[2] as f64 + t1 * o[5] as f64 + t2 * o[8] as f64);
    for (i, v) in o.iter().enumerate() {
        m.set_f32(dst + 4 * i as u32, *v);
    }
    dst
}

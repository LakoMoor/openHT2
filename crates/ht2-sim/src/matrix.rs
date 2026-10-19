//! Vector, rotation and transform helpers of the original math library (`0x4bd..`, `0x4cb..`,
//! `0x4e1..`, `0x5ab..`, `0x610f00`). Vectors are three consecutive `f32`; a transform is a 3x3
//! matrix (row-major, used as row vector times matrix) followed by a translation, 12 `f32`.
//!
//! Intermediate results are kept in `f64` and rounded where the original stores to memory (the
//! x87 unit keeps extended precision in between).

use crate::math::vec_set;
use crate::mem::Mem;

fn r(x: f64) -> f32 {
    x as f32
}

fn get3(m: &Mem, a: u32) -> [f64; 3] {
    [m.f32(a) as f64, m.f32(a + 4) as f64, m.f32(a + 8) as f64]
}

fn put3(m: &mut Mem, a: u32, v: [f64; 3]) {
    m.set_f32(a, r(v[0]));
    m.set_f32(a + 4, r(v[1]));
    m.set_f32(a + 8, r(v[2]));
}

fn get12(m: &Mem, a: u32) -> [f64; 12] {
    let mut o = [0.0; 12];
    for (i, v) in o.iter_mut().enumerate() {
        *v = m.f32(a + 4 * i as u32) as f64;
    }
    o
}

/// The identity transform at `0x7025f0`.
pub const IDENTITY: [f32; 12] = [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0];

pub fn put12(m: &mut Mem, a: u32, v: &[f32; 12]) {
    for (i, x) in v.iter().enumerate() {
        m.set_f32(a + 4 * i as u32, *x);
    }
}

/// `0x4bd4d0`: zero vector.
pub fn vec_zero(m: &mut Mem, v: u32) {
    vec_set(m, v, 0.0, 0.0, 0.0);
}

/// `0x610f00`: dot product (extended precision result).
pub fn vec_dot(m: &Mem, a: u32, b: u32) -> f64 {
    let (a, b) = (get3(m, a), get3(m, b));
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

/// `0x4bddb0`: `dst = a + b`.
pub fn vec_add(m: &mut Mem, dst: u32, a: u32, b: u32) {
    let (a, b) = (get3(m, a), get3(m, b));
    put3(m, dst, [a[0] + b[0], a[1] + b[1], a[2] + b[2]]);
}

/// `0x4bde00`: `dst = a - b`.
pub fn vec_sub(m: &mut Mem, dst: u32, a: u32, b: u32) {
    let (a, b) = (get3(m, a), get3(m, b));
    put3(m, dst, [a[0] - b[0], a[1] - b[1], a[2] - b[2]]);
}

/// `0x4bdd60`: `dst = v * s`.
pub fn vec_scale(m: &mut Mem, dst: u32, v: u32, s: f32) {
    let v = get3(m, v);
    let s = s as f64;
    put3(m, dst, [s * v[0], s * v[1], s * v[2]]);
}

/// `0x4e10d0`: `dst = v / s`.
pub fn vec_div_scalar(m: &mut Mem, dst: u32, v: u32, s: f32) {
    let v = get3(m, v);
    let s = s as f64;
    put3(m, dst, [v[0] / s, v[1] / s, v[2] / s]);
}

/// `0x4cb270`: `dst = -v`.
pub fn vec_neg(m: &mut Mem, dst: u32, v: u32) {
    let v = get3(m, v);
    put3(m, dst, [-v[0], -v[1], -v[2]]);
}

/// `0x4bde50`: `dst = a x b`, computed as the original does (`b.z*a.y - a.z*b.y`, ...).
pub fn vec_cross(m: &mut Mem, dst: u32, a: u32, b: u32) {
    let (a, b) = (get3(m, a), get3(m, b));
    put3(m, dst, [b[2] * a[1] - a[2] * b[1], a[2] * b[0] - b[2] * a[0], b[1] * a[0] - b[0] * a[1]]);
}

/// `0x5ab620` (thiscall): `v *= s` in place; the result is also copied to `out`.
pub fn vec_scale_in_place(m: &mut Mem, v: u32, out: u32, s: f32) {
    vec_scale(m, v, v, s);
    let x = get3(m, v);
    put3(m, out, x);
}

/// `0x5ab660` (thiscall): `v /= s`; copied to `out`.
pub fn vec_div_in_place(m: &mut Mem, v: u32, out: u32, s: f32) {
    vec_div_scalar(m, v, v, s);
    let x = get3(m, v);
    put3(m, out, x);
}

/// `0x5ab710` (thiscall): `v += w`; copied to `out`.
pub fn vec_add_in_place(m: &mut Mem, v: u32, out: u32, w: u32) {
    vec_add(m, v, w, v);
    let x = get3(m, v);
    put3(m, out, x);
}

/// `0x5ab750` (thiscall): `v -= w`; copied to `out`.
pub fn vec_sub_in_place(m: &mut Mem, v: u32, out: u32, w: u32) {
    vec_sub(m, v, v, w);
    let x = get3(m, v);
    put3(m, out, x);
}

/// `0x5ab400`: `dst = v / |v|`, or the zero vector at `0x702650` when `|v| < 1e-5`.
pub fn vec_unit(m: &mut Mem, dst: u32, v: u32) {
    let a = get3(m, v);
    let len = r((a[2] * a[2] + a[1] * a[1] + a[0] * a[0]).sqrt()) as f64;
    if len < 1e-5f64 as f32 as f64 {
        let z = get3(m, 0x702650);
        put3(m, dst, z);
    } else {
        put3(m, dst, [a[0] / len, a[1] / len, a[2] / len]);
    }
}

/// `0x5ab6a0`: are the two vectors closer than 1e-5?
pub fn vec_near(m: &Mem, a: u32, b: u32) -> bool {
    let (a, b) = (get3(m, a), get3(m, b));
    let d = [r(a[0] - b[0]) as f64, r(a[1] - b[1]) as f64, r(a[2] - b[2]) as f64];
    (d[2] * d[2] + d[1] * d[1] + d[0] * d[0]).sqrt().abs() < 1e-5
}

/// `0x4cb2b0`: rotate `v` by the 3x3 part of the matrix at `mat`.
pub fn rotate(m: &mut Mem, dst: u32, v: u32, mat: u32) {
    let (v, a) = (get3(m, v), get12(m, mat));
    put3(
        m,
        dst,
        [
            v[0] * a[0] + a[3] * v[1] + a[6] * v[2],
            a[1] * v[0] + a[4] * v[1] + a[7] * v[2],
            a[2] * v[0] + a[5] * v[1] + a[8] * v[2],
        ],
    );
}

/// `0x43f5b0`: transform the point `v` by the matrix at `mat` (rotate, then translate).
pub fn transform(m: &mut Mem, dst: u32, v: u32, mat: u32) {
    let (v, a) = (get3(m, v), get12(m, mat));
    put3(
        m,
        dst,
        [
            v[0] * a[0] + a[3] * v[1] + a[6] * v[2] + a[9],
            a[1] * v[0] + a[4] * v[1] + a[7] * v[2] + a[10],
            a[2] * v[0] + a[5] * v[1] + a[8] * v[2] + a[11],
        ],
    );
}

/// `0x5ab950` (thiscall): transform the point at `v` in place.
pub fn transform_in_place(m: &mut Mem, v: u32, mat: u32) {
    transform(m, v, v, mat);
}

/// `0x5ab9b0` (thiscall): rotate the vector at `v` in place.
pub fn rotate_in_place(m: &mut Mem, v: u32, mat: u32) {
    rotate(m, v, v, mat);
}

/// `0x5aba40`: `dst = a * b` for transforms (apply `a`, then `b`).
pub fn mat_mul(m: &mut Mem, dst: u32, a: u32, b: u32) {
    let (p, q) = (get12(m, a), get12(m, b));
    let mut o = [0.0f32; 12];
    for row in 0..3 {
        for col in 0..3 {
            o[row * 3 + col] = r(p[row * 3] * q[col] + p[row * 3 + 1] * q[3 + col] + p[row * 3 + 2] * q[6 + col]);
        }
    }
    let t = [p[9], p[10], p[11]];
    o[9] = r(t[0] * q[0] + q[3] * t[1] + q[6] * t[2] + q[9]);
    o[10] = r(q[1] * t[0] + q[4] * t[1] + q[7] * t[2] + q[10]);
    o[11] = r(q[2] * t[0] + q[5] * t[1] + q[8] * t[2] + q[11]);
    put12(m, dst, &o);
}

/// `0x5abbd0` / `0x5abc00` (thiscall): apply the transform `t` to the matrix at `mat`: the three
/// direction rows are rotated, the translation row is transformed.
pub fn mat_apply(m: &mut Mem, mat: u32, t: u32) {
    for i in 0..3 {
        rotate_in_place(m, mat + 12 * i, t);
    }
    transform_in_place(m, mat + 36, t);
}

/// `0x5abc40`: a translation by `(x, y, z)`.
pub fn mat_translation(m: &mut Mem, dst: u32, x: f32, y: f32, z: f32) {
    let mut o = [0.0f32; 12];
    for (i, v) in o.iter_mut().enumerate() {
        *v = m.f32(crate::globals::MAT_IDENTITY + 4 * i as u32);
    }
    o[9] = x;
    o[10] = y;
    o[11] = z;
    put12(m, dst, &o);
}

/// `0x5ac0c0`: rotation about Z.
pub fn mat_rot_z(m: &mut Mem, dst: u32, a: f32) {
    let (s, c) = (a as f64).sin_cos();
    let (s, c) = (r(s), r(c));
    put12(m, dst, &[c, s, 0.0, -s, c, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0]);
}

/// `0x5abfd0`: rotation about Y.
pub fn mat_rot_y(m: &mut Mem, dst: u32, a: f32) {
    let (s, c) = (a as f64).sin_cos();
    let (s, c) = (r(s), r(c));
    put12(m, dst, &[c, 0.0, -s, 0.0, 1.0, 0.0, s, 0.0, c, 0.0, 0.0, 0.0]);
}

/// `0x5abee0`: rotation about X.
pub fn mat_rot_x(m: &mut Mem, dst: u32, a: f32) {
    let (s, c) = (a as f64).sin_cos();
    let (s, c) = (r(s), r(c));
    put12(m, dst, &[1.0, 0.0, 0.0, 0.0, c, s, 0.0, -s, c, 0.0, 0.0, 0.0]);
}

/// Scratch area for [`dir_to_angles`].
const SCRATCH: u32 = 0x2fff_f000;

/// `0x5ab500`: direction `(x, y, z)` to two angles. Returns what the original stores through
/// its two out pointers, `(first, second)`: the azimuth (0 when the direction is vertical, mirrored
/// when `y < 0`) and the polar angle from the Z axis.
pub fn dir_to_angles(m: &mut Mem, x: f32, y: f32, z: f32) -> (f32, f32) {
    m.set_f32(SCRATCH, x);
    m.set_f32(SCRATCH + 4, y);
    m.set_f32(SCRATCH + 8, z);
    vec_unit(m, SCRATCH + 16, SCRATCH);
    let n = get3(m, SCRATCH + 16);
    let (nx, ny, nz) = (n[0], n[1], n[2].clamp(-1.0, 1.0));
    let polar = r(nz.acos());
    let mut azimuth = 0.0f32;
    if 0.001f64 as f32 <= polar && 0.001f64 as f32 <= 3.1415926f64 as f32 - polar {
        let s = (polar as f64).sin();
        let q = r(nx / s) as f64;
        azimuth = r(q.clamp(-1.0, 1.0).acos());
    }
    if ny < 0.0 {
        azimuth = 6.2831852f64 as f32 - azimuth;
    }
    (azimuth, polar)
}

/// `0x5abc90`: transform that places something at `pos` looking along `dir` with a roll about
/// its own axis: `inverse(M) * rotZ(roll) * M`, where `M` is built from the translation to `pos`
/// and the two angles of `dir`. The identity when `dir` is (nearly) zero. `dst` receives the 12 floats.
pub fn mat_look(m: &mut Mem, dst: u32, pos: [f32; 3], dir: [f32; 3], roll: f32) {
    const S: u32 = 0x2ffe_0000;
    let (a, c, b) = (S, S + 0x30, S + 0x60);
    let ident: [f32; 12] = core::array::from_fn(|i| m.f32(crate::globals::MAT_IDENTITY + 4 * i as u32));
    put12(m, a, &ident);
    put12(m, b, &ident);
    put12(m, c, &ident);
    let (d, u) = (S + 0x100, S + 0x110);
    m.set_f32(d, dir[0]);
    m.set_f32(d + 4, dir[1]);
    m.set_f32(d + 8, dir[2]);
    vec_unit(m, d, d);
    if !vec_near(m, d, 0x702650) {
        vec_unit(m, u, d);
        let (az, pol) = dir_to_angles(m, m.f32(u), m.f32(u + 4), m.f32(u + 8));
        mat_rot_z(m, c, roll);
        let (t, rz, ry, m2, m3) = (S + 0x200, S + 0x230, S + 0x260, S + 0x290, S + 0x2c0);
        mat_translation(m, t, pos[0], pos[1], pos[2]);
        mat_rot_z(m, rz, az);
        mat_rot_y(m, ry, pol);
        mat_mul(m, m2, ry, rz);
        mat_mul(m, m3, m2, t);
        let bm: [f32; 12] = core::array::from_fn(|i| m.f32(m3 + 4 * i as u32));
        put12(m, b, &bm);
        crate::math::mat_inverse(m, a, b);
        mat_apply(m, a, c);
        mat_apply(m, a, b);
    }
    let out: [f32; 12] = core::array::from_fn(|i| m.f32(a + 4 * i as u32));
    put12(m, dst, &out);
}

//! Rough contact response of a body (`0x5808c0`) and its solver `0x4e8ae1`, with the 4x4 double matrix helpers
//! (`0x50b480..0x50ba90`) and the transform scaling `0x4e3170`.
//!
//! The solver is compiled without optimisation: every intermediate sits in a frame slot as `f32`, while the x87 keeps intermediates
//! unrounded between stores (precision control 53 bits). The port keeps the frame in scratch memory with the original `[ebp - off]` offsets.

use crate::matrix::*;
use crate::mem::Mem;
use crate::moving_item::{vec_soft_limit, ItemVirtuals};

/// Stand-in for the debug log of the solver (`0x61cca9` formatting and `0x4e260c`).
pub trait RoughContactLog {
    fn log(&mut self);
}

/// Everything `0x5808c0` calls outside itself (stand-ins): the virtuals `+0x90`, `+0x94` of the body and the debug log.
pub trait RoughContactOwner: ItemVirtuals + RoughContactLog {}
impl<T: ItemVirtuals + RoughContactLog> RoughContactOwner for T {}

/// `0x50b480` (thiscall): store four doubles.
pub fn row_set(m: &mut Mem, this: u32, d: [f64; 4]) {
    for (i, v) in d.iter().enumerate() {
        m.set_f64(this + 8 * i as u32, *v);
    }
}

fn copy_words(m: &mut Mem, dst: u32, src: u32, n: u32) {
    for i in 0..n {
        let w = m.u32(src + 4 * i);
        m.set_u32(dst + 4 * i, w);
    }
}

/// `0x50b570` (thiscall): the 4x4 matrix at `this` from four rows.
pub fn mat4_from_rows(m: &mut Mem, this: u32, a: u32, b: u32, c: u32, d: u32) {
    for (i, r) in [a, b, c, d].iter().enumerate() {
        copy_words(m, this + 32 * i as u32, *r, 8);
    }
}

/// `0x5ac1b0` (thiscall): cofactor `(row, col)` of the 3x3 part (nine `f32`) at `this`, left unrounded in x87.
fn cofactor3(m: &Mem, this: u32, row: u32, col: u32) -> f64 {
    let a = |i: u32| m.f32(this + 4 * i) as f64;
    match col + row * 3 {
        0 => a(8) * a(4) - a(7) * a(5),
        1 => a(6) * a(5) - a(8) * a(3),
        2 => a(7) * a(3) - a(6) * a(4),
        3 => -(a(8) * a(1) - a(7) * a(2)),
        4 => -(a(6) * a(2) - a(8) * a(0)),
        5 => -(a(7) * a(0) - a(6) * a(1)),
        6 => a(5) * a(1) - a(4) * a(2),
        7 => a(3) * a(2) - a(5) * a(0),
        8 => a(4) * a(0) - a(3) * a(1),
        _ => f32::from_bits(0x3c23d70a) as f64,
    }
}

/// `0x5ac2a0` (thiscall): determinant of the 3x3 at `this`; the first two partial sums are stored as `f32` slots.
fn det3(m: &Mem, this: u32) -> f64 {
    let a = |i: u32| m.f32(this + 4 * i) as f64;
    let s0 = (cofactor3(m, this, 0, 2) * a(2)) as f32 as f64;
    let s1 = (cofactor3(m, this, 0, 1) * a(1) + s0) as f32 as f64;
    cofactor3(m, this, 0, 0) * a(0) + s1
}

fn sign(i: u32) -> f64 {
    if i & 1 == 0 { 1.0 } else { -1.0 }
}

/// `0x50b620` (fastcall): determinant of the 4x4 double matrix at `this`, expanded along the first row; the 3x3 minors are rounded to `f32`.
pub fn det4(m: &mut Mem, this: u32) -> f64 {
    const T: u32 = 0x7ffe_f000 + 0x33000;
    let mut acc = 0.0f64;
    for col in 0..4u32 {
        let cols: Vec<u32> = (0..4).filter(|c| *c != col).collect();
        for r in 0..3u32 {
            for (j, c) in cols.iter().enumerate() {
                let v = m.f64(this + 8 * ((r + 1) * 4 + c)) as f32;
                m.set_f32(T + 4 * (r * 3 + j as u32), v);
            }
        }
        let c0 = cofactor3(m, T, 0, 0);
        let c1 = cofactor3(m, T, 0, 1);
        let c2 = cofactor3(m, T, 0, 2);
        let a = |i: u32| m.f32(T + 4 * i) as f64;
        let d = (a(2) * c2 + a(1) * c1) + a(0) * c0;
        acc = (sign(col) * m.f64(this + 8 * col)) * d + acc;
    }
    acc
}

/// `0x50b900` (thiscall): cofactor `(row, col)` of the 4x4 double matrix at `this` (the minor is rounded to `f32`).
fn cofactor4(m: &mut Mem, this: u32, row: u32, col: u32) -> f64 {
    const T: u32 = 0x7ffe_f000 + 0x33100;
    let mut k = 0;
    for r in (0..4).filter(|r| *r != row) {
        let mut j = 0;
        for c in (0..4).filter(|c| *c != col) {
            let v = m.f64(this + 8 * (r * 4 + c)) as f32;
            m.set_f32(T + 4 * (k * 3 + j), v);
            j += 1;
        }
        k += 1;
    }
    let s = sign(row + col);
    det3(m, T) * s
}

/// `0x50b9b0(out, in)`: inverse of a 4x4 double matrix (a zero matrix when the determinant is 0 or NaN).
pub fn mat4_inverse(m: &mut Mem, out: u32, input: u32) {
    const C: u32 = 0x7ffe_f000 + 0x33200;
    const R: u32 = 0x7ffe_f000 + 0x33300;
    for i in 0..4u32 {
        for j in 0..4u32 {
            let v = m.f64(input + 8 * (i * 4 + j));
            m.set_f64(C + 8 * (j * 4 + i), v);
        }
    }
    for i in 0..32 {
        m.set_u32(R + 4 * i, 0);
    }
    let det = det4(m, input);
    if det == 0.0 || det.is_nan() {
        for i in 0..32 {
            let w = m.u32(R + 4 * i);
            m.set_u32(out + 4 * i, w);
        }
        return;
    }
    let inv = 1.0f64 / det;
    for i in 0..4u32 {
        for j in 0..4u32 {
            let c = cofactor4(m, C, i, j);
            m.set_f64(R + 8 * (i * 4 + j), c * inv);
        }
    }
    for i in 0..32 {
        let w = m.u32(R + 4 * i);
        m.set_u32(out + 4 * i, w);
    }
}

/// `0x50b810(out, row, b)`: `out[j] = sum_i row[i] * b[i][j]` (four doubles).
fn row_times_mat(m: &mut Mem, out: u32, row: u32, b: u32) {
    let mut r = [0.0f64; 4];
    for (j, o) in r.iter_mut().enumerate() {
        let mut acc = 0.0f64;
        for i in 0..4u32 {
            acc = m.f64(row + 8 * i) * m.f64(b + 32 * i + 8 * j as u32) + acc;
        }
        *o = acc;
    }
    row_set(m, out, r);
}

/// `0x50ba90(out, a, b)`: `out = a * b` for 4x4 double matrices.
pub fn mat4_mul(m: &mut Mem, out: u32, a: u32, b: u32) {
    const T: u32 = 0x7ffe_f000 + 0x33400;
    const R: u32 = 0x7ffe_f000 + 0x33500;
    for i in 0..32 {
        m.set_u32(R + 4 * i, 0);
    }
    for k in 0..4u32 {
        row_times_mat(m, T, a + 32 * k, b);
        copy_words(m, R + 32 * k, T, 8);
    }
    copy_words(m, out, R, 32);
}

/// `0x50b890(out, a, b)`: `out[j] = sum_i a[j][i] * b[i]` (matrix times four-vector).
pub fn mat4_vec(m: &mut Mem, out: u32, a: u32, b: u32) {
    let mut r = [0.0f64; 4];
    for (j, o) in r.iter_mut().enumerate() {
        let mut acc = 0.0f64;
        for i in 0..4u32 {
            acc = m.f64(a + 8 * (j as u32 * 4 + i)) * m.f64(b + 8 * i) + acc;
        }
        *o = acc;
    }
    row_set(m, out, r);
}

/// `0x4e30f4(dst, a, b)`: component-wise product.
fn vec_mul(m: &mut Mem, dst: u32, a: u32, b: u32) {
    for i in 0..3 {
        let v = m.f32(a + 4 * i) as f64 * m.f32(b + 4 * i) as f64;
        m.set_f32(dst + 4 * i, v as f32);
    }
}

/// `0x4e3170(dst, mat, v)`: the three direction rows of `mat` multiplied component-wise by `v`, translation = the zero vector `0x702650`.
fn mat_scale_rows(m: &mut Mem, dst: u32, mat: u32, v: u32) {
    const T: u32 = 0x7ffe_f000 + 0x33600;
    for k in 0..3 {
        vec_mul(m, T + 12 * k, mat + 12 * k, v);
    }
    copy_words(m, T + 36, 0x702650, 3);
    copy_words(m, dst, T, 12);
}

/// `0x4e8ae1(p1..p12)` (cdecl, frame `0x1354`): the solver of the rough contact. `p1` / `p2` are the body matrix and its inverse, `p3` / `p4` /
/// `p5` three vectors of the body, `p6` / `p7` the contact point and normal, `f8 / f9 / f10` floats; results in `p11` and `p12`.
#[allow(clippy::too_many_arguments)]
pub fn rough_contact_solve(m: &mut Mem, p: [u32; 7], f8: f32, f9: f32, f10: f32, p11: u32, p12: u32, lg: &mut dyn RoughContactLog) {
    const B: u32 = 0x7ffe_f000 + 0x12000 + 0x1400;
    let q = |o: i32| (B as i32 + o) as u32;
    let f = |m: &Mem, o: i32| m.f32((B as i32 + o) as u32) as f64;
    let st = |m: &mut Mem, o: i32, v: f64| m.set_f32((B as i32 + o) as u32, v as f32);
    let [p1, p2, p3, p4, p5, p6, p7] = p;
    let (d8, d9, d10) = (f8 as f64, f9 as f64, f10 as f64);
    vec_sub(m, q(-0x218), p6, p1 + 0x24);
    vec_unit(m, q(-0x10c), q(-0x218));
    vec_cross(m, q(-0x1218), p7, q(-0x10c));
    vec_unit(m, q(-0x2a8), q(-0x1218));
    vec_cross(m, q(-0x4c), q(-0x10c), q(-0x2a8));
    rotate(m, q(-0x1224), p4, p1);
    vec_cross(m, q(-0x1230), q(-0x1224), q(-0x218));
    vec_add(m, q(-0x4c0), p3, q(-0x1230));
    let len = crate::math::vec_len(m, q(-0x218));
    st(m, -0x324, len);
    if f(m, -0x324) == 0.0 || f(m, -0x324).is_nan() {
        copy_words(m, p11, p3, 3);
        copy_words(m, p12, p4, 3);
    }
    let dot = vec_dot(m, q(-0x2a8), q(-0x2a8));
    if dot == 0.0 || dot.is_nan() {
        vec_cross(m, q(-0x123c), q(-0x10c), 0x702640);
        vec_cross(m, q(-0x1248), q(-0x123c), q(-0x10c));
        vec_unit(m, q(-0x1254), q(-0x1248));
        copy_words(m, q(-0x2a8), q(-0x1254), 3);
        let dot = vec_dot(m, q(-0x2a8), q(-0x2a8));
        if dot == 0.0 || dot.is_nan() {
            vec_cross(m, q(-0x1260), q(-0x10c), 0x702620);
            vec_cross(m, q(-0x126c), q(-0x1260), q(-0x10c));
            vec_unit(m, q(-0x1278), q(-0x126c));
            copy_words(m, q(-0x2a8), q(-0x1278), 3);
        }
        vec_cross(m, q(-0x1284), q(-0x10c), q(-0x2a8));
        copy_words(m, q(-0x4c), q(-0x1284), 3);
    }
    let d = vec_dot(m, p3, q(-0x10c));
    st(m, -0x394, d);
    let v = (-d9) * f(m, -0x394);
    st(m, -0x4b4, v);
    if f(m, -0x394) < 0.0 || f(m, -0x394).is_nan() {
        let w = m.u32(q(-0x394));
        m.set_u32(q(-0x4b4), w);
    }
    copy_words(m, q(-0x1a8), p1, 12);
    copy_words(m, q(-0x20c), p2, 12);
    copy_words(m, q(-0x4b0), 0x7025f0, 12);
    copy_words(m, q(-0x184), 0x702650, 3);
    copy_words(m, q(-0x1e8), 0x702650, 3);
    copy_words(m, q(-0x4b0), q(-0x2a8), 3);
    copy_words(m, q(-0x4a4), q(-0x4c), 3);
    copy_words(m, q(-0x498), q(-0x10c), 3);
    crate::math::mat_inverse(m, q(-0x1d8), q(-0x4b0));
    mat_mul(m, q(-0x480), q(-0x1a8), q(-0x1d8));
    crate::math::mat_inverse(m, q(-0x178), q(-0x480));
    mat_scale_rows(m, q(-0x12b4), 0x7025f0, p5);
    mat_mul(m, q(-0x12e4), q(-0x20c), q(-0x12b4));
    mat_mul(m, q(-0x450), q(-0x12e4), q(-0x1a8));
    mat_scale_rows(m, q(-0x1314), 0x7025f0, p5);
    mat_mul(m, q(-0x1344), q(-0x178), q(-0x1314));
    mat_mul(m, q(-0x40), q(-0x1344), q(-0x480));
    crate::math::mat_inverse(m, q(-0x148), q(-0x40));
    vec_mul(m, q(-0x10), p5, p4);
    rotate(m, q(-0x360), q(-0x10), q(-0x480));
    rotate(m, q(-0x2d4), p3, q(-0x1d8));
    let z = m.f32(q(-0x4b4));
    crate::math::vec_set(m, q(-0x320), 0.0, 0.0, z);
    let x = f(m, -0x358) - ((d10 * d8) * (f(m, -0x2cc) - f(m, -0x318)));
    st(m, -0x100, x);
    if !(f(m, -0x100) * f(m, -0x358) > 0.0) {
        m.set_u32(q(-0x100), 0);
    }
    let z = m.f32(q(-0x100));
    crate::math::vec_set(m, q(-0x390), 0.0, 0.0, z);
    let l = f(m, -0x324);
    row_set(m, q(-0x2c8), [1.0, 0.0, 0.0, l * d8]);
    row_set(m, q(-0x314), [0.0, 1.0, (-l) * d8, 0.0]);
    row_set(m, q(-0x354), [l * f(m, -0x144), l * f(m, -0x138), 1.0, 0.0]);
    row_set(m, q(-0x384), [(-l) * f(m, -0x148), (-l) * f(m, -0x13c), 0.0, 1.0]);
    let qa = (l * f(m, -0x130)) * f(m, -0x100);
    let qb = ((-l) * f(m, -0x12c)) * f(m, -0x100);
    let qc = f(m, -0x35c) - ((l * d8) * f(m, -0x2d4));
    let qd = (l * d8) * f(m, -0x2d0) + f(m, -0x360);
    row_set(m, q(-0xfc), [qd, qc, qb, qa]);
    mat4_from_rows(m, q(-0xdc), q(-0x2c8), q(-0x314), q(-0x354), q(-0x384));
    mat4_inverse(m, q(-0x420), q(-0xdc));
    mat4_mul(m, q(-0x29c), q(-0xdc), q(-0x420));
    mat4_vec(m, q(-0x2f4), q(-0x420), q(-0xfc));
    for (k, o) in [-0x390i32, -0x38c, -0x320, -0x31c].iter().enumerate() {
        let v = m.f64(q(-0x2f4) + 8 * k as u32) as f32;
        m.set_f32(q(*o), v);
    }
    rotate(m, q(-0x5c), q(-0x390), q(-0x178));
    crate::math::vec_div(m, q(-0x334), [m.f32(q(-0x5c)), m.f32(q(-0x5c) + 4), m.f32(q(-0x5c) + 8)], p5);
    rotate(m, q(-0x118), q(-0x320), q(-0x4b0));
    rotate(m, q(-0x3a0), p4, p1);
    let a = vec_dot(m, p3, p3) * d8 / 2.0;
    st(m, -4, a);
    let a = vec_dot(m, q(-0x118), q(-0x118)) * d8 / 2.0;
    st(m, -0x50, a);
    let a = vec_dot(m, q(-0x10), p4) / 2.0;
    st(m, -0x1dc, a);
    let a = vec_dot(m, q(-0x5c), q(-0x334)) / 2.0;
    st(m, -0x21c, a);
    let a = f(m, -4) + f(m, -0x1dc);
    st(m, -0x328, a);
    let a = f(m, -0x50) + f(m, -0x21c);
    st(m, -0x364, a);
    if !(f(m, -0x328) == 0.0 || f(m, -0x328).is_nan()) {
        let a = f(m, -0x364) / f(m, -0x328);
        st(m, -0x4c4, a);
        if f(m, -0x4c4) > 1.0 {
            let a = f(m, -0x4c4) + f(m, -0x4c4);
            st(m, -0x4c4, a);
        }
    }
    if crate::carv::fn_004e28b0(m, p11, 400.0) != 0 || crate::carv::fn_004e28b0(m, q(-0x334), 200.0) != 0 {
        lg.log();
    }
    vec_soft_limit(m, q(-0x334));
    copy_words(m, p12, q(-0x334), 3);
    copy_words(m, p11, q(-0x118), 3);
}

/// `0x5808c0(body, a, b)` (cdecl, frame `0x9c`): the rough contact response of a body. The solver `0x4e8ae1` is run on the body matrix
/// (`+0xc64`) and its inverse (`0x5ac2e0`) with the vectors `+0x1a0`, `+0x1ac`, `+0xcd8`, the floats `[+0xcc4]`, `[+0x590]`, `0`; the result
/// `out30` goes to the virtual `+0x90(vec, 0x67d434)`, `out34` to `+0x94(vec, 0x67d420)`.
pub fn contact_response_rough(m: &mut Mem, body: u32, a: u32, b: u32, o: &mut dyn RoughContactOwner) {
    const S: u32 = 0x7ffe_f000 + 0x14000;
    let p = |off: u32| S + off;
    copy_words(m, p(0x48), body + 0xc64, 12);
    crate::math::mat_inverse(m, p(0x78), p(0x48));
    for off in [0xc, 0x10, 0x14] {
        m.set_u32(p(off), 0);
    }
    copy_words(m, p(0x30), body + 0x1a0, 3);
    copy_words(m, p(0x24), body + 0x1ac, 3);
    copy_words(m, p(0x3c), body + 0xcd8, 3);
    for off in [0x18, 0x1c, 0x20] {
        m.set_u32(p(off), 0);
    }
    let (f8, f9) = (m.f32(body + 0xcc4), m.f32(body + 0x590));
    rough_contact_solve(m, [p(0x48), p(0x78), p(0x30), p(0x24), p(0x3c), a, b], f8, f9, 0.0, p(0xc), p(0x18), o);
    o.virt(m, body, 0x90, &[p(0xc), 0x67d434]);
    o.virt(m, body, 0x94, &[p(0x18), 0x67d420]);
}

/// Stand-ins of `0x57e540`: besides the virtuals the report `0x4e2821(vec, text, limit)` of a vector (range check and log).
pub trait ImpulseOwner: ItemVirtuals {
    fn report(&mut self, m: &Mem, vec: u32, text: u32, limit: f32);
}

/// `0x57e540(body, a, b)` (cdecl, frame `0xc4`): friction response of a body against a contact. With the normal `n = [a+0x40]`, the velocity `W = +0x1a0`,
/// `V = +0x1ac`, `c = n x W`, `cn = c x n`, `u = unit(cn)`, `nW = n . W` and `t = |cn| - 0.1 * |-(nW * [+0x590]) - nW|`: when `t <= 0` (or NaN)
/// the velocity `V` is damped by `0.8 ^ (dt * 10)` and `t` is 0; otherwise `r` = `n x u` rotated by the inverse of the matrix `b` gives the part of `V`
/// along `r` (kept with `t / [+0xcd0]`) and the rest damped by `0.7 ^ (dt * 10)`. The impulse `u * t + n * (-(nW * [+0x590]))` goes to the
/// report (limit 150) and the virtual `+0x90(vec, 0x67d1e0)`, the new velocity to `+0x94(vec, 0x67d1d0)`. The frame is kept in memory (offsets
/// relative to `esp` after the prologue); the products of `f32` stay exact in `f64`, differences are rounded when stored.
pub fn contact_friction(m: &mut Mem, body: u32, a: u32, b: u32, o: &mut dyn ImpulseOwner) {
    use crate::matrix::*;
    const S: u32 = 0x7ffe_f000 + 0x16000;
    let p = |off: u32| S + off;
    let f = |m: &Mem, off: u32| m.f32(S + off) as f64;
    let copy = |m: &mut Mem, dst: u32, src: u32| {
        for i in 0..3 {
            let w = m.u32(src + 4 * i);
            m.set_u32(dst + 4 * i, w);
        }
    };
    copy(m, p(0x10), a + 0x40);
    copy(m, p(0x1c), body + 0x1a0);
    copy(m, p(0x50), body + 0x1ac);
    let cross = |m: &mut Mem, dst: u32, x: u32, y: u32| {
        // (x * y) with the component order of the compiled code: first y*z minus ..., stored as f32
        let g = |a: u32| m.f32(a) as f64;
        let v = [
            (g(x + 4) * g(y + 8) - g(x + 8) * g(y + 4)) as f32,
            (g(x + 8) * g(y) - g(x) * g(y + 8)) as f32,
            (g(x) * g(y + 4) - g(x + 4) * g(y)) as f32,
        ];
        crate::math::vec_set(m, dst, v[0], v[1], v[2]);
    };
    // c = n x W, cn = c x n
    cross(m, p(0x28), p(0x10), p(0x1c));
    cross(m, p(0x44), p(0x28), p(0x10));
    copy(m, p(0x38), p(0x44));
    vec_unit(m, p(0x70), p(0x38));
    copy(m, p(0x5c), p(0x50));
    copy(m, p(0x7c), p(0x1c));
    let dt = crate::globals::dt_hi(m);
    m.set_f32(p(0x6c), dt as f32);
    let nw = (f(m, 0x18) * f(m, 0x24) + f(m, 0x20) * f(m, 0x14)) + f(m, 0x10) * f(m, 0x1c);
    m.set_f32(p(0x34), nw as f32);
    let len = crate::math::vec_len(m, p(0x44));
    let kv = m.f32(body + 0x590) as f64;
    let mv = -(f(m, 0x34) * kv);
    m.set_f32(p(0x68), mv as f32);
    let t = len - ((mv - f(m, 0x34)).abs() * 0.1f32 as f64);
    m.set_f32(p(0x34), t as f32);
    let ex = f(m, 0x6c) * 10.0;
    if !(t > 0.0) {
        m.set_u32(p(0x34), 0);
        let pw = 0.8f64.powf(ex);
        let v = [f(m, 0x50) * pw, f(m, 0x54) * pw, f(m, 0x58) * pw];
        crate::math::vec_set(m, p(0x38), v[0] as f32, v[1] as f32, v[2] as f32);
        copy(m, p(0x5c), p(0x38));
    } else {
        crate::math::mat_inverse(m, p(0x98), b);
        let g = |m: &Mem, off: u32| m.f32(S + off) as f64;
        // tv = n x u
        let (ux, uy, uz) = (g(m, 0x70), g(m, 0x74), g(m, 0x78));
        let (nx, ny, nz) = (g(m, 0x10), g(m, 0x14), g(m, 0x18));
        crate::math::vec_set(m, p(0x28), (uz * ny - uy * nz) as f32, (nz * ux - uz * nx) as f32, (uy * nx - ny * ux) as f32);
        let (tx, ty, tz) = (g(m, 0x28), g(m, 0x2c), g(m, 0x30));
        let i = |m: &Mem, k: u32| m.f32(S + 0x98 + 4 * k) as f64;
        let rv = [
            ((tz * i(m, 6) + ty * i(m, 3)) + tx * i(m, 0)) as f32,
            ((tz * i(m, 7) + ty * i(m, 4)) + tx * i(m, 1)) as f32,
            ((tz * i(m, 8) + ty * i(m, 5)) + tx * i(m, 2)) as f32,
        ];
        crate::math::vec_set(m, p(0x1c), rv[0], rv[1], rv[2]);
        let (rx, ry, rz) = (g(m, 0x1c), g(m, 0x20), g(m, 0x24));
        let (vx, vy, vz) = (g(m, 0x50), g(m, 0x54), g(m, 0x58));
        let gg = (rz * vz + ry * vy) + rx * vx;
        crate::math::vec_set(m, p(0x38), (rx * gg) as f32, (ry * gg) as f32, (rz * gg) as f32);
        copy(m, p(0x44), p(0x38));
        vec_sub(m, p(0x8c), p(0x50), p(0x44));
        let kc = m.f32(body + 0xcd0) as f64;
        let s1 = (f(m, 0x34) / kc) as f32;
        m.set_f32(p(0x88), s1);
        let pw = 0.7f64.powf(ex);
        vec_scale(m, p(0x38), p(0x8c), pw as f32);
        vec_scale(m, p(0x28), p(0x1c), s1);
        vec_add(m, p(0xc8), p(0x28), p(0x38));
        copy(m, p(0x5c), p(0xc8));
    }
    let mb = m.f32(p(0x68));
    vec_scale(m, p(0xc4), p(0x10), mb);
    let tb = m.f32(p(0x34));
    vec_scale(m, p(0x8c), p(0x70), tb);
    vec_add(m, p(0x38), p(0x8c), p(0xc4));
    copy(m, p(0x7c), p(0x38));
    o.report(m, p(0x7c), 0x67d1f0, 150.0);
    o.virt(m, body, 0x90, &[p(0x7c), 0x67d1e0]);
    o.virt(m, body, 0x94, &[p(0x5c), 0x67d1d0]);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unhex(s: &str) -> Vec<u8> {
        (0..s.len() / 2).map(|i| u8::from_str_radix(&s[2 * i..2 * i + 2], 16).unwrap()).collect()
    }
    fn hx(m: &Mem, a: u32, n: usize) -> String {
        m.read(a, n).iter().map(|b| format!("{b:02x}")).collect()
    }

    struct RcTest {
        ev: Vec<String>,
    }
    impl ItemVirtuals for RcTest {
        fn virt(&mut self, m: &mut Mem, _this: u32, slot: u32, args: &[u32]) {
            let nm = if slot == 0x90 { "v90" } else { "v94" };
            self.ev.push(format!("{nm}|{}|{:x}", hx(m, args[0], 12), args[1]));
        }
    }
    impl RoughContactLog for RcTest {
        fn log(&mut self) {
            self.ev.push("log".to_string());
        }
    }

    #[test]
    fn native_contact_response_rough() {
        let (body, arg) = (0x2a010000u32, 0x2a013000u32);
        let mut n = 0;
        for (c, line) in include_str!("../../../tools/emu/golden_contact_8c0.tsv").lines().enumerate() {
            let v: Vec<&str> = line.split('\t').collect();
            let mut m = Mem::new();
            for it in v[0].split(',').filter(|s| !s.is_empty()) {
                let (o, x) = it.split_once(':').unwrap();
                m.write(u32::from_str_radix(o, 16).unwrap(), &unhex(x));
            }
            let (b0, a0) = (m.read(body, 0x1000), m.read(arg, 0x40));
            let mut ow = RcTest { ev: vec![] };
            contact_response_rough(&mut m, body, arg, arg + 0x10, &mut ow);
            let d = |a: &[u8], b: &[u8]| -> String {
                (0..a.len())
                    .step_by(4)
                    .filter(|&i| a[i..i + 4] != b[i..i + 4])
                    .map(|i| format!("{:x}:{}", i, b[i..i + 4].iter().map(|x| format!("{x:02x}")).collect::<String>()))
                    .collect::<Vec<_>>()
                    .join(",")
            };
            assert_eq!(d(&b0, &m.read(body, 0x1000)), v[1], "case {c}: body");
            assert_eq!(d(&a0, &m.read(arg, 0x40)), v[2], "case {c}: args");
            assert_eq!(ow.ev.join(";"), v[3], "case {c}: events");
            n += 1;
        }
        assert_eq!(n, 800);
    }

    struct FrTest {
        ev: Vec<String>,
    }
    impl ItemVirtuals for FrTest {
        fn virt(&mut self, m: &mut Mem, _this: u32, slot: u32, args: &[u32]) {
            let nm = if slot == 0x90 { "v90" } else { "v94" };
            self.ev.push(format!("{nm}|{}|{:x}", hx(m, args[0], 12), args[1]));
        }
    }
    impl ImpulseOwner for FrTest {
        fn report(&mut self, m: &Mem, vec: u32, text: u32, limit: f32) {
            self.ev.push(format!("rep|{}|{:x}|{:x}", hx(m, vec, 12), text, limit.to_bits()));
        }
    }

    #[test]
    fn native_contact_friction() {
        let (body, arg) = (0x2a010000u32, 0x2a013000u32);
        let mut n = 0;
        for (c, line) in include_str!("../../../tools/emu/golden_contact_e540.tsv").lines().enumerate() {
            let v: Vec<&str> = line.split('\t').collect();
            let mut m = Mem::new();
            for it in v[0].split(',').filter(|s| !s.is_empty()) {
                let (o, x) = it.split_once(':').unwrap();
                m.write(u32::from_str_radix(o, 16).unwrap(), &unhex(x));
            }
            let (b0, a0) = (m.read(body, 0x1000), m.read(arg, 0x200));
            let mut ow = FrTest { ev: vec![] };
            contact_friction(&mut m, body, arg, arg + 0x100, &mut ow);
            let d = |a: &[u8], b: &[u8]| -> String {
                (0..a.len())
                    .step_by(4)
                    .filter(|&i| a[i..i + 4] != b[i..i + 4])
                    .map(|i| format!("{:x}:{}", i, b[i..i + 4].iter().map(|x| format!("{x:02x}")).collect::<String>()))
                    .collect::<Vec<_>>()
                    .join(",")
            };
            assert_eq!(d(&b0, &m.read(body, 0x1000)), v[1], "case {c}: body");
            assert_eq!(d(&a0, &m.read(arg, 0x200)), v[2], "case {c}: args");
            assert_eq!(ow.ev.join(";"), v[3], "case {c}: events");
            n += 1;
        }
        assert_eq!(n, 800);
    }
}

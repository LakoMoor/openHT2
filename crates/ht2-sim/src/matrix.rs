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

/// `0x5f25c0`: resolve a transform node's local-to-world matrix. A parent
/// forces mode 2; the return value is inherited from the root's mode 1.
/// Invalid root modes preserve the supplied output before child composition.
pub fn node_world_transform(m:&mut Mem,node:u32,dst:u32)->bool {
    let mut chain=Vec::new();let mut current=node;
    loop {
        let parent=m.u32(current+0x2c);
        if parent==0 {break;}
        m.set_u32(current+0x38,2);chain.push(current);current=parent;
    }
    let mode=m.u32(current+0x38);
    if mode<=2 {let bytes=m.read(current+0x40,48);m.write(dst,&bytes);}
    for child in chain.into_iter().rev(){mat_mul(m,dst,child+0x40,dst);}
    mode==1
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
    if 0.001 <= polar as f64 && 0.001 <= 3.1415926 - polar as f64 {
        let s = (polar as f64).sin();
        let q = r(nx / s) as f64;
        azimuth = r(q.clamp(-1.0, 1.0).acos());
    }
    if ny < 0.0 {
        azimuth = (6.2831852 - azimuth as f64) as f32;
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

/// `0x5da950`: compose a transform from a position and three angles: `Ry(c) * Rx(-b) * Rz(a)`,
/// then translated by `(x, y, z)` (all arguments are doubles, narrowed to `f32`).
pub fn mat_from_euler(m: &mut Mem, dst: u32, x: f64, y: f64, z: f64, a: f64, b: f64, c: f64) {
    const S: u32 = 0x2ffd_0000;
    let (t, rz, rx, ry, m1, m2) = (S, S + 0x30, S + 0x60, S + 0x90, S + 0xc0, S + 0xf0);
    mat_translation(m, t, x as f32, y as f32, z as f32);
    mat_rot_z(m, rz, a as f32);
    mat_rot_x(m, rx, (-b) as f32);
    mat_rot_y(m, ry, c as f32);
    mat_mul(m, m1, ry, rx);
    mat_mul(m, m2, m1, rz);
    mat_mul(m, dst, m2, t);
}

/// `0x4e6362`: the point `p4` moved along the plane normal `p3` onto the plane through `p2`.
pub fn point_to_plane(m: &mut Mem, dst: u32, p2: u32, p3: u32, p4: u32) {
    const S: u32 = 0x2ffd_1000;
    vec_sub(m, S, p4, p2);
    let d = vec_dot(m, p3, S);
    vec_scale(m, S + 16, p3, d as f32);
    vec_sub(m, dst, p4, S + 16);
}

/// `0x4e63be`: where the line from `p4` towards `p5` meets the plane through `p2` with normal
/// `p3`; when `p4` and `p5` (nearly) coincide, `p4` projected onto the plane.
pub fn line_plane(m: &mut Mem, dst: u32, p2: u32, p3: u32, p4: u32, p5: u32) {
    const S: u32 = 0x2ffd_2000;
    vec_sub(m, S, p5, p4);
    let len = crate::math::vec_len(m, S) as f32;
    if len < 0.001f64 as f32 {
        point_to_plane(m, dst, p2, p3, p4);
        return;
    }
    vec_div_in_place(m, S, S + 0x10, len);
    vec_sub(m, S + 0x20, p2, p4);
    let num = vec_dot(m, p3, S + 0x20);
    let den = vec_dot(m, p3, S);
    let t = (num / den) as f32;
    vec_scale(m, S + 0x30, S, t);
    vec_add(m, dst, p4, S + 0x30);
}

#[cfg(test)]
mod hierarchy_tests {
    use super::*;
    #[test]
    fn original_node_world_chains() {
        for (case,line) in include_str!("../../../tools/emu/golden_node_transform.tsv").lines().enumerate(){
            let p:Vec<_>=line.split('\t').collect();let mut m=Mem::new();let base=0x2a000000;let out=base+0x10000;
            let nodes:Vec<_>=p[0].split(';').collect();
            for (i,node) in nodes.iter().enumerate(){
                let values:Vec<u32>=node.split(',').map(|v|v.parse().unwrap()).collect();let ptr=base+i as u32*0x100;
                m.set_u32(ptr+0x38,values[0]);m.set_u32(ptr+0x2c,if i+1<nodes.len(){ptr+0x100}else{0});
                for (k,v) in values[1..].iter().enumerate(){m.set_u32(ptr+0x40+k as u32*4,*v);}
            }
            for (i,v) in p[1].split(',').enumerate(){m.set_u32(out+i as u32*4,v.parse().unwrap());}
            assert_eq!(node_world_transform(&mut m,base,out),p[2]=="1","case {case}");
            assert_eq!((0..12).map(|i|m.u32(out+i*4)).collect::<Vec<_>>(),p[3].split(',').map(|v|v.parse::<u32>().unwrap()).collect::<Vec<_>>(),"case {case}");
            assert_eq!((0..nodes.len()).map(|i|m.u32(base+i as u32*0x100+0x38)).collect::<Vec<_>>(),p[4].split(',').map(|v|v.parse::<u32>().unwrap()).collect::<Vec<_>>(),"case {case}");
        }
    }
}

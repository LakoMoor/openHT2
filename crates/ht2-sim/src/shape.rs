//! Collision volumes of the scene and the segment test against them (`0x5f6820`).
//!
//! A volume record (`node + 0x40` of the collision blocks 12..17, 20, 23) is `{kind, count, data}`.
//! Kinds: 0 everything (block 13), 1 sphere (14), 2 inside of a sphere (15), 3 / 4 (16, 17),
//! 5 plane (12), 6 polygons (23), 7 a wall along a polyline (20).
//!
//! A polygon record (0x30 bytes, `0x5e69a0`): `+2` u16 vertex count, `+4` vertices, `+8` centre,
//! `+0x18` f64 radius around the centre, `+0x20` plane `(n, d)` through the first three vertices.
//!
//! The test moves `start` to the hit point and puts the surface normal into `end`. `mode` bit 1
//! (2) sweeps a sphere of radius `r` instead of a point.
//!
//! Ported: kinds 0, 5 and 6 for point queries (what the wheels and the vehicle placement use).
//! The sphere sweeps and kinds 1..4 and 7 are not ported yet and report no hit.

use crate::mem::Mem;

fn f(m: &Mem, a: u32) -> f64 {
    m.f32(a) as f64
}

fn r(x: f64) -> f32 {
    x as f32
}

fn v3(m: &Mem, a: u32) -> [f64; 3] {
    [f(m, a), f(m, a + 4), f(m, a + 8)]
}

fn set3(m: &mut Mem, a: u32, v: [f32; 3]) {
    m.set_f32(a, v[0]);
    m.set_f32(a + 4, v[1]);
    m.set_f32(a + 8, v[2]);
}

pub const VOL_ALL: u32 = 0;
pub const VOL_PLANE: u32 = 5;
pub const VOL_POLYS: u32 = 6;
pub const POLY_SIZE: u32 = 0x30;

/// `0x5ac6d0`: the plane `(n, d)` through `a`, `b`, `c` (`n = (b-a) x (c-a)`, unit length, or
/// `(0, -1, 0)` for a degenerate triangle; `d = -n.a`).
pub fn plane_from_points(a: [f32; 3], b: [f32; 3], c: [f32; 3]) -> [f32; 4] {
    let (a, b, c) = (a.map(|x| x as f64), b.map(|x| x as f64), c.map(|x| x as f64));
    let (f3, f1) = (r(c[0] - a[0]) as f64, r(c[1] - a[1]) as f64);
    let (p8, p9) = (r(b[0] - a[0]) as f64, r(b[1] - a[1]) as f64);
    let n = [
        r((c[2] - a[2]) * p9 - f1 * (b[2] - a[2])),
        r((b[2] - a[2]) * f3 - (c[2] - a[2]) * p8),
        r(f1 * p8 - p9 * f3),
    ];
    let len = ((n[2] as f64) * n[2] as f64 + (n[1] as f64) * n[1] as f64 + (n[0] as f64) * n[0] as f64).sqrt();
    let n = if 0.0001 <= len {
        let l = r(len) as f64;
        [r(n[0] as f64 / l), r(n[1] as f64 / l), r(n[2] as f64 / l)]
    } else {
        [0.0, -1.0, 0.0]
    };
    let d = r(a[0] * -(n[0] as f64) + -(n[2] as f64) * a[2] + a[1] * -(n[1] as f64));
    [n[0], n[1], n[2], d]
}

/// The derived part of `0x5e69a0`: plane, centre and radius of the polygon record at `poly`
/// whose `count` vertices are at `verts`.
pub fn poly_init(m: &mut Mem, poly: u32, verts: u32, count: u16) {
    m.set_u16(poly + 2, count);
    m.set_u32(poly + 4, verts);
    let v = |m: &Mem, i: u32| [m.f32(verts + 12 * i), m.f32(verts + 12 * i + 4), m.f32(verts + 12 * i + 8)];
    let pl = plane_from_points(v(m, 0), v(m, 1), v(m, 2));
    for (i, x) in pl.iter().enumerate() {
        m.set_f32(poly + 0x20 + 4 * i as u32, *x);
    }
    let mut c = [0.0f32; 3];
    for i in 0..count as u32 {
        let p = v(m, i);
        c = [r(c[0] as f64 + p[0] as f64), r(c[1] as f64 + p[1] as f64), r(c[2] as f64 + p[2] as f64)];
    }
    let n = count as f32 as f64;
    let c = [r(c[0] as f64 / n), r(c[1] as f64 / n), r(c[2] as f64 / n)];
    set3(m, poly + 8, c);
    m.set_u32(poly + 0x18, 0);
    m.set_u32(poly + 0x1c, 0);
    let mut radius = 0.0f64;
    for i in 0..count as u32 {
        let p = v(m, i);
        let d = [r(c[0] as f64 - p[0] as f64) as f64, r(c[1] as f64 - p[1] as f64) as f64, r(c[2] as f64 - p[2] as f64) as f64];
        let len = (d[2] * d[2] + d[1] * d[1] + d[0] * d[0]).sqrt();
        if radius < len {
            radius = len;
        }
    }
    m.write(poly + 0x18, &radius.to_le_bytes());
}

/// `0x5f5370`: the segment `start`..`end` against one polygon (point queries only).
pub fn fn_005f5370(m: &mut Mem, mode: u32, poly: u32, start: u32, end: u32, _r: f32) -> u32 {
    let s = v3(m, start);
    let e = v3(m, end);
    let d = [r(e[0] - s[0]) as f64, r(e[1] - s[1]) as f64, r(e[2] - s[2]) as f64];
    let n = v3(m, poly + 0x20);
    let pd = f(m, poly + 0x2c);
    if 1e-5 < d[2] * n[2] + d[1] * n[1] + d[0] * n[0] {
        return 0;
    }
    let sn = s[2] * n[2] + s[1] * n[1] + s[0] * n[0];
    if mode & 2 != 0 {
        return 0; // the sphere sweep is not ported yet
    }
    if sn + pd < -1e-5 {
        return 0;
    }
    if 1e-5 < e[0] * n[0] + e[2] * n[2] + e[1] * n[1] + pd {
        return 0;
    }
    let k = d[2] * n[2] + d[1] * n[1] + d[0] * n[0];
    let t = if k.abs() < 0.0001 { 0.5 } else { -(n[2] * s[2] + n[1] * s[1] + n[0] * s[0] + pd) / k };
    if 1.00001 < t || t < -1e-5 {
        return 0;
    }
    let p = [r(d[0] * t + s[0]), r(d[1] * t + s[1]), r(d[2] * t + s[2])];
    let pv = [p[0] as f64, p[1] as f64, p[2] as f64];
    let c = v3(m, poly + 8);
    let (dz, dy, dx) = (pv[2] - c[2], pv[1] - c[1], pv[0] - c[0]);
    let dist = (dx * dx + dy * dy + dz * dz).sqrt();
    if m.f64(poly + 0x18) < dist {
        return 0;
    }
    let count = m.u16(poly + 2) as u32;
    let verts = m.u32(poly + 4);
    let rel = |m: &Mem, i: u32| {
        let q = v3(m, verts + 12 * i);
        [r(q[0] - pv[0]) as f64, r(q[1] - pv[1]) as f64, r(q[2] - pv[2]) as f64]
    };
    let a0 = {
        let q = v3(m, verts);
        [q[0] - pv[0], q[1] - pv[1], q[2] - pv[2]]
    };
    let l = rel(m, count - 1);
    let refv = [r(a0[2] * l[1] - a0[1] * l[2]) as f64, r(l[2] * a0[0] - a0[2] * l[0]) as f64, r(a0[1] * l[0] - l[1] * a0[0]) as f64];
    for i in 1..count {
        let a = rel(m, i);
        let b = rel(m, i - 1);
        // 0x4bde50: c = b x a
        let c = [r(a[2] * b[1] - b[2] * a[1]) as f64, r(b[2] * a[0] - a[2] * b[0]) as f64, r(a[1] * b[0] - a[0] * b[1]) as f64];
        if refv[0] * c[0] + refv[2] * c[2] + refv[1] * c[1] < -1e-5 {
            return 0;
        }
    }
    set3(m, start, p);
    for i in 0..3 {
        let w = m.u32(poly + 0x20 + 4 * i);
        m.set_u32(end + 4 * i, w);
    }
    1
}

/// `0x5f6820`: the segment `start`..`end` against the volume record `vol`. Returns 0 for no hit,
/// otherwise 1 (polygons: the number of the polygon hit, counting from 1).
pub fn fn_005f6820(m: &mut Mem, vol: u32, mode: u32, start: u32, end: u32, rad: f32) -> u32 {
    match m.u32(vol) {
        VOL_ALL => {
            let (s, e) = (v3(m, start), v3(m, end));
            let d = [r(s[0] - e[0]), r(s[1] - e[1]), r(s[2] - e[2])];
            let tmp = 0x2ff6_0000;
            set3(m, tmp, d);
            crate::matrix::vec_unit(m, end, tmp);
            1
        }
        VOL_PLANE => {
            let pl = m.u32(vol + 8);
            let (s, e) = (v3(m, start), v3(m, end));
            let n = v3(m, pl);
            let d = [r(e[0] - s[0]) as f64, r(e[1] - s[1]) as f64, r(e[2] - s[2]) as f64];
            let dn = d[0] * n[0] + d[1] * n[1] + d[2] * n[2];
            if dn > 0.0 {
                return 0;
            }
            let t = if 0.0001 <= dn.abs() {
                let rr = if mode & 2 == 0 { 0.0 } else { rad as f64 };
                let sn = s[0] * n[0] + s[1] * n[1] + s[2] * n[2];
                let t = -(((f(m, pl + 12) - rr) + sn) / dn);
                if !(0.0 < t && (t as f32) < 1.0) {
                    return 0;
                }
                t as f32 as f64
            } else {
                let en = e[0] * n[0] + e[1] * n[1] + e[2] * n[2];
                if (en + f(m, pl + 12)).abs() > 0.01 {
                    return 0;
                }
                0.5
            };
            let p = [r(d[0] * t + s[0]), r(d[1] * t + s[1]), r(d[2] * t + s[2])];
            let rr = if mode & 2 == 0 { 0.0 } else { rad as f64 };
            let p = [r(p[0] as f64 - n[0] * rr), r(p[1] as f64 - n[1] * rr), r(p[2] as f64 - n[2] * rr)];
            set3(m, start, p);
            for i in 0..3 {
                let w = m.u32(pl + 4 * i);
                m.set_u32(end + 4 * i, w);
            }
            1
        }
        VOL_POLYS => {
            let (n, data) = (m.i32(vol + 4), m.u32(vol + 8));
            for i in 0..n.max(0) as u32 {
                if fn_005f5370(m, mode, data + POLY_SIZE * i, start, end, rad) != 0 {
                    return i + 1;
                }
            }
            0
        }
        _ => 0,
    }
}

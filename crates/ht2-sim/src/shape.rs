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
//! Ported: kinds 0, 5, 6 and 7 for point queries (what the wheels and the vehicle placement use).
//! The sphere sweeps and kinds 1..4 are not ported yet and report no hit.

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
pub const VOL_WALL: u32 = 7;
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
pub fn fn_005f5370(m: &mut Mem, mode: u32, poly: u32, start: u32, end: u32, rad: f32) -> u32 {
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
        return poly_sphere(m, poly, start, end, rad as f64, s, e, d, n, pd, sn);
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

/// The sphere branch of `0x5f5370`: a sphere of radius `rad` swept from `s` to `e` touching the
/// polygon's face: the centre where it touches, moved back onto the plane, must lie inside.
#[allow(clippy::too_many_arguments)]
fn poly_sphere(m: &mut Mem, poly: u32, start: u32, end: u32, rad: f64, s: [f64; 3], e: [f64; 3], d: [f64; 3], n: [f64; 3], pd: f64, sn: f64) -> u32 {
    if pd + sn < -1e-5 {
        return 0;
    }
    if 1e-5 < e[0] * n[0] + n[2] * e[2] + n[1] * e[1] + pd - rad {
        return 0;
    }
    let mut k = d[2] * n[2] + d[1] * n[1] + d[0] * n[0];
    if k.abs() < 1e-5 {
        k = if k <= 0.0 { -1e-5 } else { 1e-5 };
    }
    let t = r(-(((pd - rad) + sn) / k)) as f64;
    if t < 0.0 || 1.0 < t {
        return 0;
    }
    let hit = [r(d[0] * t + s[0]) as f64, r(r(d[1] * t) as f64 + s[1]) as f64, r(r(d[2] * t) as f64 + s[2]) as f64];
    let nr = [r(n[0] * rad) as f64, r(n[1] * rad) as f64, r(n[2] * rad) as f64];
    let hit = [r(hit[0] - nr[0]) as f64, r(hit[1] - nr[1]) as f64, r(hit[2] - nr[2]) as f64];
    let count = m.u16(poly + 2) as u32;
    let verts = m.u32(poly + 4);
    let l = {
        let q = v3(m, verts + 12 * (count - 1));
        [q[0] - hit[0], q[1] - hit[1], q[2] - hit[2]]
    };
    let rel = |m: &Mem, i: u32| {
        let q = v3(m, verts + 12 * i);
        [r(q[0] - hit[0]) as f64, r(q[1] - hit[1]) as f64, r(q[2] - hit[2]) as f64]
    };
    let a0 = rel(m, 0);
    let refv = [r(l[1] * a0[2] - l[2] * a0[1]) as f64, r(l[2] * a0[0] - a0[2] * l[0]) as f64, r(a0[1] * l[0] - l[1] * a0[0]) as f64];
    let mut prev = a0;
    for i in 1..count {
        let c = rel(m, i);
        let x = [c[2] * prev[1] - c[1] * prev[2], prev[2] * c[0] - c[2] * prev[0], c[1] * prev[0] - prev[1] * c[0]];
        if x[2] * refv[2] + x[1] * refv[1] + x[0] * refv[0] < 0.0 {
            return 0;
        }
        prev = c;
    }
    for i in 0..3 {
        let w = m.u32(poly + 0x20 + 4 * i);
        m.set_u32(end + 4 * i, w);
    }
    set3(m, start, [r(hit[0]), r(hit[1]), r(hit[2])]);
    1
}

fn unit3(v: [f64; 3]) -> [f64; 3] {
    let len = r((v[2] * v[2] + v[1] * v[1] + v[0] * v[0]).sqrt()) as f64;
    if len < 1e-5f64 as f32 as f64 {
        [0.0; 3]
    } else {
        [r(v[0] / len) as f64, r(v[1] / len) as f64, r(v[2] / len) as f64]
    }
}

fn sub3(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [r(a[0] - b[0]) as f64, r(a[1] - b[1]) as f64, r(a[2] - b[2]) as f64]
}

/// `0x5ac810`: where the line from `a` to `b` crosses the plane `(n, d)`; `None` when parallel.
fn line_plane(n: [f64; 3], d: f64, a: [f64; 3], b: [f64; 3]) -> Option<[f64; 3]> {
    let u = unit3(sub3(b, a));
    let k = (n[2] * u[2] + n[0] * u[0]) + u[1] * n[1];
    if k.abs() < 1e-5 {
        return None;
    }
    let t = -((((n[2] * a[2] + n[0] * a[0]) + a[1] * n[1]) + d) / k);
    Some([r(r(u[0] * t) as f64 + a[0]) as f64, r(r(u[1] * t) as f64 + a[1]) as f64, r(u[2] * t + a[2]) as f64])
}

fn length3(v: [f64; 3]) -> f64 {
    (v[2] * v[2] + v[1] * v[1] + v[0] * v[0]).sqrt()
}

/// The sphere branch of `0x5f5a80` (all points flat, `n` the wall normal, `d` its offset).
#[allow(clippy::too_many_arguments)]
fn wall_sphere(m: &mut Mem, start: u32, end: u32, rad: f64, s0: [f64; 3], e0: [f64; 3], s: [f64; 3], e: [f64; 3], pa: [f64; 3], pb: [f64; 3], n: [f64; 3], d: f64, dir: [f64; 3]) -> u32 {
    let dot = |u: [f64; 3], v: [f64; 3]| u[1] * v[1] + u[2] * v[2] + u[0] * v[0];
    let dr = r(d - rad) as f64;
    if 1e-5 < dot(e, n) + dr {
        return 0;
    }
    let dn = dir[1] * n[1] + dir[2] * n[2] + dir[0] * n[0];
    let (contact, t) = if 0.0001 <= dn.abs() {
        let t = r(-((s[1] * n[1] + n[2] * s[2] + n[0] * s[0] + dr) / dn)) as f64;
        if 1.0 <= t {
            return 0;
        }
        let dt = [r(dir[0] * t) as f64, r(dir[1] * t) as f64, r(dir[2] * t) as f64];
        let hit = [r(dt[0] + s[0]) as f64, r(dt[1] + s[1]) as f64, r(dt[2] + s[2]) as f64];
        let nr = [r(n[0] * rad) as f64, r(n[1] * rad) as f64, r(n[2] * rad) as f64];
        (sub3(hit, nr), t)
    } else {
        let k = dot(e, n) + dr + rad;
        let p = [r(n[0] * k) as f64, r(n[1] * k) as f64, r(n[2] * k) as f64];
        (sub3(e, p), 1.0)
    };
    let span = r(length3(sub3(pb, pa)) + rad) as f64;
    if length3(sub3(contact, pa)) < span && length3(sub3(contact, pb)) < span {
        set3(m, end, [r(n[0]), r(n[1]), r(n[2])]);
        set3(m, start, [r(contact[0]), r(contact[1]), r((e0[2] - s0[2]) * t + s0[2])]);
        return 1;
    }
    // The ends of the wall: the foot of each on the line of the move.
    let back = unit3(sub3(s, e));
    let travel = length3(sub3(s, e));
    let ahead = dot(e, n) + dr + rad;
    for corner in [pa, pb] {
        let cd = r(-(back[1] * corner[1] + back[2] * corner[2] + back[0] * corner[0])) as f64;
        let Some(foot) = line_plane(back, cd, s, e) else { continue };
        if !(length3(sub3(foot, corner)) < rad) {
            continue;
        }
        if r(length3(sub3(foot, s))) as f64 >= travel {
            continue;
        }
        if r(length3(sub3(foot, e))) as f64 >= travel || !(0.0 < ahead) {
            continue;
        }
        set3(m, end, [r(n[0]), r(n[1]), r(n[2])]);
        set3(m, start, [r(corner[0]), r(corner[1]), 0.0]);
        return 1;
    }
    0
}

/// `0x5f4b30`: a sphere of radius `rad` swept from `start` to `end` against the edges, then the
/// corners, of a polygon it is moving into: the first edge whose line it touches within the
/// edge, else the first corner it touches. `start` becomes the centre where it touches less
/// `rad` along the normal, `end` the normal.
pub fn fn_005f4b30(m: &mut Mem, poly: u32, start: u32, end: u32, rad: f32) -> u32 {
    let (s, e) = (v3(m, start), v3(m, end));
    let d = sub3(e, s);
    let n = v3(m, poly + 0x20);
    let pd = f(m, poly + 0x2c);
    let rad = rad as f64;
    if 1e-5 < (d[2] * n[2] + d[1] * n[1]) + d[0] * n[0] {
        return 0;
    }
    if (n[2] * s[2] + n[1] * s[1]) + s[0] * n[0] + pd < -1e-5 {
        return 0;
    }
    if 1e-5 < ((e[0] * n[0] + e[2] * n[2]) + e[1] * n[1]) + pd - rad {
        return 0;
    }
    let len = (d[2] * d[2] + d[1] * d[1] + d[0] * d[0]).sqrt();
    let u = unit3(d);
    let count = m.u16(poly + 2) as u32;
    let verts = m.u32(poly + 4);
    let vert = |m: &Mem, i: u32| v3(m, verts + 12 * i);
    let r2 = rad * rad;
    let finish = |m: &mut Mem, hit: [f64; 3], nrm: [f64; 3]| {
        set3(m, end, [r(nrm[0]), r(nrm[1]), r(nrm[2])]);
        let nr = [r(rad * nrm[0]) as f64, r(rad * nrm[1]) as f64, r(rad * nrm[2]) as f64];
        let p = sub3(hit, nr);
        set3(m, start, [r(p[0]), r(p[1]), r(p[2])]);
    };
    for k in 0..count {
        let next = if k + 1 < count { k + 1 } else { 0 };
        let (vk, vn) = (vert(m, k), vert(m, next));
        let w = unit3(sub3(vn, vk));
        let a = (w[0] * u[0] + w[2] * u[2]) + w[1] * u[1];
        let aa = ((u[0] * u[0] + u[1] * u[1]) + u[2] * u[2]) - a * a;
        let qx = s[0] - vk[0];
        let (qy, qz) = (r(s[1] - vk[1]) as f64, r(s[2] - vk[2]) as f64);
        let b1 = (qx * w[0] + qz * w[2]) + qy * w[1];
        let b2 = (qx * u[0] + qz * u[2]) + qy * u[1];
        let bd = (b2 + b2) - (b1 * a + b1 * a);
        let q = (qy * qy + qz * qz) + qx * qx;
        let c4 = ((q - b1 * b1) - r2) * aa * 4.0;
        let bb = bd * bd;
        if bb <= c4 {
            continue;
        }
        let sq = (bb - c4).sqrt();
        let a2 = aa + aa;
        let t1 = (-bd - sq) / a2;
        let t2 = (sq - bd) / a2;
        let t = if t2 < t1 { t2 } else { t1 };
        if t < 0.0 || len < t {
            continue;
        }
        let off = [r(u[0] * t) as f64, r(u[1] * t) as f64, r(u[2] * t) as f64];
        let hit = [r(off[0] + s[0]) as f64, r(off[1] + s[1]) as f64, r(off[2] + s[2]) as f64];
        let rel = sub3(hit, vk);
        let proj = r((rel[0] * w[0] + rel[2] * w[2]) + rel[1] * w[1]) as f64;
        if proj < 0.0 {
            continue;
        }
        let ev = sub3(vn, vk);
        if (ev[2] * ev[2] + ev[1] * ev[1] + ev[0] * ev[0]).sqrt() < proj {
            continue;
        }
        // (the original leaves the corner out of the foot of the perpendicular)
        let wp = [r(w[0] * proj) as f64, r(w[1] * proj) as f64, r(w[2] * proj) as f64];
        let nrm = unit3(sub3(hit, wp));
        finish(m, hit, nrm);
        return 1;
    }
    for k in 0..count {
        let vk = vert(m, k);
        let p = sub3(s, vk);
        let b = ((p[2] * u[2] + p[1] * u[1]) + p[0] * u[0]) * 2.0;
        let pp = (p[2] * p[2] + p[1] * p[1]) + p[0] * p[0];
        let bb = b * b;
        let c = (pp - r2) * 4.0;
        if bb < c {
            continue;
        }
        let t = r((-b - (bb - c).sqrt()) * 0.5) as f64;
        if t < 0.0 || len < t {
            continue;
        }
        let off = [r(u[0] * t) as f64, r(u[1] * t) as f64, r(u[2] * t) as f64];
        let hit = [r(s[0] + off[0]) as f64, r(s[1] + off[1]) as f64, r(s[2] + off[2]) as f64];
        let nrm = unit3(sub3(hit, vk));
        finish(m, hit, nrm);
        return 1;
    }
    0
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
            if mode & 2 != 0 {
                for i in 0..n.max(0) as u32 {
                    if fn_005f4b30(m, data + POLY_SIZE * i, start, end, rad) != 0 {
                        return i + 1;
                    }
                }
            }
            0
        }
        VOL_WALL => {
            // `count` points 12 bytes apart; a wall between each two in turn.
            let (n, data) = (m.i32(vol + 4), m.u32(vol + 8));
            for i in 0..(n - 1).max(0) as u32 {
                if fn_005f5a80(m, mode, data + 12 * i, data + 12 * (i + 1), start, end, rad) != 0 {
                    return i + 1;
                }
            }
            0
        }
        _ => 0,
    }
}

/// `0x5f5a80`: the segment `start`..`end` against the wall standing on the line from `a` to
/// `b`: vertical, unbounded up and down, facing `(a - b) x up`. Only a segment moving into
/// the wall from its front hits; the hit point (its height taken along the segment) goes to
/// `start`, the wall normal to `end`. With `mode` bit 2 a sphere of radius `rad` is swept: it
/// touches the wall face, or else a sphere moving along the wall touches one of its ends
/// (then `start` is that end at height 0).
pub fn fn_005f5a80(m: &mut Mem, mode: u32, a: u32, b: u32, start: u32, end: u32, rad: f32) -> u32 {
    let (pa, pb) = (v3(m, a), v3(m, b));
    let (s0, e0) = (v3(m, start), v3(m, end));
    let up = v3(m, crate::globals::VEC_UP);
    let e = [r(pa[0] - pb[0]) as f64, r(pa[1] - pb[1]) as f64, r(pa[2] - pb[2]) as f64];
    let c = [r(e[1] * up[2] - e[2] * up[1]) as f64, r(e[2] * up[0] - e[0] * up[2]) as f64, r(e[0] * up[1] - e[1] * up[0]) as f64];
    let len = r((c[2] * c[2] + c[1] * c[1] + c[0] * c[0]).sqrt()) as f64;
    let n = if len < 1e-5f64 as f32 as f64 {
        v3(m, crate::globals::VEC_ZERO)
    } else {
        [r(c[0] / len) as f64, r(c[1] / len) as f64, r(c[2] / len) as f64]
    };
    let neg = [r(-n[0]) as f64, r(-n[1]) as f64, r(-n[2]) as f64];
    // The plane offset, then everything flat.
    let d = r(neg[1] * pa[1] + neg[2] * pa[2] + neg[0] * pa[0]) as f64;
    let (s, en) = ([s0[0], s0[1], 0.0], [e0[0], e0[1], 0.0]);
    let (pa, pb) = ([pa[0], pa[1], 0.0], [pb[0], pb[1], 0.0]);
    let dir = [r(en[0] - s[0]) as f64, r(en[1] - s[1]) as f64, 0.0];
    let dot = |u: [f64; 3], v: [f64; 3]| u[2] * v[2] + u[1] * v[1] + u[0] * v[0];
    if !(dir[1] * n[1] + dir[2] * n[2] + dir[0] * n[0] <= 1e-5 && -1e-5 <= s[1] * n[1] + n[2] * s[2] + n[0] * s[0] + d) {
        return 0;
    }
    if mode & 2 != 0 {
        return wall_sphere(m, start, end, rad as f64, s0, e0, s, en, pa, pb, n, d, dir);
    }
    if 1e-5 < dot(en, n) + d {
        return 0;
    }
    let diff = [r(en[0] - s[0]) as f64, r(en[1] - s[1]) as f64, r(en[2] - s[2]) as f64];
    let dn = dot(diff, n);
    let t = if 0.0001 <= dn.abs() { r(-((dot(s, n) + d) / dn)) as f64 } else { 0.5 };
    if !(-1e-5 <= t && t <= 1.00001) {
        return 0;
    }
    let sc = [r(diff[0] * t) as f64, r(diff[1] * t) as f64, r(diff[2] * t) as f64];
    let hit = [r(s[0] + sc[0]) as f64, r(s[1] + sc[1]) as f64, r(s[2] + sc[2]) as f64];
    let length = |u: [f64; 3], v: [f64; 3]| {
        let w = [r(u[0] - v[0]) as f64, r(u[1] - v[1]) as f64, r(u[2] - v[2]) as f64];
        (w[2] * w[2] + w[1] * w[1] + w[0] * w[0]).sqrt()
    };
    let span = length(pb, pa) + 1e-5;
    if length(hit, pa) <= span && length(hit, pb) <= span {
        set3(m, start, [r(hit[0]), r(hit[1]), r((e0[2] - s0[2]) * t + s0[2])]);
        set3(m, end, [r(n[0]), r(n[1]), r(n[2])]);
        return 1;
    }
    0
}

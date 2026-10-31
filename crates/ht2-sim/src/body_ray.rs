//! Segment tests of the body's collision machinery: `0x4e6578` (a segment cast through the room object's virtual
//! `[vtable+0x34](4, rec)`, hit brought back to the node frame by `0x4e5698`) and `0x5843c0` (the sphere-like test of
//! the body: the cast from the body position `(matrix +0x218..)` to a point, then the distance of the end point from the
//! hit plane and the scaled square root of the ratio of the hit distance to the segment length). The cast itself is an
//! external owner (`ProbeOwner::cast`, answered by the scene graph).
use crate::globals::MAT_IDENTITY;
use crate::mem::Mem;
use crate::scene_probe::{query_to_local, ProbeOwner};

const SCRATCH: u32 = 0x7ffe_f000;

/// `0x4e6578(a, obj, p0, p1, rec)` (cdecl): fills the query record `rec` (identity matrix, segment `p0 -> p1`, `+0x74 = 0`,
/// `+0x68 = 0`, `+0x78 = a`), casts with kind 4 and on a hit brings the hit into the node's frame. Returns 1 on a hit.
pub fn segment_cast(m: &mut Mem, a: u32, obj: u32, p0: u32, p1: u32, rec: u32, owner: &mut dyn ProbeOwner) -> u32 {
    for i in 0..12 {
        let v = m.u32(MAT_IDENTITY + 4 * i);
        m.set_u32(rec + 4 + 4 * i, v);
    }
    for i in 0..3 {
        let (u, v) = (m.u32(p0 + 4 * i), m.u32(p1 + 4 * i));
        m.set_u32(rec + 0x4c + 4 * i, u);
        m.set_u32(rec + 0x58 + 4 * i, v);
    }
    m.set_u32(rec, 0);
    m.set_u32(rec + 0x74, 0);
    m.set_u32(rec + 0x68, 0);
    m.set_u32(rec + 0x78, a);
    let flags = owner.cast(m, obj, 4, rec);
    if flags & 1 != 0 {
        query_to_local(m, rec, SCRATCH, SCRATCH + 0x100);
        1
    } else {
        0
    }
}

/// `0x57de90`: where the body keeps its matrix (`+0x1f4`, or `+0x700` while `[+0x5a0] != 0`).
fn body_matrix(m: &Mem, body: u32) -> u32 {
    if m.u32(body + 0x5a0) != 0 {
        body + 0x700
    } else {
        body + 0x1f4
    }
}

/// `0x5843c0(body, target, ratio, distance, rec)` (cdecl, `f32*` for `ratio` and `distance`): casts from the body position to
/// `target + 0x24` through the room object (`[+0x320]`, or `[+0x7f0]` while `[+0x5a0] != 0`). Returns 0 on a miss, 2 (and
/// `*ratio = 0`) when either squared length is below 1e-6, else 1 with `*distance = n . (hit - end)` and
/// `*ratio *= sqrt(min(|hit - start|^2 / |end - start|^2, 1))`.
pub fn body_ray_test(m: &mut Mem, body: u32, target: u32, ratio: u32, distance: u32, rec: u32, owner: &mut dyn ProbeOwner) -> u32 {
    let room = if m.u32(body + 0x5a0) != 0 { m.u32(body + 0x7f0) } else { m.u32(body + 0x320) };
    let mat = body_matrix(m, body);
    let (p0, p1) = (SCRATCH + 0x200, SCRATCH + 0x20c);
    for i in 0..3 {
        let (u, v) = (m.u32(mat + 0x24 + 4 * i), m.u32(target + 0x24 + 4 * i));
        m.set_u32(p0 + 4 * i, u);
        m.set_u32(p1 + 4 * i, v);
    }
    if segment_cast(m, body, room, p0, p1, rec, owner) & 1 == 0 {
        return 0;
    }
    let f = |m: &Mem, a: u32| m.f32(a) as f64;
    let (s, e) = ([f(m, p0), f(m, p0 + 4), f(m, p0 + 8)], [f(m, p1), f(m, p1 + 4), f(m, p1 + 8)]);
    // v1 = end - start (x, y stored as f32, z kept on the FPU), w = hit - start (all f32)
    let v1 = [(e[0] - s[0]) as f32 as f64, (e[1] - s[1]) as f32 as f64, e[2] - s[2]];
    let w = [(f(m, rec + 0x34) - s[0]) as f32 as f64, (f(m, rec + 0x38) - s[1]) as f32 as f64, (f(m, rec + 0x3c) - s[2]) as f32 as f64];
    let h = [f(m, rec + 0x34) - e[0], f(m, rec + 0x38) - e[1], f(m, rec + 0x3c) - e[2]];
    let d = (f(m, rec + 0x44) * h[1] + f(m, rec + 0x48) * h[2]) + f(m, rec + 0x40) * h[0];
    m.set_f32(distance, d as f32);
    let a = ((v1[0] * v1[0] + v1[1] * v1[1]) + v1[2] * v1[2]) as f32;
    let b = ((w[0] * w[0] + w[1] * w[1]) + w[2] * w[2]) as f32;
    if !(b as f64 >= 1e-6) || !(a as f64 >= 1e-6) {
        m.set_u32(ratio, 0);
        return 2;
    }
    let mut q = b as f64 / a as f64;
    if !(q <= 1.0) {
        q = 1.0;
    }
    let r = q.sqrt() * f(m, ratio);
    m.set_f32(ratio, r as f32);
    1
}

#[cfg(test)]
mod tests {
    use super::*;
    fn unhex(s: &str) -> Vec<u8> {
        (0..s.len()).step_by(2).map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap()).collect()
    }
    struct Stub {
        mat: Vec<u8>,
        pn: Vec<u8>,
        flag: u32,
        calls: Vec<String>,
    }
    impl ProbeOwner for Stub {
        fn cast(&mut self, m: &mut Mem, owner: u32, kind: u32, q: u32) -> u32 {
            let seg: String = m.read(q + 0x4c, 24).iter().map(|b| format!("{b:02x}")).collect();
            self.calls.push(format!("{owner:x}|{kind:x}|{q:x}|{seg}"));
            m.write(q + 4, &self.mat);
            m.write(q + 0x34, &self.pn);
            self.flag
        }
        fn set_case(&mut self, _: &mut Mem, _: u32, _: i32) {}
    }
    #[test]
    fn native_body_ray() {
        let base = 0x2a010000u32;
        let at = |i: u32| base + 0x2000 * i;
        let (body, tgt, ratio, dist, rec, room) = (at(0), at(1), at(2), at(3), at(4), at(5));
        let mut n = 0;
        for (c, line) in include_str!("../../../tools/emu/golden_body_ray.tsv").lines().enumerate() {
            let v: Vec<&str> = line.split('\t').collect();
            let mut m = Mem::new();
            crate::globals::install_constants(&mut m);
            for it in v[1].split(',').filter(|s| !s.is_empty()) {
                let (o, x) = it.split_once(':').unwrap();
                m.write(body + u32::from_str_radix(o, 16).unwrap(), &unhex(x));
            }
            // the room pointers in the golden are the emulator's address of the room object
            m.set_u32(body + 0x320, room);
            m.set_u32(body + 0x7f0, room);
            m.write(tgt + 0x24, &unhex(v[2]));
            m.write(ratio, &unhex(v[3]));
            m.set_f32(dist, -77.0);
            let mut o = Stub { mat: unhex(v[4]), pn: unhex(v[5]), flag: v[6].parse().unwrap(), calls: vec![] };
            let r = body_ray_test(&mut m, body, tgt, ratio, dist, rec, &mut o);
            let hex = |a: u32, k: u32| m.read(a, k as usize).iter().map(|b| format!("{b:02x}")).collect::<String>();
            assert_eq!(format!("{r:x}"), v[7], "case {c}: result");
            assert_eq!(hex(ratio, 4), v[8], "case {c}: ratio");
            assert_eq!(hex(dist, 4), v[9], "case {c}: distance");
            assert_eq!(hex(rec, 0x80), v[10], "case {c}: rec");
            assert_eq!(o.calls.join(";"), v[11], "case {c}: cast");
            n += 1;
        }
        assert_eq!(n, 300);
    }
}

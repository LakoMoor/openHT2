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

/// `0x4e67f9(a, room, radius, p0, p1, rec)` (cdecl): the record as in `segment_cast` but with `rec[0] = radius`, queried against
/// the road of `room` (`0x4e6059(room, 6, rec)`, `World::room_collide(0, ..)`). Returns the query's flags when bit 0 is set,
/// else 0; the hit stays in the room's frame.
pub fn road_segment_cast(m: &mut Mem, a: u32, room: u32, radius: u32, p0: u32, p1: u32, rec: u32, world: &mut dyn crate::contact::World) -> u32 {
    room_segment_cast(m, a, room, radius, p0, p1, rec, 0, world)
}

/// `0x4e6d41`: the same against the objects of the room (`0x4e623e(room, 6, rec)`, `World::room_collide(1, ..)`).
pub fn object_segment_cast(m: &mut Mem, a: u32, room: u32, radius: u32, p0: u32, p1: u32, rec: u32, world: &mut dyn crate::contact::World) -> u32 {
    room_segment_cast(m, a, room, radius, p0, p1, rec, 1, world)
}

#[allow(clippy::too_many_arguments)]
fn room_segment_cast(m: &mut Mem, a: u32, room: u32, radius: u32, p0: u32, p1: u32, rec: u32, which: u32, world: &mut dyn crate::contact::World) -> u32 {
    for i in 0..12 {
        let v = m.u32(MAT_IDENTITY + 4 * i);
        m.set_u32(rec + 4 + 4 * i, v);
    }
    for i in 0..3 {
        let (u, v) = (m.u32(p0 + 4 * i), m.u32(p1 + 4 * i));
        m.set_u32(rec + 0x4c + 4 * i, u);
        m.set_u32(rec + 0x58 + 4 * i, v);
    }
    m.set_u32(rec, radius);
    m.set_u32(rec + 0x74, 0);
    m.set_u32(rec + 0x68, 0);
    m.set_u32(rec + 0x78, a);
    let r = world.room_collide(m, which, room, 6, rec);
    if r & 1 != 0 {
        r
    } else {
        0
    }
}

/// `0x4e560a(obj, p0, p1, rec, radius)` (cdecl): a null `obj` gives 0; else fills the record (identity matrix, segment, `+0x74 =
/// +0x68 = +0x64 = 0`, `rec[0] = radius`) and returns the flags of `[obj->vtable+0x34](6, rec)`.
pub fn object_cast_raw(m: &mut Mem, obj: u32, p0: u32, p1: u32, rec: u32, radius: u32, owner: &mut dyn ProbeOwner) -> u32 {
    if obj == 0 {
        return 0;
    }
    for i in 0..12 {
        let v = m.u32(MAT_IDENTITY + 4 * i);
        m.set_u32(rec + 4 + 4 * i, v);
    }
    for i in 0..3 {
        let (u, v) = (m.u32(p0 + 4 * i), m.u32(p1 + 4 * i));
        m.set_u32(rec + 0x4c + 4 * i, u);
        m.set_u32(rec + 0x58 + 4 * i, v);
    }
    m.set_u32(rec + 0x74, 0);
    m.set_u32(rec + 0x68, 0);
    m.set_u32(rec, radius);
    m.set_u32(rec + 0x64, 0);
    owner.cast(m, obj, 6, rec)
}

/// `0x4e5a45`: `object_cast_raw` and, on a hit (bit 0), the hit brought into the node's frame (`0x4e5698`).
pub fn object_cast(m: &mut Mem, obj: u32, p0: u32, p1: u32, rec: u32, radius: u32, owner: &mut dyn ProbeOwner) -> u32 {
    let r = object_cast_raw(m, obj, p0, p1, rec, radius, owner);
    if r & 1 != 0 {
        query_to_local(m, rec, SCRATCH, SCRATCH + 0x100);
    }
    r
}

/// `0x57de90`: where the body keeps its matrix (`+0x1f4`, or `+0x700` while `[+0x5a0] != 0`).
fn body_matrix(m: &Mem, body: u32) -> u32 {
    if m.u32(body + 0x5a0) != 0 {
        body + 0x700
    } else {
        body + 0x1f4
    }
}

/// `0x584570(body, p0*, p1*, target, rec)` (cdecl): the body against its (up to six) neighbours listed at `+0xdb4` (the list ends at
/// the first null). A neighbour is considered when `((|dv|_1 * dt + r_n + r_b))^2 >= [+0xdcc + 4i]` (`dv` the difference of the
/// velocity words `+0xce4..`, `dt` as `f32`). A neighbour of class 2 with `[+0xf88] == 2` is a sphere (centre: its matrix
/// position, radius `[+0xcc8]`): `p0`/`p1` are set to the body position and `target + 0x24`, and unless they coincide the segment
/// is tested against the sphere with the swept radius `[body+0xcd0]`; a hit puts the hit point and normal into `rec + 0x34` /
/// `+0x40`, the neighbour into `rec + 0x74` and returns 1. Other neighbours are cast with `0x4e5a45(n + 0x100, p0, p1, rec,
/// [body+0xcd0])` (the caller's `p0`/`p1`), a hit returns its flags. Returns 0 when nothing is hit. The temporary shape's heap
/// allocation and release (`0x632643`, `0x5f4a90`) are not modelled (no effect on the result).
pub fn body_contact_test(m: &mut Mem, body: u32, p0: u32, p1: u32, target: u32, rec: u32, owner: &mut dyn ProbeOwner) -> u32 {
    let dt = crate::globals::dt_hi(m) as f32 as f64;
    let (shape, data, l0, l1) = (SCRATCH + 0x400, SCRATCH + 0x440, SCRATCH + 0x480, SCRATCH + 0x490);
    let f = |m: &Mem, a: u32| m.f32(a) as f64;
    for i in 0..6u32 {
        let n = m.u32(body + 0xdb4 + 4 * i);
        if n == 0 {
            return 0;
        }
        let (z, y, x) = (f(m, n + 0xcec) - f(m, body + 0xcec), f(m, n + 0xce8) - f(m, body + 0xce8), f(m, n + 0xce4) - f(m, body + 0xce4));
        let sum = (x.abs() + y.abs()) + z.abs();
        let reach = (sum * dt + f(m, n + 0xcc8)) + f(m, body + 0xcc8);
        if !(reach * reach >= f(m, body + 0xdcc + 4 * i)) {
            continue;
        }
        if m.u32(n + 0x188) == 2 && m.u32(n + 0xf88) == 2 {
            let (nm, bm) = (body_matrix(m, n), body_matrix(m, body));
            for k in 0..3 {
                let (c, b, t) = (m.u32(nm + 0x24 + 4 * k), m.u32(bm + 0x24 + 4 * k), m.u32(target + 0x24 + 4 * k));
                m.set_u32(data + 4 * k, c);
                m.set_u32(p0 + 4 * k, b);
                m.set_u32(p1 + 4 * k, t);
                m.set_u32(l0 + 4 * k, b);
                m.set_u32(l1 + 4 * k, t);
            }
            let radius = m.u32(n + 0xcc8);
            m.set_u32(data + 12, radius);
            m.set_u32(shape, 1);
            m.set_u32(shape + 8, data);
            if !crate::matrix::vec_near(m, l0, l1) {
                let rad = m.f32(body + 0xcd0);
                if crate::shape::fn_005f6820(m, shape, 2, l0, l1, rad) != 0 {
                    for k in 0..3 {
                        let (h, nrm) = (m.u32(l0 + 4 * k), m.u32(l1 + 4 * k));
                        m.set_u32(rec + 0x34 + 4 * k, h);
                        m.set_u32(rec + 0x40 + 4 * k, nrm);
                    }
                    m.set_u32(rec + 0x74, n);
                    return 1;
                }
            }
        } else {
            let radius = m.u32(body + 0xcd0);
            let r = object_cast(m, n + 0x100, p0, p1, rec, radius, owner);
            if r & 1 != 0 {
                return r;
            }
        }
    }
    0
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

    #[test]
    fn native_road_segment_cast() {
        segment_cast_golden(0, include_str!("../../../tools/emu/golden_road_segment_cast.tsv"));
        segment_cast_golden(1, include_str!("../../../tools/emu/golden_object_segment_cast.tsv"));
    }

    fn segment_cast_golden(which_id: u32, golden: &str) {
        struct W {
            which: u32,
            hit: Vec<u8>,
            flags: u32,
            calls: Vec<String>,
        }
        impl crate::contact::World for W {
            fn collide(&mut self, _: &mut Mem, _: u32, _: u32, _: u32) -> u32 {
                unreachable!()
            }
            fn room_collide(&mut self, m: &mut Mem, which: u32, room: u32, mode: u32, rec: u32) -> u32 {
                assert_eq!(which, self.which);
                let dump: String = m.read(rec, 0x7c).iter().map(|b| format!("{b:02x}")).collect();
                self.calls.push(format!("{room:x}|{mode:x}|{rec:x}|{dump}"));
                m.write(rec + 0x34, &self.hit);
                self.flags
            }
        }
        let base = 0x2a010000u32;
        let (rec, p0, p1) = (base, base + 0x1000, base + 0x1100);
        let mut n = 0;
        for (c, line) in golden.lines().enumerate() {
            let v: Vec<&str> = line.split('\t').collect();
            let mut m = Mem::new();
            crate::globals::install_constants(&mut m);
            let u = |x: &str| u32::from_str_radix(x, 16).unwrap();
            m.write(p0, &unhex(v[3]));
            m.write(p1, &unhex(v[4]));
            // the record's previous contents are random in the golden, but the function overwrites all words it reads
            let mut w = W { which: which_id, hit: unhex(v[5]), flags: u(v[6]), calls: vec![] };
            let rad = u32::from_le_bytes(unhex(v[2]).try_into().unwrap());
            let r = if which_id == 0 { road_segment_cast(&mut m, u(v[0]), u(v[1]), rad, p0, p1, rec, &mut w) } else { object_segment_cast(&mut m, u(v[0]), u(v[1]), rad, p0, p1, rec, &mut w) };
            assert_eq!(format!("{r:x}"), v[7], "case {c}");
            let fin: String = m.read(rec, 0x80).iter().map(|b| format!("{b:02x}")).collect();
            // compare the words the function writes; the rest of the record was random in the emulator only
            for (i, (a, b)) in fin.as_bytes().chunks(8).zip(v[8].as_bytes().chunks(8)).enumerate() {
                let written = matches!(i, 0 | 1..=12 | 0x13..=0x18 | 0x1a | 0x1d | 0x1e) || (0xd..=0x12).contains(&i);
                if written {
                    assert_eq!(a, b, "case {c}: rec word {i:x}");
                }
            }
            let head = |x: &str| x.splitn(4, '|').take(2).collect::<Vec<_>>().join("|");
            assert_eq!(w.calls.len(), 1, "case {c}");
            assert_eq!(head(&w.calls[0]), head(v[9]), "case {c}: room query");
            n += 1;
        }
        assert_eq!(n, 200);
    }

    #[test]
    fn native_object_cast() {
        struct Stub {
            mat: Vec<u8>,
            pn: Vec<u8>,
            flags: u32,
            calls: Vec<String>,
        }
        impl ProbeOwner for Stub {
            fn cast(&mut self, m: &mut Mem, owner: u32, kind: u32, q: u32) -> u32 {
                let d: String = m.read(q, 0x7c).iter().map(|b| format!("{b:02x}")).collect();
                self.calls.push(format!("{owner:x}|{kind:x}|{q:x}|{d}"));
                m.write(q + 4, &self.mat);
                m.write(q + 0x34, &self.pn);
                self.flags
            }
            fn set_case(&mut self, _: &mut Mem, _: u32, _: i32) {}
        }
        let base = 0x2a010000u32;
        let at = |i: u32| base + 0x2000 * i;
        let (obj, rec, p0, p1) = (at(0), at(2), at(3), at(4));
        let mut n = 0;
        for (c, line) in include_str!("../../../tools/emu/golden_object_cast.tsv").lines().enumerate() {
            let v: Vec<&str> = line.split('\t').collect();
            let mut m = Mem::new();
            crate::globals::install_constants(&mut m);
            m.write(p0, &unhex(v[1]));
            m.write(p1, &unhex(v[2]));
            let rad = u32::from_le_bytes(unhex(v[3]).try_into().unwrap());
            let mut o = Stub { mat: unhex(v[4]), pn: unhex(v[5]), flags: u32::from_str_radix(v[6], 16).unwrap(), calls: vec![] };
            let r = object_cast(&mut m, if v[0] == "1" { obj } else { 0 }, p0, p1, rec, rad, &mut o);
            assert_eq!(format!("{r:x}"), v[7], "case {c}");
            let fin: Vec<String> = (0..0x20).map(|w| format!("{:08x}", m.u32(rec + 4 * w).swap_bytes())).collect();
            let want: Vec<&str> = (0..0x20).map(|w| &v[8][w * 8..w * 8 + 8]).collect();
            for w in 0..0x20usize {
                let written = w == 0 || (1..=0x18).contains(&w) || matches!(w, 0x19 | 0x1a | 0x1d);
                if written && v[0] == "1" {
                    assert_eq!(fin[w], want[w], "case {c}: rec word {w:x}");
                }
            }
            let head = |x: &str| x.splitn(4, '|').take(3).collect::<Vec<_>>().join("|");
            assert_eq!(o.calls.len(), (v[0] == "1") as usize, "case {c}");
            if let Some(call) = o.calls.first() {
                assert_eq!(head(call), head(v[9]), "case {c}: call");
            }
            n += 1;
        }
        assert_eq!(n, 250);
    }

    #[test]
    fn native_body_contact() {
        struct Stub {
            pn: Vec<u8>,
            flags: u32,
            calls: Vec<String>,
        }
        impl ProbeOwner for Stub {
            fn cast(&mut self, m: &mut Mem, owner: u32, kind: u32, q: u32) -> u32 {
                assert_eq!(kind, 6);
                let hex = |a: u32, n: usize| m.read(a, n).iter().map(|b| format!("{b:02x}")).collect::<String>();
                self.calls.push(format!("cast|{owner:x}|{}|{}|{:x}", hex(q + 0x4c, 12), hex(q + 0x58, 12), m.u32(q)));
                for k in 0..12u32 {
                    m.set_f32(q + 4 + 4 * k, if k % 4 == 0 { 1.0 } else { 0.0 });
                }
                m.write(q + 0x34, &self.pn);
                self.flags
            }
            fn set_case(&mut self, _: &mut Mem, _: u32, _: i32) {}
        }
        let base = 0x2a000000u32;
        let (body, rec, tgt, p0, p1, clk) = (base + 0x10000, base + 0x20000, base + 0x21000, base + 0x22000, base + 0x23000, base + 0x24000);
        let nb = |i: u32| base + 0x12000 + 0x2000 * i;
        let mut n = 0;
        for (c, line) in include_str!("../../../tools/emu/golden_body_contact.tsv").lines().enumerate() {
            let v: Vec<&str> = line.split('\t').collect();
            let mut m = Mem::new();
            crate::globals::install_constants(&mut m);
            m.set_u32(0x6974c4, clk);
            m.write(0x6974c2, &0i16.to_le_bytes());
            let put = |m: &mut Mem, at: u32, list: &str| {
                for it in list.split(',').filter(|s| !s.is_empty()) {
                    let (o, x) = it.split_once(':').unwrap();
                    m.write(at + u32::from_str_radix(o, 16).unwrap(), &unhex(x));
                }
            };
            put(&mut m, body, v[0]);
            for (i, set) in v[1].split(';').filter(|s| !s.is_empty()).enumerate() {
                put(&mut m, nb(i as u32), set);
            }
            m.write(clk + 0xb90, &unhex(v[2]));
            m.write(tgt + 0x24, &unhex(v[3]));
            m.write(p0, &unhex(v[4]));
            m.write(p1, &unhex(v[5]));
            let mut o = Stub { pn: unhex(v[6]), flags: v[7].parse().unwrap(), calls: vec![] };
            let r = body_contact_test(&mut m, body, p0, p1, tgt, rec, &mut o);
            let hex = |m: &Mem, a: u32, k: usize| m.read(a, k).iter().map(|b| format!("{b:02x}")).collect::<String>();
            assert_eq!(format!("{r:x}"), v[8], "case {c}: result");
            assert_eq!(hex(&m, p0, 12), v[10], "case {c}: p0");
            assert_eq!(hex(&m, p1, 12), v[11], "case {c}: p1");
            let events: Vec<&str> = v[12].split(';').filter(|e| e.starts_with("cast")).collect();
            assert_eq!(o.calls.join(";"), events.join(";"), "case {c}: casts");
            // the words the function writes itself (not the stub's): hit point / normal / neighbour on a shape hit
            let words: Vec<String> = [0x34u32, 0x38, 0x3c, 0x40, 0x44, 0x48, 0x74].iter().map(|w| hex(&m, rec + w, 4)).collect();
            let want: Vec<String> = [0x34usize, 0x38, 0x3c, 0x40, 0x44, 0x48, 0x74].iter().map(|w| v[9][w * 2..w * 2 + 8].to_string()).collect();
            assert_eq!(words, want, "case {c}: rec");
            n += 1;
        }
        assert_eq!(n, 300);
    }
}

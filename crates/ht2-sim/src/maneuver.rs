//! `0x54f040`: the maneuver chooser of a vehicle (`__fastcall`, `ecx` = the vehicle scene). It looks at the vehicle in front
//! (`[scene + 0x414]`), asks the road module where both stand and which way their roads run, and picks a manoeuvre 0..6
//! (stored in `[scene + 0x5074]`).
//!
//! The function's frame is modelled as memory at `f` (the original's `esp` after its pushes), the offsets in the code are
//! the original's `[esp + X]`. The x87 arithmetic is done in `f64` (the game runs with a 53-bit control word) and every
//! `fstp dword` is an `as f32`. The road module calls go through [`ManeuverRoad`].

use crate::instruments::{ftol_61d830, maneuver_prologue, object_matrix_select, vehicle_next_point, vehicle_second_kind, world_list_find, world_object_by_id};
use crate::mem::Mem;

/// The road module calls of the maneuver chooser (`PositionId` = 8 dwords). The queries themselves live in `road.rs`; the
/// golden test replays recorded answers (the same pattern as `SceneLightOwner`).
pub trait ManeuverRoad {
    /// `0x533340(name, pos, &object, &position_id, fallback)`: writes the object at `obj_out` and the place at `pid_out`.
    fn zone(&mut self, m: &mut Mem, name: u32, pos: u32, obj_out: u32, pid_out: u32, fallback: u32) -> u32;
    /// `0x445c90(out, pid)` (`getRightDirection`): returns the address of the resulting `PositionId`.
    fn opposite(&mut self, m: &mut Mem, out: u32, pid: [u32; 8]) -> u32;
    /// `0x438d70(pid, out)` (`getRoadParameters`): fills the 0x30 bytes at `out`.
    fn params(&mut self, m: &mut Mem, pid: [u32; 8], out: u32);
    /// `0x439580(out, pid)` (`getCoordinates`): three floats at `out`.
    fn coordinates(&mut self, m: &mut Mem, out: u32, pid: [u32; 8]);
    /// `0x439320(out, pid)` (`getTangent`): three floats at `out`, returns `out`.
    fn tangent(&mut self, m: &mut Mem, out: u32, pid: [u32; 8]) -> u32;
    /// `0x533510(&pid)`: the one-way test.
    fn one_way(&mut self, m: &mut Mem, pid: u32) -> u32;
}

fn pid_at(m: &Mem, a: u32) -> [u32; 8] {
    let mut p = [0u32; 8];
    for (i, w) in p.iter_mut().enumerate() {
        *w = m.u32(a + 4 * i as u32);
    }
    p
}

/// Where the original's control flow goes after the road work.
enum Exit {
    /// `0x54fe1d`: the choice is final.
    Done(u32),
    /// `0x54fdd1`: the choice may still become 6.
    Check(u32),
    /// `0x54fdcd`: the choice is `[f + 0x30]`.
    Tail,
}

/// `0x54f040`. Returns the other vehicle + 0x10, or 0.
pub fn maneuver(m: &mut Mem, scene: u32, f: u32, road: &mut dyn ManeuverRoad) -> u32 {
    if !maneuver_prologue(m, scene, f) {
        return 0;
    }
    let v = world_list_find(m, m.u32(scene + 0x414));
    m.set_u32(f + 0x98, v);
    let exit = if v == 0 { Exit::Tail } else { road_work(m, scene, f, road, v) };
    let choice = match exit {
        Exit::Done(c) => c,
        Exit::Check(c) => kind6(m, f, c),
        Exit::Tail => {
            let c = m.u32(f + 0x30);
            kind6(m, f, c)
        }
    };
    finish(m, scene, f, choice)
}

/// `0x54fdd1`..`0x54fe18`: with the first gate's state 6, the choice becomes 6 unless the vehicle is further than 5 and the
/// scene's axes point away from it.
fn kind6(m: &Mem, f: u32, choice: u32) -> u32 {
    let ff = |o: u32| m.f32(f + o) as f64;
    if m.u32(f + 0xb0) != 6 {
        return choice;
    }
    if ff(0xbc) > 5.0 {
        let dot = (ff(0x34) * ff(0x58) + ff(0x3c) * ff(0x60)) + ff(0x38) * ff(0x5c);
        if dot < 0.0 || dot.is_nan() {
            return choice;
        }
    }
    6
}

/// `0x54fe1d`..: the distance to the vehicle ahead and the stores into the scene.
fn finish(m: &mut Mem, scene: u32, f: u32, choice: u32) -> u32 {
    let v = m.u32(f + 0x98);
    let x = m.u32(f + 0x9c);
    for i in 0..3u32 {
        m.set_u32(f + 0x20 + 4 * i, m.u32(x + 0x1a0 + 4 * i));
    }
    for i in 0..3u32 {
        let d = m.f32(f + 0x20 + 4 * i) as f64 - m.f32(f + 0x34 + 4 * i) as f64;
        m.set_f32(f + 0x4c + 4 * i, d as f32);
    }
    for i in 0..3u32 {
        m.set_u32(f + 0x20 + 4 * i, m.u32(f + 0x4c + 4 * i));
    }
    let len = crate::math::vec_len(m, f + 0x20);
    if !(len >= 3.0) {
        return if v == 0 { 0 } else { v + 0x10 };
    }
    let size = ((len - 3.0) * m.f32(x + 0xcc4) as f64 * 2.5e-05).sqrt() as f32;
    m.set_f32(f + 0x9c, size);
    if choice == 0 {
        let l = crate::math::vec_len(m, scene + 0x1b0);
        m.set_u32(scene + 0x507c, 1);
        m.set_f32(scene + 0x5088, (l * (1.0f32 / 3.0) as f64 + 10.0) as f32);
    } else {
        m.set_u32(scene + 0x507c, 0);
    }
    let clock = m.f64(m.u32(0x6ef1fc) + 0xb98);
    m.set_u32(scene + 0x5074, choice);
    m.set_u32(scene + 0x5078, choice);
    m.set_f32(scene + 0x5094, size);
    m.set_f32(scene + 0x5084, clock as f32);
    if v == 0 {
        m.set_u32(scene + 0x5098, 0);
        return 0;
    }
    let id = world_object_by_id(m, m.u32(scene + 0x414));
    m.set_u32(scene + 0x5098, id);
    v + 0x10
}

/// `0x54f31a`..`0x54fdcd`.
fn road_work(m: &mut Mem, scene: u32, f: u32, road: &mut dyn ManeuverRoad, v: u32) -> Exit {
    let (c30, c34, c38) = (m.f32(0x702630) as f64, m.f32(0x702634) as f64, m.f32(0x702638) as f64);
    let ff = |m: &Mem, o: u32| m.f32(f + o) as f64;
    for o in [0xc0, 0xc4, 0xdc, 0xe0, 0xe4, 0xfc] {
        m.set_u32(f + o, u32::MAX);
    }
    for o in [0xc8, 0xcc, 0xd0, 0xd4, 0xd8, 0xe8, 0xec, 0xf0, 0xf4, 0xf8, 0x80, 0x90, 0xb8] {
        m.set_u32(f + o, 0);
    }
    let name = m.u32(m.u32(scene + 0x334) + 0xc);
    road.zone(m, name, scene + 0x258, f + 0x80, f + 0xc0, 0);
    if m.u32(f + 0x80) == 0 {
        return Exit::Tail;
    }
    let name = m.u32(m.u32(v + 0x334) + 0xc);
    road.zone(m, name, f + 0x1cc, f + 0x80, f + 0xe0, 0);
    if m.u32(f + 0x80) == 0 {
        return Exit::Tail;
    }
    for pid in [f + 0xc0, f + 0xe0] {
        let p = road.opposite(m, f + 0x178, pid_at(m, pid));
        let b = m.read(p, 32);
        m.write(pid, &b);
    }
    for o in [0x124, 0x128, 0x154, 0x158] {
        m.set_u32(f + o, u32::MAX);
    }
    for base in [f + 0x100, f + 0x130] {
        for (o, w) in [(0, 0), (8, 0), (0xc, 0), (0x10, 0x66d638), (0x14, 0x66d628), (0x18, 0), (0x1c, 0)] {
            m.set_u32(base + o, w);
        }
    }
    road.params(m, pid_at(m, f + 0xc0), f + 0x100);
    road.params(m, pid_at(m, f + 0xe0), f + 0x130);
    let lanes = m.i32(f + 0x120);
    m.set_f32(f + 0xac, (m.f64(f + 0x108) * 0.5) as f32);
    m.set_u32(f + 0xb4, (lanes > 1) as u32);
    road.coordinates(m, f + 0x4c, pid_at(m, f + 0xc0));
    let t = road.tangent(m, f + 0x20, pid_at(m, f + 0xc0));
    crate::matrix::vec_unit(m, f + 0x40, t);
    // the unit tangent crossed with the axis at 0x702630, stored as floats
    let cross = |m: &mut Mem, a: u32, out: u32| {
        let (x, y, z) = (m.f32(a) as f64, m.f32(a + 4) as f64, m.f32(a + 8) as f64);
        m.set_f32(out, (y * c38 - z * c34) as f32);
        m.set_f32(out + 4, (z * c30 - x * c38) as f32);
        m.set_f32(out + 8, (x * c34 - y * c30) as f32);
    };
    cross(m, f + 0x40, f + 0x14);
    for i in 0..3u32 {
        m.set_u32(f + 0x20 + 4 * i, m.u32(f + 0x14 + 4 * i));
    }
    crate::matrix::vec_unit(m, f + 0x84, f + 0x20);
    for i in 0..3u32 {
        let d = m.f32(f + 0x4c + 4 * i) as f64 - m.f32(scene + 0x258 + 4 * i) as f64;
        m.set_f32(f + 0xa0 + 4 * i, d as f32);
    }
    road.coordinates(m, f + 0x16c, pid_at(m, f + 0xe0));
    let t = road.tangent(m, f + 0x20, pid_at(m, f + 0xe0));
    crate::matrix::vec_unit(m, f + 0x160, t);
    let dot = (ff(m, 0x40) * ff(m, 0x58) + ff(m, 0x48) * ff(m, 0x60)) + ff(m, 0x44) * ff(m, 0x5c);
    if dot < 0.0 || dot.is_nan() {
        for o in [0x84, 0x88, 0x8c] {
            m.set_f32(f + o, -m.f32(f + o));
        }
    }
    let one = road.one_way(m, f + 0xc0);
    for i in 0..3u32 {
        m.set_u32(f + 0x64 + 4 * i, m.u32(f + 0x40 + 4 * i));
    }
    let (a0, a4, a8) = (ff(m, 0xa0), ff(m, 0xa4), ff(m, 0xa8));
    let x1 = a4 * c38 - a8 * c34;
    let x2 = a8 * c30 - a0 * c38;
    let x3 = a0 * c34 - a4 * c30;
    let d2 = (x3 * ff(m, 0x48) + x2 * ff(m, 0x44)) + x1 * ff(m, 0x40);
    if d2 < 0.0 || d2.is_nan() {
        for i in 0..3u32 {
            m.set_f32(f + 0x64 + 4 * i, -m.f32(f + 0x40 + 4 * i));
        }
    }
    m.set_f32(f + 0x2c, (ff(m, 0xac) + 2.0) as f32);
    for o in [0x70, 0x74, 0x78] {
        m.set_u32(f + o, 0);
    }
    m.set_f32(f + 0x7c, (ff(m, 0x2c) + 1.0) as f32);
    m.set_f32(f + 0x94, -m.f32(f + 0x7c));
    let found = sweep(m, scene, f, one != 0);
    // do the roads of the two places meet at the same end points
    let rooms = m.u32(0x68cf38);
    let room = |m: &Mem, o: u32| m.u32(rooms + 4 * m.u32(f + o));
    let end = |m: &Mem, r: u32, o: u32| m.u16(m.u32(r + o) + 0x10);
    let (ra, rb) = (room(m, 0xc0), room(m, 0xe0));
    let (dx, si) = (end(m, ra, 0x70), end(m, ra, 0x74));
    let (cx, ax) = (end(m, rb, 0x70), end(m, rb, 0x74));
    let mut same = if (dx == cx && si == ax) || (dx == ax && si == cx) { 1 } else { m.u32(f + 0xb8) };
    if m.rd8(f + 0x100) & 0x30 != 0 || m.rd8(f + 0x130) & 0x30 != 0 {
        same = 0;
    }
    if !found || same == 0 {
        return Exit::Tail;
    }
    for i in 0..3u32 {
        m.set_u32(f + 0x14 + 4 * i, m.u32(f + 0x58 + 4 * i));
    }
    let l1 = crate::math::vec_len(m, scene + 0x1d4) as f32;
    m.set_f32(f + 0x2c, l1);
    let l2 = crate::math::vec_len(m, f + 0xa0) as f32;
    m.set_f32(f + 0x90, l2);
    if ff(m, 0x2c) > 0.0 {
        for i in 0..3u32 {
            let q = (m.f32(scene + 0x1d4 + 4 * i) as f64 / ff(m, 0x2c)) as f32;
            m.set_f32(f + 0x14 + 4 * i, q);
        }
    }
    let t = [ff(m, 0x64), ff(m, 0x68), ff(m, 0x6c)];
    let s = [ff(m, 0x58), ff(m, 0x5c), ff(m, 0x60)];
    let n = [ff(m, 0x14), ff(m, 0x18), ff(m, 0x1c)];
    let t1 = (t[0] * s[0] + t[2] * s[2]) + t[1] * s[1];
    let (edx, edi) = (t1 > 0.0, t1 > 0.8);
    let esi = ((n[1] * t[1] + n[2] * t[2]) + n[0] * t[0]) > 0.8;
    let t3 = (n[2] * s[2] + n[1] * s[1]) + n[0] * s[0];
    let ecx = !(t3 >= -0.8);
    if ff(m, 0x2c) > 1.0 {
        if !edx && ff(m, 0x90) > 1.5 && m.u32(f + 0xb4) == 0 {
            return Exit::Done(1);
        }
        if ecx {
            return Exit::Done(3);
        }
        if !edi || !esi {
            return Exit::Done(2);
        }
        if m.u32(f + 0xb4) != 0 || !(ff(m, 0x94) > 0.3) || !(ff(m, 0x7c) < 0.0) {
            return Exit::Tail;
        }
        return Exit::Done(5);
    }
    if !edx || !edi {
        return Exit::Done(4);
    }
    let q = (ff(m, 0xac) - ff(m, 0x90)) / 3.0;
    if (ftol_61d830(q) as i32).wrapping_add(1) <= 1 {
        return Exit::Check(0);
    }
    neighbours(m, scene, f)
}

/// `0x54f861`..`0x54f9bd`: the points of the vehicle's list in turn; true when one lies across the lane.
fn sweep(m: &mut Mem, scene: u32, f: u32, one: bool) -> bool {
    let ff = |m: &Mem, o: u32| m.f32(f + o) as f64;
    let mut found = m.u32(f + 0x90) != 0;
    loop {
        let cur = vehicle_next_point(m, scene + 0x10, f + 0x70);
        if found {
            break;
        }
        if cur == 0 {
            for i in 0..3u32 {
                m.set_u32(f + 0x70 + 4 * i, m.u32(0x702650 + 4 * i));
            }
        }
        let g = |o: u32| m.f32(scene + o) as f64;
        let p = [m.f32(f + 0x70) as f64, m.f32(f + 0x74) as f64, m.f32(f + 0x78) as f64];
        let a1 = ((p[2] * g(0x24c) + p[1] * g(0x240)) + p[0] * g(0x234)) + g(0x258);
        let a2 = (((p[2] * g(0x250) + p[1] * g(0x244)) + p[0] * g(0x238)) + g(0x25c)) as f32;
        let a3 = (((p[2] * g(0x254) + p[1] * g(0x248)) + p[0] * g(0x23c)) + g(0x260)) as f32;
        m.set_f32(f + 0x24, a2);
        m.set_f32(f + 0x28, a3);
        let d0 = ff(m, 0x4c) - a1;
        let d1 = ff(m, 0x50) - a2 as f64;
        let dz = (ff(m, 0x54) - a3 as f64) as f32;
        m.set_f32(f + 0x1c, dz);
        let vv = d1 * ff(m, 0x88) + d0 * ff(m, 0x84);
        if vv < ff(m, 0x7c) || vv.is_nan() {
            m.set_f32(f + 0x7c, vv as f32);
        }
        if vv > ff(m, 0x94) {
            m.set_f32(f + 0x94, vv as f32);
        }
        let l = ff(m, 0x2c);
        if one {
            if (vv < 2.0 || vv.is_nan()) && vv > -l {
                found = true;
            }
        } else if vv > -l && (vv < l || vv.is_nan()) {
            found = true;
        }
        if cur == 0 {
            break;
        }
    }
    found
}

/// `0x54fc77`..`0x54fdb5`: the other vehicles around (`scene + 0xdc4`, six slots): the first one that is close, in front
/// and in the way decides.
fn neighbours(m: &mut Mem, scene: u32, f: u32) -> Exit {
    for k in 0..6u32 {
        let slot = scene + 0xdc4 + 4 * k;
        let other = m.u32(slot);
        if other == 0 {
            m.set_u32(f + 0x30, 4);
            return Exit::Done(4);
        }
        let kind2 = vehicle_second_kind(m, other);
        object_matrix_select(m, other, f + 0x178);
        for i in 0..3u32 {
            m.set_u32(f + 0x14 + 4 * i, m.u32(f + 0x178 + 0x24 + 4 * i));
        }
        for i in 0..3u32 {
            let d = m.f32(scene + 0x228 + 4 * i) as f64 - m.f32(f + 0x14 + 4 * i) as f64;
            m.set_f32(f + 0x64 + 4 * i, d as f32);
        }
        if kind2 == scene {
            continue;
        }
        let dist = m.f32(slot + 0x18) as f64;
        if !(dist < 1000.0 || dist.is_nan()) {
            continue;
        }
        for i in 0..3u32 {
            m.set_u32(f + 0x20 + 4 * i, m.u32(other + 0x1c4 + 4 * i));
        }
        let g = |o: u32| m.f32(scene + o) as f64;
        let ff = |m: &Mem, o: u32| m.f32(f + o) as f64;
        let (a, b, c) = (ff(m, 0x14), ff(m, 0x18), ff(m, 0x1c));
        let u = ((b * g(0x2a0) + c * g(0x2ac)) + a * g(0x294)) + g(0x2b8);
        let w = ((b * g(0x2a4) + c * g(0x2b0)) + a * g(0x298)) + g(0x2bc);
        if !(u.abs() < w || u.is_nan() || w.is_nan()) {
            continue;
        }
        let dot = (ff(m, 0x6c) * ff(m, 0x28) + ff(m, 0x68) * ff(m, 0x24)) + ff(m, 0x20) * ff(m, 0x64);
        if dot > 0.0 {
            m.set_u32(f + 0x30, 0);
            return Exit::Tail;
        }
    }
    m.set_u32(f + 0x30, 4);
    Exit::Done(4)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bytes(s: &str) -> Vec<u8> {
        (0..s.len()).step_by(2).map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap()).collect()
    }
    fn hex(s: &str) -> u32 {
        u32::from_str_radix(s, 16).unwrap()
    }
    /// `off:hex,off:hex` runs of non-zero 16-byte blocks at `base`.
    fn load(m: &mut Mem, base: u32, s: &str) {
        for run in s.split(',').filter(|r| !r.is_empty()) {
            let (off, data) = run.split_once(':').unwrap();
            m.write(base + hex(off), &bytes(data));
        }
    }

    /// Replays the recorded road module answers in order and checks the arguments.
    struct Replay {
        calls: Vec<Vec<String>>,
        next: usize,
        case: usize,
    }
    impl Replay {
        fn take(&mut self, kind: &str, args: &[u32]) -> (Vec<u8>, u32) {
            let c = &self.calls[self.next];
            self.next += 1;
            assert_eq!(c[0], kind, "case {}: call {}", self.case, self.next);
            let want: Vec<String> = args.iter().map(|a| format!("{:x}", a)).collect();
            assert_eq!(c[1], want.join(","), "case {}: arguments of {}", self.case, kind);
            (bytes(&c[2]), hex(&c[3]))
        }
    }
    impl ManeuverRoad for Replay {
        fn zone(&mut self, m: &mut Mem, name: u32, pos: u32, obj_out: u32, pid_out: u32, fallback: u32) -> u32 {
            let (b, r) = self.take("zone", &[name, pos, obj_out, pid_out, fallback]);
            m.write(obj_out, &b[..4]);
            m.write(pid_out, &b[4..]);
            r
        }
        fn opposite(&mut self, m: &mut Mem, out: u32, pid: [u32; 8]) -> u32 {
            let mut a = vec![out];
            a.extend(pid);
            let (b, r) = self.take("opp", &a);
            m.write(out, &b);
            r
        }
        fn params(&mut self, m: &mut Mem, pid: [u32; 8], out: u32) {
            let mut a = pid.to_vec();
            a.push(out);
            let (b, _) = self.take("params", &a);
            m.write(out, &b);
        }
        fn coordinates(&mut self, m: &mut Mem, out: u32, pid: [u32; 8]) {
            let mut a = vec![out];
            a.extend(pid);
            let (b, _) = self.take("coord", &a);
            m.write(out, &b);
        }
        fn tangent(&mut self, m: &mut Mem, out: u32, pid: [u32; 8]) -> u32 {
            let mut a = vec![out];
            a.extend(pid);
            let (b, r) = self.take("tan", &a);
            m.write(out, &b);
            r
        }
        fn one_way(&mut self, m: &mut Mem, pid: u32) -> u32 {
            let (b, r) = self.take("one", &[pid]);
            assert_eq!(m.read(pid, 32), b, "case {}: one-way place", self.case);
            r
        }
    }

    #[test]
    fn native_maneuver() {
        // the emulator's addresses (tools/emu/gen_maneuver.py)
        let (scene, clock, glb, rooms, nodes, table, names, veh) =
            (0x2a01_0000u32, 0x2a03_0000u32, 0x2a03_1000u32, 0x2a03_2000u32, 0x2a03_3000u32, 0x2a03_4000u32, 0x2a03_5000u32, 0x2a04_0000u32);
        let _ = names;
        for (c, line) in include_str!("../../../tools/emu/golden_maneuver.tsv").lines().enumerate() {
            let v: Vec<&str> = line.trim_end_matches(['\r', '\n']).split('\t').collect();
            let mut m = Mem::new();
            load(&mut m, scene, v[0]);
            for veh_s in v[1].split('|') {
                let (i, s) = veh_s.split_once('@').unwrap();
                load(&mut m, veh + 0x2000 * i.parse::<u32>().unwrap(), s);
            }
            let (rs, ns) = v[2].split_once('/').unwrap();
            load(&mut m, rooms, rs);
            load(&mut m, nodes, ns);
            let k: Vec<&str> = v[3].split('/').collect();
            m.set_u32(0x6ef1fc, clock);
            m.write(clock + 0xb98, &bytes(k[0]));
            m.write(0x702620, &bytes(k[1]));
            m.write(0x702630, &bytes(k[2]));
            m.write(0x702650, &bytes(k[3]));
            m.set_u32(0x6cec6c, glb);
            m.set_u32(0x68cf38, table);
            // the list heads and the room table are plain pointers
            m.set_u32(0x6f653c, veh + 0x10 + 0x178 + 4);
            m.write(0x6ef1f8, &bytes(k[4]));
            for r in 0..4u32 {
                m.set_u32(table + 4 * r, rooms + 0x100 * r);
            }
            let calls: Vec<Vec<String>> = v[4].split(';').filter(|s| !s.is_empty()).map(|s| s.split('|').map(str::to_string).collect()).collect();
            let mut road = Replay { calls, next: 0, case: c };
            let f = hex(v[5]);
            let got = maneuver(&mut m, scene, f, &mut road);
            assert_eq!(road.next, road.calls.len(), "case {c}: unused road answers");
            assert_eq!(got, hex(v[7]), "case {c}: result");
            assert_eq!(m.read(scene + 0x5070, 0x30), bytes(v[8]), "case {c}: scene words");
        }
    }
}

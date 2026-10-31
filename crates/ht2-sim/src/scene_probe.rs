//! `0x54cb90` (`__thiscall`, ecx = vehicle scene): casts a ray of kind 7 from the scene point `+0xd88` through the
//! owner `[+0x2aa8]` (virtual call `[vtable+0x34]`), looks up the hit object's `K_HFLAG_nnn` / `K_FLAG_nnn` entries in
//! the global name table and switches the second one to the case (`0x4e480c`) that matches the direction of the hit
//! normal in the hit object's frame. The ray cast and `SetCaseSwitch` are external owners (`ProbeOwner`).
//!
//! `f` is a scratch area of `0x180` bytes standing in for the original's stack frame.
use crate::globals::{MAT_IDENTITY, VEC_X, VEC_Y};
use crate::instruments::global_name_lookup;
use crate::math::{mat_inverse, vec_set};
use crate::matrix::{rotate, transform, vec_add_in_place, vec_dot, vec_neg, vec_sub_in_place};
use crate::mem::Mem;

pub trait ProbeOwner {
    /// `[owner->vtable + 0x34](kind, query)`: fills the query record, returns the low bit as the hit flag.
    fn cast(&mut self, m: &mut Mem, owner: u32, kind: u32, query: u32) -> u32;
    /// `0x4e480c(object, index)`.
    fn set_case(&mut self, m: &mut Mem, object: u32, index: i32);
}

/// `0x4e5698`: bring the hit point `q+0x34` and normal `q+0x40` into the frame of the matrix at `q+4`.
fn query_to_local(m: &mut Mem, q: u32, inv: u32, tmp: u32) {
    mat_inverse(m, inv, q + 4);
    transform(m, tmp, q + 0x34, inv);
    for i in 0..3 {
        let v = m.u32(tmp + 4 * i);
        m.set_u32(q + 0x34 + 4 * i, v);
    }
    rotate(m, tmp, q + 0x40, inv);
    for i in 0..3 {
        let v = m.u32(tmp + 4 * i);
        m.set_u32(q + 0x40 + 4 * i, v);
    }
}

pub fn scene_probe(m: &mut Mem, s: u32, f: u32, owner: &mut dyn ProbeOwner) {
    let own = m.u32(s + 0x2aa8);
    if own == 0 {
        return;
    }
    let (q, w, d, t, n, inv, tmp) = (f, f + 0xd0, f + 0xdc, f + 0xe8, f + 0xf4, f + 0x140, f + 0x170);
    crate::contact::rec_init(m, q);
    for i in 0..12 {
        let v = m.u32(MAT_IDENTITY + 4 * i);
        m.set_u32(q + 4 + 4 * i, v);
    }
    let g = |m: &Mem, o: u32| m.f32(s + o) as f64;
    let (x, y, z) = (g(m, 0xd88), g(m, 0xd8c), g(m, 0xd90));
    let o = [
        (g(m, 0x240) * y + g(m, 0x24c) * z + x * g(m, 0x234) + g(m, 0x258)) as f32,
        (g(m, 0x238) * x + g(m, 0x244) * y + g(m, 0x250) * z + g(m, 0x25c)) as f32,
        (g(m, 0x23c) * x + g(m, 0x248) * y + g(m, 0x254) * z + g(m, 0x260)) as f32,
    ];
    for i in 0..3 {
        m.set_f32(q + 0x4c + 4 * i, o[i as usize]);
    }
    let c = [m.f32(VEC_Y) as f64, m.f32(VEC_Y + 4) as f64, m.f32(VEC_Y + 8) as f64];
    let wv = [
        (c[2] * g(m, 0x21c) + c[1] * g(m, 0x210) + c[0] * g(m, 0x204)) as f32,
        (c[2] * g(m, 0x220) + c[1] * g(m, 0x214) + c[0] * g(m, 0x208)) as f32,
        (c[2] * g(m, 0x224) + c[1] * g(m, 0x218) + c[0] * g(m, 0x20c)) as f32,
    ];
    for i in 0..3 {
        m.set_f32(w + 4 * i, wv[i as usize]);
    }
    let pz = (g(m, 0x224) * z + g(m, 0x218) * y + g(m, 0x20c) * x + g(m, 0x230)) as f32;
    let py = (g(m, 0x220) * z + g(m, 0x214) * y + g(m, 0x208) * x + g(m, 0x22c)) as f32;
    let px = (g(m, 0x21c) * z + g(m, 0x210) * y + x * g(m, 0x204) + g(m, 0x228)) as f32;
    vec_set(m, d, px, py, pz);
    vec_add_in_place(m, q + 0x58, t, d);
    let dv = [
        (g(m, 0x228) - g(m, 0x258)) as f32,
        (g(m, 0x22c) - g(m, 0x25c)) as f32,
        (g(m, 0x230) - g(m, 0x260)) as f32,
    ];
    vec_set(m, d, dv[0], dv[1], dv[2]);
    let dot = dv[2] as f64 * wv[2] as f64 + dv[1] as f64 * wv[1] as f64 + dv[0] as f64 * wv[0] as f64;
    if dot > 0.0 {
        vec_add_in_place(m, q + 0x58, t, w);
    } else {
        vec_sub_in_place(m, q + 0x58, t, w);
    }
    let v = m.u32(s + 0xce4);
    m.set_u32(q, v);
    m.set_u32(q + 0x74, 0);
    m.set_u32(q + 0x64, 0);
    let hit_flag = owner.cast(m, own, 7, q);
    query_to_local(m, q, inv, tmp);
    let hit = m.u32(q + 0x64);
    if hit_flag & 1 == 0 || hit == 0 {
        return;
    }
    if m.u32(s + 0x2aac) == 1 && m.u32(hit) != 0x4d {
        return;
    }
    let id = m.i32(hit + 4);
    let (b1, b2) = (f + 0x100, f + 0x120);
    for (buf, text) in [(b1, format!("K_HFLAG_{:03}", id)), (b2, format!("K_FLAG_{:03}", id))] {
        for (i, ch) in text.bytes().chain(std::iter::once(0)).enumerate() {
            m.wr8(buf + i as u32, ch);
        }
    }
    let first = global_name_lookup(m, b1);
    let second = global_name_lookup(m, b2);
    if first == 0 {
        return;
    }
    owner.set_case(m, first, 1);
    if second == 0 {
        return;
    }
    let neg = [-m.f32(q + 0x40), -m.f32(q + 0x44), -m.f32(q + 0x48)];
    vec_set(m, n, neg[0], neg[1], neg[2]);
    if vec_dot(m, n, VEC_Y) > 0.707 {
        owner.set_case(m, second, 1);
        return;
    }
    if vec_dot(m, n, VEC_X) > 0.707 {
        owner.set_case(m, second, 2);
        return;
    }
    vec_neg(m, t, VEC_Y);
    if vec_dot(m, n, t) > 0.707 {
        owner.set_case(m, second, 3);
        return;
    }
    vec_neg(m, t, VEC_X);
    if vec_dot(m, n, t) > 0.707 {
        owner.set_case(m, second, 4);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Replay {
        cast: Vec<String>,
        calls: Vec<(u32, i32)>,
        got: Vec<(u32, i32)>,
    }
    fn unhex(s: &str) -> Vec<u8> {
        (0..s.len()).step_by(2).map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap()).collect()
    }
    impl ProbeOwner for Replay {
        fn cast(&mut self, m: &mut Mem, _owner: u32, kind: u32, q: u32) -> u32 {
            let v: Vec<String> = self.cast[0].split('|').map(String::from).collect();
            assert_eq!(kind, u32::from_str_radix(&v[0], 16).unwrap());
            assert_eq!(m.read(q, 0x70), unhex(&v[1]), "query at the cast");
            if !v[2].is_empty() {
                m.write(q + 4, &unhex(&v[2]));
            }
            m.write(q + 0x34, &unhex(&v[3]));
            m.set_u32(q + 0x64, u32::from_str_radix(&v[4], 16).unwrap());
            u32::from_str_radix(&v[5], 16).unwrap()
        }
        fn set_case(&mut self, _m: &mut Mem, object: u32, index: i32) {
            self.got.push((object, index));
        }
    }
    #[test]
    fn native_scene_probe() {
        let hex = |s: &str| u32::from_str_radix(s, 16).unwrap();
        let (scene, own, table, nodes, strs, hits, frame) =
            (0x2a010000u32, 0x2a020000u32, 0x2a040000u32, 0x2a042000u32, 0x2a060000u32, 0x2a070000u32, 0x2a0a0000u32);
        let mut n = 0;
        for (c, line) in include_str!("../../../tools/emu/golden_scene_probe.tsv").lines().enumerate() {
            let v: Vec<&str> = line.split('\t').collect();
            let mut m = Mem::new();
            crate::globals::install_constants(&mut m);
            for it in v[0].split(',') {
                let (o, x) = it.split_once(':').unwrap();
                m.set_u32(scene + hex(o), hex(x));
            }
            let k = v[2].parse::<usize>().unwrap();
            let image = unhex(v[1]);
            m.write(table, &image[..0x2000]);
            m.write(nodes, &image[0x2000..0x2000 + 0x20 * k]);
            m.write(strs, &image[0x2000 + 0x20 * k..]);
            m.set_u32(0x71e828, table);
            m.write(hits, &unhex(v[3]));
            let _ = own;
            let mut owner = Replay {
                cast: vec![v[4].to_string()],
                calls: v[5]
                    .split(',')
                    .filter(|s| !s.is_empty())
                    .map(|s| {
                        let (a, b) = s.split_once(':').unwrap();
                        (hex(a), b.parse().unwrap())
                    })
                    .collect(),
                got: vec![],
            };
            if v[4].is_empty() {
                owner.cast = vec![String::new()];
            }
            scene_probe(&mut m, scene, frame, &mut owner);
            assert_eq!(owner.got, owner.calls, "case {c}: set_case calls");
            n += 1;
        }
        assert_eq!(n, 500);
    }
}

//! Small pieces of the road-mesh query used by `0x585150`: `0x56f650` (accessor of the collected node list), `0x56f3f0`
//! (starts a collection: the box into globals, the room's traversal virtual `[vtable+0x20](0, 0x56f450)` is an external owner),
//! and `0x5818b0` (`__thiscall` flag setter).
use crate::mem::Mem;

/// The list of collected nodes (30 entries) and their extra words.
pub const NODE_LIST: u32 = 0x6f2e58;
pub const NODE_EXTRA: u32 = 0x6f326c;
pub const BOX: u32 = 0x6f3248;
pub const COUNT: u32 = 0x6f34c0;

/// `0x56f650(idx, o1, o2, o3, o4, o5, o6)` (cdecl): for the collected node `idx` of kind 9 whose `+0x40` is 7 writes
/// `*o1 = [n+0x48]`, `*o2 = [n+0x30]`, `*(f64*)o3 = [n+0x34] * 0.01`, `*o4 = [n+0x3c] + 4`, `*o5 = [n+0x38] - 1`,
/// `*o6 = [0x6f326c + 4 idx]` and returns `[n+0x44]`; otherwise returns -1.
#[allow(clippy::too_many_arguments)]
pub fn mesh_node_info(m: &mut Mem, idx: u32, o: [u32; 6]) -> u32 {
    let n = m.u32(NODE_LIST + 4 * idx);
    if m.u32(n + 0x10) != 9 || m.u32(n + 0x40) != 7 {
        return u32::MAX;
    }
    let extra = m.u32(NODE_EXTRA + 4 * idx);
    m.set_u32(o[5], extra);
    let w = m.u32(n + 0x48);
    m.set_u32(o[0], w);
    let w = m.u32(n + 0x30);
    m.set_u32(o[1], w);
    let scaled = m.i32(n + 0x34) as f64 * 0.01;
    m.set_f64(o[2], scaled);
    let w = m.u32(n + 0x3c).wrapping_add(4);
    m.set_u32(o[3], w);
    let w = m.u32(n + 0x38).wrapping_sub(1);
    m.set_u32(o[4], w);
    m.u32(n + 0x44)
}

/// The room's traversal `[room->vtable + 0x20](0, 0x56f450)` that fills the list through the collector `0x56f450` (the graph walk
/// is the scene's, not ported here: stand-in owner).
pub trait MeshTraversal {
    fn traverse(&mut self, m: &mut Mem, room: u32);
}

/// `0x56f3f0(room, a, b, c, d, count*)` (cdecl): stores the four box words, clears the counter, runs the traversal, writes the
/// counter to `*count` and returns the list address.
pub fn mesh_query_begin(m: &mut Mem, room: u32, words: [u32; 4], count: u32, owner: &mut dyn MeshTraversal) -> u32 {
    for (i, w) in words.iter().enumerate() {
        m.set_u32(BOX + 4 * i as u32, *w);
    }
    m.set_u32(COUNT, 0);
    owner.traverse(m, room);
    let c = m.u32(COUNT);
    m.set_u32(count, c);
    NODE_LIST
}

/// `0x5818b0(flags)` (thiscall): `+0x37c |= flags`, `+0x388 = 0x4e27a5()` (the frame length of the clock).
pub fn contact_flags_set(m: &mut Mem, this: u32, flags: u32) {
    let w = m.u32(this + 0x37c) | flags;
    m.set_u32(this + 0x37c, w);
    let v = crate::globals::time(m);
    m.set_f32(this + 0x388, v);
}

/// `0x5f25c0(out)` (thiscall, `this` = node): the node's matrix `+0x40` composed along the parent chain `[+0x2c]`. A node with a
/// parent is switched to mode 2 (`+0x38 = 2`). Mode 0: `out = matrix`, result 0; mode 1: `out = matrix`, result 1; mode 2
/// without parent as mode 0, else `out = matrix * (parent's world matrix)` and the parent's result; other modes leave `out` alone
/// and return 0.
pub fn node_world_matrix(m: &mut Mem, node: u32, out: u32) -> u32 {
    let parent = m.u32(node + 0x2c);
    if parent != 0 {
        m.set_u32(node + 0x38, 2);
    }
    let copy = |m: &mut Mem| {
        let b = m.read(node + 0x40, 48);
        m.write(out, &b);
    };
    match m.u32(node + 0x38) {
        0 => {
            copy(m);
            0
        }
        1 => {
            copy(m);
            1
        }
        2 => {
            if parent == 0 {
                copy(m);
                return 0;
            }
            let r = node_world_matrix(m, parent, out);
            let tmp = 0x7ffd_0000;
            crate::matrix::mat_mul(m, tmp, node + 0x40, out);
            let b = m.read(tmp, 48);
            m.write(out, &b);
            r
        }
        _ => 0,
    }
}

/// The virtuals the collector calls on scene nodes (the graph's classes are not ported here: stand-in owner).
pub trait CollectOwner {
    /// `[child->vtable + 0x20](0, 0x56f450)`: walks the child with the collector (type 0x11 nodes).
    fn traverse(&mut self, m: &mut Mem, child: u32);
    /// `[node->vtable + 0x3c]()`: the number of children (type 0x12 nodes).
    fn count(&mut self, m: &mut Mem, node: u32) -> i32;
}

/// The cache `0x6f34c4`: the parent node of the type 0x11 group being walked.
pub const GROUP_CACHE: u32 = 0x6f34c4;

/// `0x56f450(node)` (cdecl): the collector callback. Returns -1, or for a type 0x12 node the child index to visit. A node with
/// flag bit 0 or with 30 nodes already collected is skipped. A kind-9 node named `sharp...` whose `+0x40` is 7 is collected into
/// the list `0x6f2e58` (with the parent node it hangs on in `0x6f326c`) when the distance of its bounding centre (`+0x48`, taken
/// through the world matrix of that parent) from the box centre `0x6f3248` is at most the box radius plus its own radius
/// (`[+0x3c]`). A type 0x11 group (flag bit 3 clear) walks its child with the group cached in `0x6f34c4`.
pub fn mesh_collect(m: &mut Mem, node: u32, owner: &mut dyn CollectOwner) -> u32 {
    use crate::math::vec_len;
    if m.u32(node + 0x28) & 1 != 0 || m.i32(COUNT) >= 0x1e {
        return u32::MAX;
    }
    let ty = m.u32(node + 0x10);
    if ty == 9 {
        let name = m.u32(node + 0xc);
        let sharp = name != 0 && (0..5).all(|i| m.rd8(name + i) == b"sharp"[i as usize]);
        if sharp && m.u32(node + 0x40) == 7 {
            let (mat, vec) = (0x7ffc_0000u32, 0x7ffc_0100u32);
            for i in 0..12 {
                let w = m.u32(crate::globals::MAT_IDENTITY + 4 * i);
                m.set_u32(mat + 4 * i, w);
            }
            let shape = m.u32(node + 0x2c);
            let mut parent = 0;
            if m.u32(shape + 0x10) == 4 {
                parent = m.u32(shape + 0x3c);
                if parent != 0 {
                    node_world_matrix(m, parent, mat);
                }
            } else {
                parent = m.u32(GROUP_CACHE);
                if parent != 0 {
                    node_world_matrix(m, parent, mat);
                }
            }
            let r = m.f32(m.u32(node + 0x3c)) as f64;
            let c = m.u32(node + 0x48);
            let g = |m: &Mem, i: u32| m.f32(mat + 4 * i) as f64;
            let cv = [m.f32(c) as f64, m.f32(c + 4) as f64, m.f32(c + 8) as f64];
            let x = ((g(m, 0) * cv[0] + g(m, 6) * cv[2]) + g(m, 3) * cv[1]) + g(m, 9);
            let y = (((g(m, 1) * cv[0] + g(m, 7) * cv[2]) + g(m, 4) * cv[1]) + g(m, 10)) as f32;
            let z = (((g(m, 2) * cv[0] + g(m, 8) * cv[2]) + g(m, 5) * cv[1]) + g(m, 11)) as f32;
            let d = [(x - m.f32(BOX) as f64) as f32, (y as f64 - m.f32(BOX + 4) as f64) as f32, (z as f64 - m.f32(BOX + 8) as f64) as f32];
            for (i, v) in d.iter().enumerate() {
                m.set_f32(vec + 4 * i as u32, *v);
            }
            let l = vec_len(m, vec);
            let reach = m.f32(BOX + 12) as f64 + r;
            if !(l > reach) {
                let n = m.u32(COUNT);
                m.set_u32(NODE_LIST + 4 * n, node);
                m.set_u32(NODE_EXTRA + 4 * n, parent);
                m.set_u32(COUNT, n + 1);
            }
            return u32::MAX;
        }
    }
    if ty == 0x11 {
        if m.u32(node + 0x28) & 8 != 0 {
            return u32::MAX;
        }
        let w = m.u32(node + 0x44);
        m.set_u32(GROUP_CACHE, w);
        let child = m.u32(node + 0x40);
        owner.traverse(m, child);
        m.set_u32(GROUP_CACHE, 0);
        return u32::MAX;
    }
    if ty == 0x12 {
        let mut idx = m.i32(node + 0x38);
        let link = m.u32(node + 0x3c);
        if link != 0 {
            idx = m.i32(link + 0x38);
        }
        if !(idx < owner.count(m, node)) {
            idx = owner.count(m, node) - 1;
        }
        return idx as u32;
    }
    u32::MAX
}
#[cfg(test)]
mod tests {
    use super::*;
    fn unhex(s: &str) -> Vec<u8> {
        (0..s.len()).step_by(2).map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap()).collect()
    }
    #[test]
    fn native_mesh_query() {
        let base = 0x2a010000u32;
        let out = base + 0x10000;
        let mut counts = (0, 0, 0);
        for (c, line) in include_str!("../../../tools/emu/golden_mesh_query.tsv").lines().enumerate() {
            let v: Vec<&str> = line.split('\t').collect();
            let mut m = Mem::new();
            match v[0] {
                "acc" => {
                    let idx: u32 = v[1].parse().unwrap();
                    let node = base + 0x400 * (c as u32 % 8);
                    m.write(node, &unhex(v[4]));
                    m.set_u32(NODE_LIST + 4 * idx, node);
                    m.write(NODE_EXTRA + 4 * idx, &unhex(v[5]));
                    let outs = [0, 1, 2, 3, 4, 5].map(|k| out + 0x10 * k);
                    let r = mesh_node_info(&mut m, idx, outs);
                    assert_eq!(format!("{r:x}"), v[6], "case {c}: result");
                    let got: String = m.read(out, 0x60).iter().map(|b| format!("{b:02x}")).collect();
                    assert_eq!(got, v[7], "case {c}: outputs");
                    counts.0 += 1;
                }
                "beg" => {
                    struct T(Vec<String>);
                    impl MeshTraversal for T {
                        fn traverse(&mut self, m: &mut Mem, room: u32) {
                            let b: String = m.read(BOX, 16).iter().map(|b| format!("{b:02x}")).collect();
                            self.0.push(format!("{room:x},0,{b},{:08x}", m.u32(COUNT).swap_bytes()));
                            m.set_u32(COUNT, 7);
                        }
                    }
                    let words: Vec<u32> = unhex(v[1]).chunks(4).map(|c| u32::from_le_bytes(c.try_into().unwrap())).collect();
                    m.set_u32(COUNT, 99);
                    let mut t = T(vec![]);
                    let room = base + 0x20000;
                    let r = mesh_query_begin(&mut m, room, [words[0], words[1], words[2], words[3]], out, &mut t);
                    assert_eq!(format!("{r:x}"), v[2], "case {c}");
                    let got: String = m.read(out, 4).iter().map(|b| format!("{b:02x}")).collect();
                    assert_eq!(got, v[3], "case {c}: count");
                    let boxes: String = m.read(BOX, 16).iter().map(|b| format!("{b:02x}")).collect();
                    assert_eq!(boxes, v[4], "case {c}: box");
                    counts.1 += 1;
                }
                _ => {
                    let this = base;
                    m.write(0x6974c2, &unhex(v[5]));
                    m.write(0x6974bc, &unhex(v[6]));
                    m.set_u32(0x6974c4, base + 0x2000);
                    m.write(base + 0x2000 + 0xb98, &unhex(v[7]));
                    m.write(this + 0x37c, &unhex(v[2]));
                    contact_flags_set(&mut m, this, u32::from_str_radix(v[1], 16).unwrap());
                    let hex = |a: u32| m.read(a, 4).iter().map(|b| format!("{b:02x}")).collect::<String>();
                    assert_eq!(hex(this + 0x37c), v[3], "case {c}: flags");
                    assert_eq!(hex(this + 0x388), v[4], "case {c}: time");
                    counts.2 += 1;
                }
            }
        }
        assert_eq!(counts, (120, 60, 60));
    }

    #[test]
    fn native_node_world_matrix() {
        let base = 0x2a010000u32;
        let out = base + 0x10000;
        let mut n = 0;
        for (c, line) in include_str!("../../../tools/emu/golden_node_matrix.tsv").lines().enumerate() {
            let v: Vec<&str> = line.split('\t').collect();
            let len: usize = v[0].parse().unwrap();
            let mut m = Mem::new();
            let node = |i: usize| base + 0x200 * i as u32;
            for (i, d) in v[1].split(';').enumerate() {
                let (mode, mat) = d.split_once(',').unwrap();
                m.set_u32(node(i) + 0x38, mode.parse().unwrap());
                m.write(node(i) + 0x40, &unhex(mat));
                m.set_u32(node(i) + 0x2c, if i + 1 < len { node(i + 1) } else { 0 });
            }
            let r = node_world_matrix(&mut m, node(0), out);
            assert_eq!(format!("{r:x}"), v[2], "case {c}: result");
            let got: String = m.read(out, 0x30).iter().map(|b| format!("{b:02x}")).collect();
            assert_eq!(got, v[3], "case {c}: matrix");
            let modes: Vec<String> = (0..len).map(|i| m.u32(node(i) + 0x38).to_string()).collect();
            assert_eq!(modes.join(","), v[4], "case {c}: modes");
            n += 1;
        }
        assert_eq!(n, 200);
    }

    #[test]
    fn native_mesh_collect() {
        struct Ev(Vec<String>, i32);
        impl CollectOwner for Ev {
            fn traverse(&mut self, m: &mut Mem, child: u32) {
                self.0.push(format!("t|{child:x}|56f450|{:x}", m.u32(GROUP_CACHE) & 0xff));
            }
            fn count(&mut self, _: &mut Mem, node: u32) -> i32 {
                self.0.push(format!("c|{node:x}"));
                self.1
            }
        }
        let base = 0x2a010000u32;
        let at = |i: u32| base + 0x400 * i;
        let (node, own, chain2, name, bnd, cen, child) = (at(0), at(1), at(3), at(4), at(5), at(6), at(7));
        let mut n = 0;
        for (c, line) in include_str!("../../../tools/emu/golden_mesh_collect.tsv").lines().enumerate() {
            let v: Vec<&str> = line.split('\t').collect();
            let mut m = Mem::new();
            crate::globals::install_constants(&mut m);
            let put = |m: &mut Mem, at: u32, list: &str| {
                for it in list.split(',').filter(|s| !s.is_empty()) {
                    let (o, x) = it.split_once(':').unwrap();
                    m.set_u32(at + u32::from_str_radix(o, 16).unwrap(), u32::from_str_radix(x, 16).unwrap());
                }
            };
            put(&mut m, node, v[0]);
            put(&mut m, own, v[1]);
            m.write(own + 0x40, &unhex(v[2]));
            m.set_u32(at(2) + 0x38, 3);
            m.set_u32(chain2 + 0x38, u32::from_str_radix(v[3], 16).unwrap());
            m.write(chain2 + 0x40, &unhex(v[4]));
            let nm = unhex(v[5]);
            m.write(name, &nm);
            m.wr8(name + nm.len() as u32, 0);
            m.write(bnd, &unhex(v[6]));
            m.write(cen, &unhex(v[7]));
            m.write(BOX, &unhex(v[8]));
            m.set_u32(COUNT, v[9].parse().unwrap());
            m.set_u32(GROUP_CACHE, u32::from_str_radix(v[10], 16).unwrap());
            let _ = child;
            let mut o = Ev(vec![], v[11].parse().unwrap());
            let r = mesh_collect(&mut m, node, &mut o);
            let hex = |a: u32, k: usize| m.read(a, k).iter().map(|b| format!("{b:02x}")).collect::<String>();
            assert_eq!(format!("{r:x}"), v[12], "case {c}: result");
            assert_eq!(hex(NODE_LIST, 32), v[13], "case {c}: list");
            assert_eq!(hex(NODE_EXTRA, 32), v[14], "case {c}: extra");
            assert_eq!(hex(COUNT, 4), v[15], "case {c}: count");
            assert_eq!(hex(GROUP_CACHE, 4), v[16], "case {c}: cache");
            assert_eq!(o.0.join(";"), v[17], "case {c}: events");
            n += 1;
        }
        assert_eq!(n, 400);
    }}

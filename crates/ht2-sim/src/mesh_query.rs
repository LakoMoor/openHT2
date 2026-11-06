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
}



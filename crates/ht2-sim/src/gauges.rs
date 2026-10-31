//! `0x5553e0` (`__fastcall`, `ecx` = vehicle scene): the gauge change check at the start of the scene update. When
//! `[scene + 0x51a4] & 0x80` it picks the gauge level from the counters of the gauge object `[scene + 0x2540]`
//! (`0x5893e0`), compares the nine gauge words (`+0x523c`, against `object + 4 + 0x14 * i`) and the eight words
//! `+0x5260` against `+0x5280`, and tells the network (`0x546240`, `changed`) when something differs; then it stores
//! the new words.

use crate::mem::Mem;

/// `0x5893e0` (thiscall on the gauge object): set level `idx` (`0 <= idx <= [this + 8]`) and its ratio `[this + 0xc]`.
pub fn gauge_set_level(m: &mut Mem, this: u32, idx: i32) {
    let max = m.i32(this + 8);
    if idx < 0 || idx > max {
        return;
    }
    m.set_i32(this + 4, idx);
    if max == 0 {
        m.set_u32(this + 0xc, 0xffc0_0000); // 0 / 0 on the x87: the real indefinite
    } else {
        m.set_f32(this + 0xc, (idx as f64 / max as f64) as f32);
    }
}

pub fn scene_gauges(m: &mut Mem, scene: u32, changed: &mut impl FnMut(&mut Mem, u32)) {
    if m.rd8(scene + 0x51a4) & 0x80 == 0 {
        return;
    }
    let obj = m.u32(scene + 0x2540);
    let (a, b, c) = (m.i32(obj + 0x2c), m.i32(obj + 0x40), m.i32(obj + 4));
    let level = if a >= 2 && b >= 2 {
        4
    } else if a >= 1 && b >= 1 {
        3
    } else if a >= 1 && c < 3 {
        2
    } else if b >= 1 && c < 3 {
        1
    } else if m.i32(obj + 0x18) > 0 {
        3
    } else {
        0
    };
    gauge_set_level(m, obj, level);
    let obj = m.u32(scene + 0x2540);
    for i in 0..9u32 {
        if m.u32(scene + 0x523c + 4 * i) != m.u32(obj + 4 + 0x14 * i) {
            m.set_u32(scene + 0x4f50, 1);
            break;
        }
    }
    for i in 0..8u32 {
        if m.u32(scene + 0x5260 + 4 * i) != m.u32(scene + 0x5280 + 4 * i) {
            m.set_u32(scene + 0x4f50, 1);
            break;
        }
    }
    if m.u32(scene + 0x4f50) != 0 {
        changed(m, scene);
    }
    for i in 0..9u32 {
        m.set_u32(scene + 0x523c + 4 * i, m.u32(obj + 4 + 0x14 * i));
    }
    for i in 0..8u32 {
        m.set_u32(scene + 0x5260 + 4 * i, m.u32(scene + 0x5280 + 4 * i));
    }
    if m.u32(scene + 0x4f50) != 0 {
        m.set_u32(scene + 0x4f50, 0);
        changed(m, scene);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn bytes(s: &str) -> Vec<u8> {
        (0..s.len()).step_by(2).map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap()).collect()
    }
    #[test]
    fn native_scene_gauges() {
        let (scene, obj) = (0x2a01_0000u32, 0x2a03_0000u32);
        let areas = [(0x2540u32, 0x2544u32), (0x4f50, 0x4f54), (0x51a4, 0x51a5), (0x523c, 0x52a0)];
        for (c, line) in include_str!("../../../tools/emu/golden_scene_gauges.tsv").lines().enumerate() {
            let v: Vec<&str> = line.trim_end_matches(['\r', '\n']).split('\t').collect();
            let mut m = Mem::new();
            let mut at = 0;
            let before = bytes(v[0]);
            for (a, b) in areas {
                m.write(scene + a, &before[at..at + (b - a) as usize]);
                at += (b - a) as usize;
            }
            m.write(obj, &bytes(v[1]));
            let mut calls = Vec::new();
            scene_gauges(&mut m, scene, &mut |m, s| calls.push(hex_u32(m.u32(s + 0x4f50))));
            assert_eq!(calls.join(","), v[2], "case {c}: calls");
            let mut after = Vec::new();
            for (a, b) in areas {
                after.extend(m.read(scene + a, (b - a) as usize));
            }
            assert_eq!(after, bytes(v[3]), "case {c}: scene");
            assert_eq!(m.read(obj, 0xc0), bytes(v[4]), "case {c}: gauge object");
        }
    }
    fn hex_u32(x: u32) -> String {
        x.to_le_bytes().iter().map(|b| format!("{:02x}", b)).collect()
    }
}

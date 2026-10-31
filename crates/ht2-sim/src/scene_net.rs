//! `0x546360` (`__thiscall`, ecx = vehicle scene): the network message with the effect request slots. Builds the
//! payload `[+0x4b4] (u32), 7 (u32)` followed by 11 records of 0x11 bytes (the flag byte `+0x5468 + 4i`, the position
//! `+0x5494 + 12i`, the level `+0x5518 + 4i`), 0xc3 bytes in all, and hands it to the net stream
//! (`0x602be0` construct, `0x603110` write on the channel `(i16)[0x67d10a]`, `0x5c3630` flush on `[0x6ef1fc]`,
//! `0x602e80` destroy), which is an external owner here (`NetOwner::send`).
use crate::mem::Mem;

pub trait NetOwner {
    /// The stream sequence above for one message.
    fn send(&mut self, m: &mut Mem, channel: i32, payload: &[u8]);
}

pub fn scene_effect_message(m: &mut Mem, s: u32, owner: &mut dyn NetOwner) {
    if m.u32(s + 0x4c4) == 0 || m.u32(s + 0x184) != 0 {
        return;
    }
    let mut p: Vec<u8> = Vec::with_capacity(0xc3);
    p.extend_from_slice(&m.u32(s + 0x4b4).to_le_bytes());
    p.extend_from_slice(&7u32.to_le_bytes());
    for i in 0..11u32 {
        p.push(m.rd8(s + 0x5468 + 4 * i));
        p.extend_from_slice(&m.read(s + 0x5494 + 12 * i, 12));
        p.extend_from_slice(&m.read(s + 0x5518 + 4 * i, 4));
    }
    let channel = m.i16(0x67d10a) as i32;
    owner.send(m, channel, &p);
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Rec(Option<(i32, Vec<u8>)>);
    impl NetOwner for Rec {
        fn send(&mut self, _m: &mut Mem, channel: i32, payload: &[u8]) {
            self.0 = Some((channel, payload.to_vec()));
        }
    }
    fn unhex(s: &str) -> Vec<u8> {
        (0..s.len()).step_by(2).map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap()).collect()
    }
    #[test]
    fn native_scene_effect_message() {
        let scene = 0x2a010000u32;
        let mut n = 0;
        let mut sent = 0;
        for (c, line) in include_str!("../../../tools/emu/golden_scene_net.tsv").lines().enumerate() {
            let v: Vec<&str> = line.split('\t').collect();
            let mut m = Mem::new();
            m.write(scene + 0x4b0, &unhex(v[0]));
            m.write(scene + 0x4c0, &unhex(v[1]));
            m.write(scene + 0x180, &unhex(v[2]));
            m.write(scene + 0x5468, &unhex(v[3]));
            m.set_i16(0x67d10a, v[4].parse().unwrap());
            let mut r = Rec(None);
            scene_effect_message(&mut m, scene, &mut r);
            match (&r.0, v[5]) {
                (None, "") => {}
                (Some((ch, p)), exp) => {
                    let (a, b) = exp.split_once(':').unwrap();
                    assert_eq!(ch.to_string(), a, "case {c}: channel");
                    assert_eq!(*p, unhex(b), "case {c}: payload");
                    sent += 1;
                }
                (None, _) => panic!("case {c}: no message"),
            }
            n += 1;
        }
        assert_eq!(n, 200);
        assert!(sent > 50);
    }
}

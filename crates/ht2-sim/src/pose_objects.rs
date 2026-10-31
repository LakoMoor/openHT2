//! Two small pose accessors of the engine objects `scene_return_to_last` / `vehicle_reset` talk to.
use crate::math::mat_inverse;
use crate::mem::Mem;

/// `0x561d10` (`__thiscall`, `ret 8`): `this[0..12] = pose`, `this[12..24] = inverse(pose)` (`0x5ac2e0`, computed
/// from `this`), `this[0x18] = id`. `scratch` needs 48 bytes.
pub fn pose_set(m: &mut Mem, this: u32, id: u32, pose: u32, scratch: u32) {
    for i in 0..12 {
        let v = m.u32(pose + 4 * i);
        m.set_u32(this + 4 * i, v);
    }
    let inv = mat_inverse(m, scratch, this);
    for i in 0..12 {
        let v = m.u32(inv + 4 * i);
        m.set_u32(this + 0x30 + 4 * i, v);
    }
    m.set_u32(this + 0x60, id);
}

/// `0x5f25a0` (`__thiscall`): copy the 12 words at `this + 0x40` to `out`.
pub fn pose_get(m: &mut Mem, this: u32, out: u32) {
    for i in 0..12 {
        let v = m.u32(this + 0x40 + 4 * i);
        m.set_u32(out + 4 * i, v);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn unhex(s: &str) -> Vec<u8> {
        (0..s.len()).step_by(2).map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap()).collect()
    }
    #[test]
    fn native_pose_objects() {
        let (this, pose, out, scratch) = (0x2a010000u32, 0x2a020000u32, 0x2a030000u32, 0x2a040000u32);
        let mut n = 0;
        for (c, line) in include_str!("../../../tools/emu/golden_pose_objects.tsv").lines().enumerate() {
            let v: Vec<&str> = line.split('\t').collect();
            let mut m = Mem::new();
            m.write(this, &unhex(v[1]));
            m.write(pose, &unhex(v[2]));
            if v[0] == "set" {
                pose_set(&mut m, this, u32::from_str_radix(v[3], 16).unwrap(), pose, scratch);
                assert_eq!(m.read(this, 0x64), unhex(v[4]), "case {c}");
            } else {
                pose_get(&mut m, this, out);
                assert_eq!(m.read(out, 48), unhex(v[4]), "case {c}");
            }
            n += 1;
        }
        assert_eq!(n, 300);
    }
}

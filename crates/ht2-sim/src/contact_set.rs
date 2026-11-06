//! `0x587c80` (`__thiscall`, ecx = the contact set of a body, `0x1910` bytes): the constructor. Two arrays of 64 vectors
//! (`+0x0`, `+0x300`) are set to the constant vector `0x702650`; per entry `i` the words `+0x600`, `+0x700`, `+0x800`, `+0x900`,
//! `+0xa00`, `+0xb04`, `+0xc04` (`+4i`) are cleared and the 48-byte matrix `+0xd04 + 0x30 i` is set to the identity
//! `0x7025f0`; the counter `+0xb00` ends as 0. (The vector constructor loop `0x4011e0(.., 0x4bd4d0)` zeroes the matrix
//! area first; the identity copy overwrites it.)
use crate::globals::{MAT_IDENTITY, VEC_ZERO};
use crate::mem::Mem;

pub fn contact_set_ctor(m: &mut Mem, b: u32) {
    for i in 0..0x40u32 {
        for k in 0..3 {
            let v = m.u32(VEC_ZERO + 4 * k);
            m.set_u32(b + 12 * i + 4 * k, 0);
            m.set_u32(b + 0x300 + 12 * i + 4 * k, 0);
            m.set_u32(b + 12 * i + 4 * k, v);
            m.set_u32(b + 12 * (i + 0x40) + 4 * k, v);
        }
    }
    for i in 0..0x40u32 {
        m.set_u32(b + 0xb00, i);
        for o in [0x600, 0x700, 0x800, 0x900, 0xa00, 0xb04, 0xc04] {
            m.set_u32(b + o + 4 * i, 0);
        }
        for k in 0..12 {
            let v = m.u32(MAT_IDENTITY + 4 * k);
            m.set_u32(b + 0xd04 + 0x30 * i + 4 * k, v);
        }
    }
    m.set_u32(b + 0xb00, 0);
}

#[cfg(test)]
mod tests {
    use super::*;
    fn unhex(s: &str) -> Vec<u8> {
        (0..s.len()).step_by(2).map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap()).collect()
    }
    #[test]
    fn native_contact_set_ctor() {
        let b = 0x2a010000u32;
        let mut n = 0;
        for (c, line) in include_str!("../../../tools/emu/golden_contact_set_ctor.tsv").lines().enumerate() {
            let v: Vec<&str> = line.split('\t').collect();
            let mut m = Mem::new();
            m.write(VEC_ZERO, &unhex(v[0]));
            m.write(MAT_IDENTITY, &unhex(v[1]));
            let mut want = unhex(v[2]);
            m.write(b, &want);
            contact_set_ctor(&mut m, b);
            for it in v[3].split(',').filter(|s| !s.is_empty()) {
                let (o, x) = it.split_once(':').unwrap();
                let o = usize::from_str_radix(o, 16).unwrap();
                want[o..o + 4].copy_from_slice(&unhex(x));
            }
            assert_eq!(m.read(b, 0x1910), want, "case {c}");
            n += 1;
        }
        assert_eq!(n, 20);
    }
}

//! `0x555500` (`__fastcall`, ecx = vehicle scene): the level the gauge message carries. Zero without a car object
//! `[+0x5460]`; otherwise `0x546a30` builds a record and `0x542530` weighs it with the 36 floats at `+0x28c0`:
//! `sum |rec[i]| * w[i]` for the nine words `[[+0x2540] + 0xc + 0x14 i]` and `sum |rec[14 + k]| * w[9 + k]` for the
//! 27 words `[car + 0x3cac + 0x14 k]` (`0x501480`). The other record entries (four floats `+0x24` of the objects at
//! `+0x52a0`.., the virtual `[vtable+0xb8]`, the tail of `0x5014b0`) are built but never weighed, so they are
//! not computed here. The sums are kept in extended precision by the original (`f64` here, as everywhere).
use crate::mem::Mem;

pub fn scene_level(m: &Mem, s: u32) -> f64 {
    let car = m.u32(s + 0x5460);
    if car == 0 {
        return 0.0;
    }
    let obj = m.u32(s + 0x2540);
    let w = |i: u32| m.f32(s + 0x28c0 + 4 * i) as f64;
    let mut sum = 0.0f64;
    for i in 0..9u32 {
        let a = m.f32(obj + 0xc + 0x14 * i) as f64;
        sum += a.abs() * w(i);
    }
    for k in 0..27u32 {
        let b = m.f32(car + 0x3cac + 0x14 * k) as f64;
        sum += b.abs() * w(9 + k);
    }
    sum
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_scene_level() {
        let unhex = |s: &str| -> Vec<u8> { (0..s.len()).step_by(2).map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap()).collect() };
        let (scene, obj, car) = (0x2a010000u32, 0x2a020000u32, 0x2a030000u32);
        let mut n = 0;
        for (c, line) in include_str!("../../../tools/emu/golden_scene_level.tsv").lines().enumerate() {
            let v: Vec<&str> = line.split('\t').collect();
            let mut m = Mem::new();
            m.set_u32(scene + 0x5460, if v[0] == "1" { car } else { 0 });
            m.set_u32(scene + 0x2540, obj);
            m.write(scene + 0x28c0, &unhex(v[1]));
            m.write(obj, &unhex(v[2]));
            m.write(car + 0x3c00, &unhex(v[3]));
            let got = scene_level(&m, scene);
            let want = f64::from_bits(u64::from_str_radix(v[4], 16).unwrap());
            assert_eq!(got.to_bits(), want.to_bits(), "case {c}: {got} vs {want}");
            n += 1;
        }
        assert_eq!(n, 300);
    }
}

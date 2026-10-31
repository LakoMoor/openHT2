//! `0x580dd0` (`__fastcall`, ecx = vehicle object): the object's reset step. With `[+0x4a8]` and `[+0x37c]` both set
//! it only clears `[+0x4b0]` (and calls the virtual `[vtable+0xcc]` while `[+0x174] = 0`); otherwise it resets the
//! state (`+0x32e` from the world record `[0x6ef1fc]+0xbb0`, the vectors `+0x1dc`/`+0x1e8` from `0x702650`, the
//! virtual `[vtable+0x4c]`, `0x5d32f0(this, 1)`) and, with the debug flag `[0x696c6c]`, writes three log lines
//! (`0x4ea27e`). The virtual calls, `0x5d32f0`, `0x5f25a0` and the log are external (`ResetOwner`).
use crate::globals::{MAT_IDENTITY, VEC_ZERO};
use crate::mem::Mem;

pub trait ResetOwner {
    /// A virtual call `[this->vtable + slot]()`.
    fn virt(&mut self, m: &mut Mem, this: u32, slot: u32);
    /// A direct call of `address` (`0x5d32f0`, `0x5f25a0`).
    fn call(&mut self, m: &mut Mem, address: u32, this: u32, args: &[u32]);
    /// `0x4ea27e(text)`.
    fn log(&mut self, m: &mut Mem, text: &str);
}

fn c_string(m: &Mem, p: u32) -> String {
    (0..).map(|i| m.rd8(p + i)).take_while(|&b| b != 0).map(|b| b as char).collect()
}

/// `f` is a scratch area for the 48-byte matrix the original keeps on its stack. The log reads three floats at
/// `f+0x28` (the callee `0x5f25a0` leaves data past the 12 matrix words; the original prints stack contents there).
pub fn vehicle_reset(m: &mut Mem, b: u32, f: u32, owner: &mut dyn ResetOwner) {
    if m.u32(b + 0x4a8) != 0 && m.u32(b + 0x37c) != 0 {
        m.set_u32(b + 0x4b0, 0);
        if m.u32(b + 0x174) == 0 {
            owner.virt(m, b, 0xcc);
        }
        return;
    }
    if m.u32(b + 0x174) == 0 {
        let rec = m.u32(0x6ef1fc);
        let v = m.u32(rec + 0xbb0);
        m.set_u16(b + 0x32e, v as u16);
    }
    for i in 0..3 {
        let v = m.u32(VEC_ZERO + 4 * i);
        m.set_u32(b + 0x1dc + 4 * i, v);
        m.set_u32(b + 0x1e8 + 4 * i, v);
    }
    m.set_u32(b + 0x37c, 0);
    owner.virt(m, b, 0x4c);
    m.set_u32(b + 0x4a8, 0);
    m.set_u32(b + 0x4b0, 1);
    owner.call(m, 0x5d32f0, b, &[1]);
    if m.u32(0x696c6c) == 0 {
        return;
    }
    for i in 0..12 {
        let v = m.u32(MAT_IDENTITY + 4 * i);
        m.set_u32(f + 4 * i, v);
    }
    let sub = m.u32(m.u32(b + 0x20) + 0x44);
    owner.call(m, 0x5f25a0, sub, &[f]);
    owner.log(m, "run");
    let name = c_string(m, m.u32(m.u32(b + 0x140) + 0xc));
    let t = [m.f32(f + 0x28) as f64, m.f32(f + 0x2c) as f64, m.f32(f + 0x30) as f64];
    owner.log(m, &format!("  {}_M0 {:6.3} {:6.3} {:6.3} ", name, t[0], t[1], t[2]));
    let a = [m.f32(b + 0x218) as f64, m.f32(b + 0x21c) as f64, m.f32(b + 0x220) as f64];
    owner.log(m, &format!("  {}_M {:6.3} {:6.3} {:6.3} ", name, a[0], a[1], a[2]));
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Replay {
        ev: Vec<String>,
        pos: usize,
    }
    fn unhex(s: &str) -> Vec<u8> {
        (0..s.len()).step_by(2).map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap()).collect()
    }
    impl Replay {
        fn next(&mut self) -> Vec<String> {
            let e = self.ev[self.pos].split('|').map(String::from).collect();
            self.pos += 1;
            e
        }
    }
    impl ResetOwner for Replay {
        fn virt(&mut self, _m: &mut Mem, this: u32, slot: u32) {
            assert_eq!(self.next().join("|"), format!("v|{slot:x}|{this:x}"));
        }
        fn call(&mut self, m: &mut Mem, address: u32, this: u32, args: &[u32]) {
            let e = self.next();
            assert_eq!(&e[..3], &["c".to_string(), format!("{address:x}"), format!("{this:x}")]);
            if address == 0x5f25a0 {
                m.write(args[0], &unhex(&e[3]));
            } else {
                assert_eq!(args, &[1]);
            }
        }
        fn log(&mut self, _m: &mut Mem, text: &str) {
            let e = self.next();
            assert_eq!(e[0], "l");
            assert_eq!(e[1], text);
        }
    }
    #[test]
    fn native_vehicle_reset() {
        let hex = |s: &str| u32::from_str_radix(s, 16).unwrap();
        let (b, f, aux, world) = (0x2a010000u32, 0x2a0f0000u32, 0x2a020000u32, 0x2a030000u32);
        let mut n = 0;
        for (c, line) in include_str!("../../../tools/emu/golden_vehicle_reset.tsv").lines().enumerate() {
            let v: Vec<&str> = line.split('\t').collect();
            let mut m = Mem::new();
            m.write(0x702650, &unhex(v[0]));
            let ident: [f32; 12] = [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0];
            for (i, x) in ident.iter().enumerate() {
                m.set_f32(MAT_IDENTITY + 4 * i as u32, *x);
            }
            for it in v[1].split(',') {
                let (o, x) = it.split_once(':').unwrap();
                m.set_u32(b + hex(o), hex(x));
            }
            m.write(aux, &unhex(v[2]));
            m.set_u32(0x6ef1fc, world);
            m.write(world + 0xbb0, &unhex(v[5]));
            m.set_u32(0x696c6c, hex(v[6]));
            let mut r = Replay { ev: v[3].split(';').filter(|s| !s.is_empty()).map(String::from).collect(), pos: 0 };
            vehicle_reset(&mut m, b, f, &mut r);
            assert_eq!(r.pos, r.ev.len(), "case {c}: events");
            for it in v[4].split(',') {
                let (o, x) = it.split_once(':').unwrap();
                assert_eq!(m.u32(b + hex(o)), hex(x), "case {c}: +{o}");
            }
            n += 1;
        }
        assert_eq!(n, 300);
    }
}

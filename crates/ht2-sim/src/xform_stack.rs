//! The scene transform stack: `0x5ae580` (push) and `0x5ae8a0` (pop). Object layout: matrices `+0x1490` (stack A, 48 bytes each, count
//! `+0x202c`), `+0x8f0` (B, count `+0x2020`), inverse-transposes `+0xcb0`; constants `+0x1070`, `+0x10a0 + 0x30 * [+0x2024]`;
//! `+0x20fc` = the global generation counter `0x67ef88` after each change. Statics: `0x702a48`, `0x702a10`.

use crate::mem::Mem;

const G48: u32 = 0x702a48;
const G10: u32 = 0x702a10;
const DBG_ON: u32 = 0x720904;
const DBG_OBJ: u32 = 0x720908;
const GEN: u32 = 0x67ef88;

/// What the stack calls outside itself (stand-in): the debug object `[0x720908]`, virtual `+0x2c(obj, 1, matrix4x4)`, used only when
/// `[0x720904] != 0`; its result is checked only by the push (non-zero: error report `0x5d2ba0(0x67ef94, 0)`).
pub trait StackDebug {
    fn debug(&mut self, m: &mut Mem, obj: u32, m16: u32) -> u32;
    fn error(&mut self);
}

fn copy12(m: &mut Mem, dst: u32, src: u32) {
    for i in 0..12 {
        let w = m.u32(src + 4 * i);
        m.set_u32(dst + 4 * i, w);
    }
}

/// The 4x4 handed to the debug object: the 12 words of the top matrix, a zero column and a 1.0 in the last row.
fn debug_call(m: &mut Mem, this: u32, o: &mut dyn StackDebug) -> u32 {
    const L: u32 = 0x7ffe_f000 + 0xb800;
    let top = this + 0x1460 + 0x30 * m.u32(this + 0x202c);
    for r in 0..4u32 {
        for c in 0..3u32 {
            let w = m.u32(top + 4 * (3 * r + c));
            m.set_u32(L + 16 * r + 4 * c, w);
        }
        m.set_u32(L + 16 * r + 12, if r == 3 { 0x3f80_0000 } else { 0 });
    }
    let obj = m.u32(DBG_OBJ);
    o.debug(m, obj, L)
}

/// `0x5ae580(matrix, kind)` (thiscall): with both counts below 18 the matrix is pushed on stack A: `kind == 0` as it is, else
/// `matrix * top` (`0x5aba40`). The new top `T` gives `G48 = (T * [+0x1070]) * [+0x10a0 + 0x30 * [+0x2024]]`, which is stored at
/// `+0x8f0 + 0x30 * nB`; the inverse (`0x5ac480`, static `G10`) is stored transposed (`0x5ac5f0`) at `+0xcb0 + 0x30 * nB`; `nB += 1`, the
/// generation `0x67ef88` is incremented and written to `+0x20fc`. With the debug object the top matrix is shown to it first.
pub fn stack_push(m: &mut Mem, this: u32, mat: u32, kind: u32, o: &mut dyn StackDebug) {
    const L: u32 = 0x7ffe_f000 + 0xb600;
    let (t1, t2) = (L, L + 0x40);
    let (na, nb) = (m.i32(this + 0x202c), m.i32(this + 0x2020));
    if na >= 0x12 || nb >= 0x12 {
        return;
    }
    let c = m.u32(this + 0x2024);
    let p1 = this + 0x10a0 + 0x30 * c;
    let top_new = this + 0x1490 + 0x30 * na as u32;
    if kind == 0 {
        copy12(m, top_new, mat);
        m.set_i32(this + 0x202c, na + 1);
        crate::matrix::mat_mul(m, t1, mat, this + 0x1070);
    } else {
        crate::matrix::mat_mul(m, t1, mat, this + 0x1460 + 0x30 * na as u32);
        copy12(m, top_new, t1);
        m.set_i32(this + 0x202c, na + 1);
        crate::matrix::mat_mul(m, t1, top_new, this + 0x1070);
    }
    crate::matrix::mat_mul(m, t2, t1, p1);
    copy12(m, G48, t2);
    let nb = m.u32(this + 0x2020);
    copy12(m, this + 0x8f0 + 0x30 * nb, G48);
    if m.u32(DBG_ON) != 0 && debug_call(m, this, o) != 0 {
        o.error();
    }
    crate::math::mat_inverse_scene(m, G10, G48);
    transpose_store(m, this + 0xcb0 + 0x30 * nb, G10);
    m.set_u32(this + 0x2020, nb + 1);
    let g = m.u32(GEN).wrapping_add(1);
    m.set_u32(GEN, g);
    m.set_u32(this + 0x20fc, g);
}

/// `0x5ac5f0(dst, src)`: the 3x3 part transposed, the translation copied.
fn transpose_store(m: &mut Mem, dst: u32, src: u32) {
    const MAP: [u32; 12] = [0, 3, 6, 1, 4, 7, 2, 5, 8, 9, 10, 11];
    for (i, s) in MAP.iter().enumerate() {
        let w = m.u32(src + 4 * s);
        m.set_u32(dst + 4 * i as u32, w);
    }
}

/// `0x5ae8a0` (thiscall): with both counts above 1 they are decremented, the generation `0x67ef88` is incremented and written to
/// `+0x20fc`; with the debug object the new top matrix is shown to it.
pub fn stack_pop(m: &mut Mem, this: u32, o: &mut dyn StackDebug) {
    let (nb, na) = (m.i32(this + 0x2020), m.i32(this + 0x202c));
    if nb <= 1 || na <= 1 {
        return;
    }
    m.set_i32(this + 0x2020, nb - 1);
    m.set_i32(this + 0x202c, na - 1);
    let g = m.u32(GEN).wrapping_add(1);
    m.set_u32(GEN, g);
    m.set_u32(this + 0x20fc, g);
    if m.u32(DBG_ON) != 0 {
        debug_call(m, this, o);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn unhex(s: &str) -> Vec<u8> {
        (0..s.len()).step_by(2).map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap()).collect()
    }
    struct T {
        ev: Vec<String>,
        ret: u32,
    }
    impl StackDebug for T {
        fn debug(&mut self, m: &mut Mem, obj: u32, m16: u32) -> u32 {
            let h: String = m.read(m16, 64).iter().map(|x| format!("{x:02x}")).collect();
            self.ev.push(format!("dbg|{obj:x}|1|{h}"));
            self.ret
        }
        fn error(&mut self) {
            self.ev.push("err|67ef94|0".to_string());
        }
    }
    #[test]
    fn native_xform_stack() {
        let (obj, mat) = (0x2a010000u32, 0x2a014000u32);
        let mut n = 0;
        for (c, line) in include_str!("../../../tools/emu/golden_xform_stack.tsv").lines().enumerate() {
            let v: Vec<&str> = line.split('\t').collect();
            let mut m = Mem::new();
            for it in v[0].split(',').filter(|s| !s.is_empty()) {
                let (o, x) = it.split_once(':').unwrap();
                m.write(u32::from_str_radix(o, 16).unwrap(), &unhex(x));
            }
            let (o0, g0, c0) = (m.read(obj, 0x2200), m.read(G10, 0x70), m.read(GEN, 4));
            let mut t = T { ev: vec![], ret: v[3].parse().unwrap() };
            if v[1] == "0" {
                stack_push(&mut m, obj, mat, v[2].parse().unwrap(), &mut t);
            } else {
                stack_pop(&mut m, obj, &mut t);
            }
            let d = |a: &[u8], b: &[u8]| -> String {
                (0..a.len())
                    .step_by(4)
                    .filter(|&i| a[i..i + 4] != b[i..i + 4])
                    .map(|i| format!("{:x}:{}", i, b[i..i + 4].iter().map(|x| format!("{x:02x}")).collect::<String>()))
                    .collect::<Vec<_>>()
                    .join(",")
            };
            assert_eq!(d(&o0, &m.read(obj, 0x2200)), v[4], "case {c}: object");
            assert_eq!(d(&g0, &m.read(G10, 0x70)), v[5], "case {c}: statics");
            assert_eq!(d(&c0, &m.read(GEN, 4)), v[6], "case {c}: counter");
            assert_eq!(t.ev.join(";"), v[7], "case {c}: events");
            n += 1;
        }
        assert_eq!(n, 500);
    }
}

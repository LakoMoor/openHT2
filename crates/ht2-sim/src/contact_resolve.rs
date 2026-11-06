//! Contact resolution of two bodies (`0x57eae0(a, rec, b)`, cdecl, frame `0x16e0`).
//!
//! This is a mechanical transliteration of the original (about 2000 instructions of x87 code with every intermediate in a frame slot): each
//! instruction became one statement over the same register / stack-frame / x87-stack model, so rounding points (`f32` stores, unrounded `f64`
//! values left on the x87 stack) are those of the original. Callees are the already ported helpers; the debug log, the report `0x4e2821` and
//! the virtuals `+0x90` / `+0x94` of the bodies are stand-ins (`ResolveOwner`). The control flow is a `match` over the original branch targets.

use crate::carv::fn_004e28b0;
use crate::contact_rough::{mat_scale_rows, vec_mul};
use crate::math::{mat_inverse, vec_len};
use crate::matrix::*;
use crate::mem::Mem;
use crate::moving_item::{vec_soft_limit, ItemVirtuals};

/// Stand-ins of `0x57eae0`: the virtuals, the report `0x4e2821(vec, text, limit)` and the debug log (`0x61cca9` text, `0x4e260c`).
pub trait ResolveOwner: ItemVirtuals {
    fn report(&mut self, m: &Mem, vec: u32, text: u32, limit: f32);
    fn log(&mut self);
}

/// x87 `fcom`: condition bits C3 / C2 / C0 of the status word.
fn fcom(a: f64, b: f64) -> u32 {
    if a.is_nan() || b.is_nan() {
        0x4500
    } else if a > b {
        0
    } else if a < b {
        0x0100
    } else {
        0x4000
    }
}

#[inline(always)]
fn fadd(a: f64, b: f64) -> f64 {
    a + b
}
#[inline(always)]
fn fsub(a: f64, b: f64) -> f64 {
    a - b
}
#[inline(always)]
fn fmul(a: f64, b: f64) -> f64 {
    a * b
}
#[inline(always)]
fn fdiv(a: f64, b: f64) -> f64 {
    a / b
}

/// `0x57eae0(a, rec, b)`.
#[allow(unused_assignments, unused_variables, unused_mut, clippy::all)]
pub fn contact_resolve(m: &mut Mem, a: u32, rec: u32, b: u32, o: &mut dyn ResolveOwner) {
    const S: u32 = 0x7ffe_f000 + 0x50000;
    for i in 0..(0x2400 / 4) {
        m.set_u32(S - 0x400 + 4 * i, 0);
    }
    let ebp0 = S + 0x1f00;
    m.set_u32(ebp0 + 8, a);
    m.set_u32(ebp0 + 0xc, rec);
    m.set_u32(ebp0 + 0x10, b);
    let (mut eax, mut ebx, mut ecx, mut edx, mut esi, mut edi) = (b, a, 0u32, 0u32, 0u32, 0u32);
    let (mut esp, mut ebp) = (S, ebp0);
    let mut fs: Vec<f64> = Vec::new();
    let mut sw: u32 = 0;
    let mut zf = false;
    let mut pc: u32 = 0x57eaf9;
    macro_rules! arg {
        ($n:expr) => {
            m.u32(esp.wrapping_add(4 * $n))
        };
    }
    macro_rules! call {
        ($t:expr) => {{
            match $t {
                0x4bdef0 => {
                    for i in 0..3 {
                        let w = arg!(i);
                        m.set_u32(ecx.wrapping_add(4 * i), w);
                    }
                    esp = esp.wrapping_add(12);
                    eax = ecx;
                }
                0x4bdd60 => {
                    let (d, v, s) = (arg!(0), arg!(1), f32::from_bits(arg!(2)));
                    vec_scale(m, d, v, s);
                    eax = d;
                }
                0x4bddb0 => {
                    let (d, x, y) = (arg!(0), arg!(1), arg!(2));
                    vec_add(m, d, x, y);
                    eax = d;
                }
                0x4cb270 => {
                    let (d, v) = (arg!(0), arg!(1));
                    vec_neg(m, d, v);
                    eax = d;
                }
                0x4cb2b0 => {
                    let (d, v, t) = (arg!(0), arg!(1), arg!(2));
                    rotate(m, d, v, t);
                    eax = d;
                }
                0x4e30f4 => {
                    let (d, x, y) = (arg!(0), arg!(1), arg!(2));
                    vec_mul(m, d, x, y);
                    eax = d;
                }
                0x4e3170 => {
                    let (d, t, v) = (arg!(0), arg!(1), arg!(2));
                    mat_scale_rows(m, d, t, v);
                    eax = d;
                }
                0x4e794f => {
                    let v = arg!(0);
                    vec_soft_limit(m, v);
                }
                0x4e28b0 => {
                    let (v, l) = (arg!(0), f32::from_bits(arg!(1)));
                    eax = fn_004e28b0(m, v, l);
                }
                0x5ab4d0 => {
                    let v = arg!(0);
                    fs.push(vec_len(m, v));
                }
                0x610f00 => {
                    let (x, y) = (arg!(0), arg!(1));
                    fs.push(vec_dot(m, x, y));
                }
                0x5ab620 => {
                    let (out, s) = (arg!(0), f32::from_bits(arg!(1)));
                    vec_scale_in_place(m, ecx, out, s);
                    esp = esp.wrapping_add(8);
                    eax = out;
                }
                0x5ab660 => {
                    let (out, s) = (arg!(0), f32::from_bits(arg!(1)));
                    vec_div_in_place(m, ecx, out, s);
                    esp = esp.wrapping_add(8);
                    eax = out;
                }
                0x5ab710 => {
                    let (out, w) = (arg!(0), arg!(1));
                    vec_add_in_place(m, ecx, out, w);
                    esp = esp.wrapping_add(8);
                    eax = out;
                }
                0x5aba40 => {
                    let (d, x, y) = (arg!(0), arg!(1), arg!(2));
                    mat_mul(m, d, x, y);
                    eax = d;
                }
                0x5ac2e0 => {
                    let (d, s) = (arg!(0), arg!(1));
                    eax = mat_inverse(m, d, s);
                }
                0x4e2821 => {
                    let (v, t, l) = (arg!(0), arg!(1), f32::from_bits(arg!(2)));
                    o.report(m, v, t, l);
                }
                0x61cca9 => {
                    eax = 0;
                }
                0x4e260c => {
                    o.log();
                }
                t => panic!("call {t:x}"),
            }
        }};
    }
    macro_rules! callv {
        ($r:expr, $slot:expr) => {{
            let args = [arg!(0), arg!(1)];
            o.virt(m, ecx, $slot, &args);
            esp = esp.wrapping_add(8);
        }};
    }
    loop {
        match pc {
            0x57eaf9 => {
        ecx = 0xc;
        esi = ebx.wrapping_add(0xc64);
        edi = esp.wrapping_add(0x1cc);
        while ecx != 0 { let v = m.u32(esi); m.set_u32(edi, v); esi = esi.wrapping_add(4); edi = edi.wrapping_add(4); ecx -= 1; }
        esi = eax.wrapping_add(0xc64);
        ecx = 0xc;
        edi = esp.wrapping_add(0x19c);
        edx = esp.wrapping_add(0x22c);
        while ecx != 0 { let v = m.u32(esi); m.set_u32(edi, v); esi = esi.wrapping_add(4); edi = edi.wrapping_add(4); ecx -= 1; }
        ecx = esp.wrapping_add(0x1cc);
        esp = esp.wrapping_sub(4); m.set_u32(esp, ecx);
        esp = esp.wrapping_sub(4); m.set_u32(esp, edx);
        call!(0x5ac2e0);
        eax = esp.wrapping_add(0x1a4);
        ecx = esp.wrapping_add(0x30c);
        esp = esp.wrapping_sub(4); m.set_u32(esp, eax);
        esp = esp.wrapping_sub(4); m.set_u32(esp, ecx);
        call!(0x5ac2e0);
        edi = m.u32(ebp.wrapping_add(0xc));
        esi = m.u32(ebp.wrapping_add(0x10));
        fs.push(m.f32(edi.wrapping_add(0x34)) as f64);
        { let b = m.f32(esp.wrapping_add(0x200)) as f64; let n = fs.len() - 1; fs[n] = fsub(fs[n], b); }
        { let v = fs.pop().unwrap(); m.set_f32(esp.wrapping_add(0x1c), v as f32); }
        fs.push(m.f32(edi.wrapping_add(0x38)) as f64);
        { let b = m.f32(esp.wrapping_add(0x204)) as f64; let n = fs.len() - 1; fs[n] = fsub(fs[n], b); }
        edx = m.u32(esp.wrapping_add(0x1c));
        m.set_u32(esp.wrapping_add(0x14c), edx);
        { let v = fs.pop().unwrap(); m.set_f32(esp.wrapping_add(0x20), v as f32); }
        fs.push(m.f32(edi.wrapping_add(0x3c)) as f64);
        { let b = m.f32(esp.wrapping_add(0x208)) as f64; let n = fs.len() - 1; fs[n] = fsub(fs[n], b); }
        eax = m.u32(esp.wrapping_add(0x20));
        m.set_u32(esp.wrapping_add(0x150), eax);
        { let v = fs.pop().unwrap(); m.set_f32(esp.wrapping_add(0x24), v as f32); }
        fs.push(m.f32(edi.wrapping_add(0x34)) as f64);
        { let b = m.f32(esp.wrapping_add(0x1d0)) as f64; let n = fs.len() - 1; fs[n] = fsub(fs[n], b); }
        ecx = m.u32(esp.wrapping_add(0x24));
        m.set_u32(esp.wrapping_add(0x154), ecx);
        { let v = fs.pop().unwrap(); m.set_f32(esp.wrapping_add(0x1c), v as f32); }
        fs.push(m.f32(edi.wrapping_add(0x38)) as f64);
        { let b = m.f32(esp.wrapping_add(0x1d4)) as f64; let n = fs.len() - 1; fs[n] = fsub(fs[n], b); }
        edx = m.u32(esp.wrapping_add(0x1c));
        m.set_u32(esp.wrapping_add(0x194), edx);
        { let v = fs.pop().unwrap(); m.set_f32(esp.wrapping_add(0x20), v as f32); }
        fs.push(m.f32(edi.wrapping_add(0x3c)) as f64);
        { let b = m.f32(esp.wrapping_add(0x1d8)) as f64; let n = fs.len() - 1; fs[n] = fsub(fs[n], b); }
        eax = m.u32(esp.wrapping_add(0x20));
        m.set_u32(esp.wrapping_add(0x198), eax);
        eax = edi.wrapping_add(0x40);
        { let v = fs.pop().unwrap(); m.set_f32(esp.wrapping_add(0x24), v as f32); }
        ecx = m.u32(esp.wrapping_add(0x24));
        edx = m.u32(eax);
        m.set_u32(esp.wrapping_add(0x19c), ecx);
        m.set_u32(esp.wrapping_add(0x28), edx);
        ecx = m.u32(eax.wrapping_add(0x4));
        m.set_u32(esp.wrapping_add(0x2c), ecx);
        ecx = m.u32(ebx.wrapping_add(0xcc4));
        edx = m.u32(eax.wrapping_add(0x8));
        eax = m.u32(0x696be0);
        m.set_u32(esp.wrapping_add(0x100), eax);
        m.set_u32(esp.wrapping_add(0x30), edx);
        edx = m.u32(esi.wrapping_add(0xcc4));
        eax = esp.wrapping_add(0x14c);
        esp = esp.wrapping_sub(4); m.set_u32(esp, eax);
        m.set_u32(esp.wrapping_add(0x90), ecx);
        m.set_u32(esp.wrapping_add(0x44), edx);
        call!(0x5ab4d0);
        { let v = fs.pop().unwrap(); m.set_f32(esp.wrapping_add(0xf0), v as f32); }
        ecx = esp.wrapping_add(0x198);
        esp = esp.wrapping_sub(4); m.set_u32(esp, ecx);
        call!(0x5ab4d0);
        edx = ebx.wrapping_add(0x1a0);
        esp = esp.wrapping_add(0x18);
        { let v = fs.pop().unwrap(); m.set_f32(esp.wrapping_add(0xd8), v as f32); }
        eax = m.u32(edx);
        m.set_u32(esp.wrapping_add(0x58), eax);
        eax = ebx.wrapping_add(0x1ac);
        ecx = m.u32(edx.wrapping_add(0x4));
        m.set_u32(esp.wrapping_add(0x5c), ecx);
        ecx = m.u32(eax);
        edx = m.u32(edx.wrapping_add(0x8));
        m.set_u32(esp.wrapping_add(0x9c), ecx);
        fs.push(m.f32(esp.wrapping_add(0x9c)) as f64);
        { let b = m.f32(esp.wrapping_add(0x1cc)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        m.set_u32(esp.wrapping_add(0x60), edx);
        edx = m.u32(eax.wrapping_add(0x4));
        m.set_u32(esp.wrapping_add(0xa0), edx);
        ecx = ebx.wrapping_add(0xcd8);
        fs.push(m.f32(esp.wrapping_add(0xa0)) as f64);
        { let b = m.f32(esp.wrapping_add(0x1d8)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        eax = m.u32(eax.wrapping_add(0x8));
        edx = m.u32(ecx);
        m.set_u32(esp.wrapping_add(0xa4), eax);
        m.set_u32(esp.wrapping_add(0xe0), edx);
        { let t = fs[fs.len() - 1 - 0]; let n = fs.len() - 1 - 1; let s = fs[n]; fs[n] = fadd(s, t); }
        fs.pop();
        fs.push(m.f32(esp.wrapping_add(0xa4)) as f64);
        { let b = m.f32(esp.wrapping_add(0x1e4)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        eax = m.u32(ecx.wrapping_add(0x4));
        edx = esi.wrapping_add(0x1a0);
        m.set_u32(esp.wrapping_add(0xe4), eax);
        { let t = fs[fs.len() - 1 - 0]; let n = fs.len() - 1 - 1; let s = fs[n]; fs[n] = fadd(s, t); }
        fs.pop();
        ecx = m.u32(ecx.wrapping_add(0x8));
        eax = m.u32(edx);
        m.set_u32(esp.wrapping_add(0xe8), ecx);
        m.set_u32(esp.wrapping_add(0x64), eax);
        { let v = fs.pop().unwrap(); m.set_f32(esp.wrapping_add(0x84), v as f32); }
        fs.push(m.f32(esp.wrapping_add(0x9c)) as f64);
        { let b = m.f32(esp.wrapping_add(0x1d0)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        fs.push(m.f32(esp.wrapping_add(0xa0)) as f64);
        { let b = m.f32(esp.wrapping_add(0x1dc)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        ecx = m.u32(edx.wrapping_add(0x4));
        eax = esi.wrapping_add(0x1ac);
        m.set_u32(esp.wrapping_add(0x68), ecx);
        { let t = fs[fs.len() - 1 - 0]; let n = fs.len() - 1 - 1; let s = fs[n]; fs[n] = fadd(s, t); }
        fs.pop();
        fs.push(m.f32(esp.wrapping_add(0xa4)) as f64);
        { let b = m.f32(esp.wrapping_add(0x1e8)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        ecx = m.u32(eax);
        edx = m.u32(edx.wrapping_add(0x8));
        m.set_u32(esp.wrapping_add(0xa8), ecx);
        m.set_u32(esp.wrapping_add(0x6c), edx);
        { let t = fs[fs.len() - 1 - 0]; let n = fs.len() - 1 - 1; let s = fs[n]; fs[n] = fadd(s, t); }
        fs.pop();
        edx = m.u32(eax.wrapping_add(0x4));
        m.set_u32(esp.wrapping_add(0xac), edx);
        { let v = fs.pop().unwrap(); m.set_f32(esp.wrapping_add(0x88), v as f32); }
        fs.push(m.f32(esp.wrapping_add(0x9c)) as f64);
        { let b = m.f32(esp.wrapping_add(0x1d4)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        fs.push(m.f32(esp.wrapping_add(0xa0)) as f64);
        { let b = m.f32(esp.wrapping_add(0x1e0)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        eax = m.u32(eax.wrapping_add(0x8));
        m.set_u32(esp.wrapping_add(0xb0), eax);
        { let t = fs[fs.len() - 1 - 0]; let n = fs.len() - 1 - 1; let s = fs[n]; fs[n] = fadd(s, t); }
        fs.pop();
        fs.push(m.f32(esp.wrapping_add(0xa4)) as f64);
        { let b = m.f32(esp.wrapping_add(0x1ec)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        { let t = fs[fs.len() - 1 - 0]; let n = fs.len() - 1 - 1; let s = fs[n]; fs[n] = fadd(s, t); }
        fs.pop();
        { let v = fs.pop().unwrap(); m.set_f32(esp.wrapping_add(0x8c), v as f32); }
        fs.push(m.f32(esp.wrapping_add(0xa8)) as f64);
        { let b = m.f32(esp.wrapping_add(0x19c)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        fs.push(m.f32(esp.wrapping_add(0xac)) as f64);
        { let b = m.f32(esp.wrapping_add(0x1a8)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        { let t = fs[fs.len() - 1 - 0]; let n = fs.len() - 1 - 1; let s = fs[n]; fs[n] = fadd(s, t); }
        fs.pop();
        fs.push(m.f32(esp.wrapping_add(0xb0)) as f64);
        { let b = m.f32(esp.wrapping_add(0x1b4)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        { let t = fs[fs.len() - 1 - 0]; let n = fs.len() - 1 - 1; let s = fs[n]; fs[n] = fadd(s, t); }
        fs.pop();
        { let v = fs.pop().unwrap(); m.set_f32(esp.wrapping_add(0xb4), v as f32); }
        fs.push(m.f32(esp.wrapping_add(0xa8)) as f64);
        { let b = m.f32(esp.wrapping_add(0x1a0)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        fs.push(m.f32(esp.wrapping_add(0xac)) as f64);
        { let b = m.f32(esp.wrapping_add(0x1ac)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        ecx = esi.wrapping_add(0xcd8);
        { let t = fs[fs.len() - 1 - 0]; let n = fs.len() - 1 - 1; let s = fs[n]; fs[n] = fadd(s, t); }
        fs.pop();
        fs.push(m.f32(esp.wrapping_add(0xb0)) as f64);
        { let b = m.f32(esp.wrapping_add(0x1b8)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        edx = m.u32(ecx);
        m.set_u32(esp.wrapping_add(0x100), edx);
        { let t = fs[fs.len() - 1 - 0]; let n = fs.len() - 1 - 1; let s = fs[n]; fs[n] = fadd(s, t); }
        fs.pop();
        eax = m.u32(ecx.wrapping_add(0x4));
        m.set_u32(esp.wrapping_add(0x104), eax);
        { let v = fs.pop().unwrap(); m.set_f32(esp.wrapping_add(0xb8), v as f32); }
        fs.push(m.f32(esp.wrapping_add(0xa8)) as f64);
        { let b = m.f32(esp.wrapping_add(0x1a4)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        fs.push(m.f32(esp.wrapping_add(0xac)) as f64);
        { let b = m.f32(esp.wrapping_add(0x1b0)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        ecx = m.u32(ecx.wrapping_add(0x8));
        m.set_u32(esp.wrapping_add(0x108), ecx);
        { let t = fs[fs.len() - 1 - 0]; let n = fs.len() - 1 - 1; let s = fs[n]; fs[n] = fadd(s, t); }
        fs.pop();
        fs.push(m.f32(esp.wrapping_add(0xb0)) as f64);
        { let b = m.f32(esp.wrapping_add(0x1bc)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        { let t = fs[fs.len() - 1 - 0]; let n = fs.len() - 1 - 1; let s = fs[n]; fs[n] = fadd(s, t); }
        fs.pop();
        { let v = fs.pop().unwrap(); m.set_f32(esp.wrapping_add(0xbc), v as f32); }
        fs.push(m.f32(esp.wrapping_add(0x58)) as f64);
        fs.push(m.f32(esp.wrapping_add(0x5c)) as f64);
        fs.push(m.f32(esp.wrapping_add(0x60)) as f64);
        { let v = fs[fs.len() - 1 - 1]; fs.push(v); }
        { let b = fs[fs.len() - 1 - 2]; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        { let v = fs[fs.len() - 1 - 3]; fs.push(v); }
        { let b = fs[fs.len() - 1 - 4]; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        { let t = fs[fs.len() - 1 - 0]; let n = fs.len() - 1 - 1; let s = fs[n]; fs[n] = fadd(s, t); }
        fs.pop();
        { let v = fs[fs.len() - 1 - 1]; fs.push(v); }
        { let b = fs[fs.len() - 1 - 2]; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        { let t = fs[fs.len() - 1 - 0]; let n = fs.len() - 1 - 1; let s = fs[n]; fs[n] = fadd(s, t); }
        fs.pop();
        { let v = fs.pop().unwrap(); let n = fs.len() - 3; fs[n] = v; }
        fs.pop();
        fs.pop();
        fs.push(m.f32(esp.wrapping_add(0x64)) as f64);
        fs.push(m.f32(esp.wrapping_add(0x68)) as f64);
        fs.push(m.f32(esp.wrapping_add(0x6c)) as f64);
        { let v = fs[fs.len() - 1 - 0]; fs.push(v); }
        { let b = fs[fs.len() - 1 - 1]; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        { let v = fs[fs.len() - 1 - 2]; fs.push(v); }
        { let b = fs[fs.len() - 1 - 3]; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        { let t = fs[fs.len() - 1 - 0]; let n = fs.len() - 1 - 1; let s = fs[n]; fs[n] = fadd(s, t); }
        fs.pop();
        { let v = fs[fs.len() - 1 - 3]; fs.push(v); }
        { let b = fs[fs.len() - 1 - 4]; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        { let t = fs[fs.len() - 1 - 0]; let n = fs.len() - 1 - 1; let s = fs[n]; fs[n] = fadd(s, t); }
        fs.pop();
        { let v = fs.pop().unwrap(); m.set_f32(esp.wrapping_add(0x80), v as f32); }
        fs.pop();
        fs.pop();
        fs.pop();
        fs.push(m.f32(esp.wrapping_add(0x9c)) as f64);
        fs.push(m.f32(esp.wrapping_add(0xa0)) as f64);
        fs.push(m.f32(esp.wrapping_add(0xa4)) as f64);
        { let v = fs[fs.len() - 1 - 0]; fs.push(v); }
        { let b = fs[fs.len() - 1 - 1]; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        { let v = fs[fs.len() - 1 - 2]; fs.push(v); }
        { let b = fs[fs.len() - 1 - 3]; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        { let t = fs[fs.len() - 1 - 0]; let n = fs.len() - 1 - 1; let s = fs[n]; fs[n] = fadd(s, t); }
        fs.pop();
        { let v = fs[fs.len() - 1 - 3]; fs.push(v); }
        { let b = fs[fs.len() - 1 - 4]; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        { let t = fs[fs.len() - 1 - 0]; let n = fs.len() - 1 - 1; let s = fs[n]; fs[n] = fadd(s, t); }
        fs.pop();
        { let v = fs.pop().unwrap(); m.set_f32(esp.wrapping_add(0x54), v as f32); }
        fs.pop();
        fs.pop();
        fs.pop();
        fs.push(m.f32(esp.wrapping_add(0xa8)) as f64);
        fs.push(m.f32(esp.wrapping_add(0xac)) as f64);
        fs.push(m.f32(esp.wrapping_add(0xb0)) as f64);
        { let v = fs[fs.len() - 1 - 0]; fs.push(v); }
        { let b = fs[fs.len() - 1 - 1]; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        { let v = fs[fs.len() - 1 - 2]; fs.push(v); }
        { let b = fs[fs.len() - 1 - 3]; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        { let t = fs[fs.len() - 1 - 0]; let n = fs.len() - 1 - 1; let s = fs[n]; fs[n] = fadd(s, t); }
        fs.pop();
        { let v = fs[fs.len() - 1 - 3]; fs.push(v); }
        { let b = fs[fs.len() - 1 - 4]; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        { let t = fs[fs.len() - 1 - 0]; let n = fs.len() - 1 - 1; let s = fs[n]; fs[n] = fadd(s, t); }
        fs.pop();
        { let v = fs.pop().unwrap(); m.set_f32(esp.wrapping_add(0x98), v as f32); }
        fs.pop();
        fs.pop();
        fs.pop();
        { let a = fs[fs.len() - 1 - 0]; let b = f32::from_bits(0x461c4000) as f64; sw = fcom(a, b); }
        eax = (eax & 0xffff0000) | sw;
        zf = ((eax >> 8) & 0x41) == 0;
        if !zf { pc = 0x57ef5d; continue; }
        { let n = fs.len() - 1; fs[n] = fs[n].sqrt(); }
        fs.push(m.f32(esp.wrapping_add(0x58)) as f64);
        { let b = fs[fs.len() - 1 - 1]; let n = fs.len() - 1; fs[n] = fdiv(fs[n], b); }
        { let v = fs.pop().unwrap(); m.set_f32(esp.wrapping_add(0x70), v as f32); }
        fs.push(m.f32(esp.wrapping_add(0x5c)) as f64);
        { let b = fs[fs.len() - 1 - 1]; let n = fs.len() - 1; fs[n] = fdiv(fs[n], b); }
        { let v = fs.pop().unwrap(); m.set_f32(esp.wrapping_add(0x74), v as f32); }
        { let b = m.f32(esp.wrapping_add(0x60)) as f64; let n = fs.len() - 1; fs[n] = fdiv(b, fs[n]); }
        fs.push(m.f32(esp.wrapping_add(0x70)) as f64);
        { let b = f32::from_bits(0x42480000) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        { let v = fs.pop().unwrap(); m.set_f32(esp.wrapping_add(0xc), v as f32); }
        fs.push(m.f32(esp.wrapping_add(0x74)) as f64);
        { let b = f32::from_bits(0x42480000) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        edx = m.u32(esp.wrapping_add(0xc));
        m.set_u32(esp.wrapping_add(0x58), edx);
        { let v = fs.pop().unwrap(); m.set_f32(esp.wrapping_add(0x10), v as f32); }
        eax = m.u32(esp.wrapping_add(0x10));
        { let b = f32::from_bits(0x42480000) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        m.set_u32(esp.wrapping_add(0x5c), eax);
        { let v = fs.pop().unwrap(); m.set_f32(esp.wrapping_add(0x14), v as f32); }
        ecx = m.u32(esp.wrapping_add(0x14));
        m.set_u32(esp.wrapping_add(0x60), ecx);
        pc = 0x57ef5f; continue;
        pc = 0x57ef5d; continue;
    }
    0x57ef5d => {
        fs.pop();
        pc = 0x57ef5f; continue;
    }
    0x57ef5f => {
        fs.push(m.f32(esp.wrapping_add(0x80)) as f64);
        { let a = fs[fs.len() - 1 - 0]; let b = f32::from_bits(0x461c4000) as f64; sw = fcom(a, b); }
        fs.pop();
        eax = (eax & 0xffff0000) | sw;
        zf = ((eax >> 8) & 0x41) == 0;
        if !zf { pc = 0x57efd2; continue; }
        fs.push(m.f32(esp.wrapping_add(0x80)) as f64);
        { let n = fs.len() - 1; fs[n] = fs[n].sqrt(); }
        fs.push(m.f32(esp.wrapping_add(0x64)) as f64);
        { let b = fs[fs.len() - 1 - 1]; let n = fs.len() - 1; fs[n] = fdiv(fs[n], b); }
        { let v = fs.pop().unwrap(); m.set_f32(esp.wrapping_add(0x70), v as f32); }
        fs.push(m.f32(esp.wrapping_add(0x68)) as f64);
        { let b = fs[fs.len() - 1 - 1]; let n = fs.len() - 1; fs[n] = fdiv(fs[n], b); }
        { let v = fs.pop().unwrap(); m.set_f32(esp.wrapping_add(0x74), v as f32); }
        { let b = m.f32(esp.wrapping_add(0x6c)) as f64; let n = fs.len() - 1; fs[n] = fdiv(b, fs[n]); }
        fs.push(m.f32(esp.wrapping_add(0x70)) as f64);
        { let b = f32::from_bits(0x42480000) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        { let v = fs.pop().unwrap(); m.set_f32(esp.wrapping_add(0xc), v as f32); }
        fs.push(m.f32(esp.wrapping_add(0x74)) as f64);
        { let b = f32::from_bits(0x42480000) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        edx = m.u32(esp.wrapping_add(0xc));
        m.set_u32(esp.wrapping_add(0x64), edx);
        { let v = fs.pop().unwrap(); m.set_f32(esp.wrapping_add(0x10), v as f32); }
        eax = m.u32(esp.wrapping_add(0x10));
        { let b = f32::from_bits(0x42480000) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        m.set_u32(esp.wrapping_add(0x68), eax);
        { let v = fs.pop().unwrap(); m.set_f32(esp.wrapping_add(0x14), v as f32); }
        ecx = m.u32(esp.wrapping_add(0x14));
        m.set_u32(esp.wrapping_add(0x6c), ecx);
        pc = 0x57efd2; continue;
    }
    0x57efd2 => {
        fs.push(m.f32(esp.wrapping_add(0x54)) as f64);
        { let a = fs[fs.len() - 1 - 0]; let b = f32::from_bits(0x44610000) as f64; sw = fcom(a, b); }
        fs.pop();
        eax = (eax & 0xffff0000) | sw;
        zf = ((eax >> 8) & 0x41) == 0;
        if !zf { pc = 0x57f051; continue; }
        fs.push(m.f32(esp.wrapping_add(0x54)) as f64);
        { let n = fs.len() - 1; fs[n] = fs[n].sqrt(); }
        fs.push(m.f32(esp.wrapping_add(0x9c)) as f64);
        { let b = fs[fs.len() - 1 - 1]; let n = fs.len() - 1; fs[n] = fdiv(fs[n], b); }
        { let v = fs.pop().unwrap(); m.set_f32(esp.wrapping_add(0x70), v as f32); }
        fs.push(m.f32(esp.wrapping_add(0xa0)) as f64);
        { let b = fs[fs.len() - 1 - 1]; let n = fs.len() - 1; fs[n] = fdiv(fs[n], b); }
        { let v = fs.pop().unwrap(); m.set_f32(esp.wrapping_add(0x74), v as f32); }
        { let b = m.f32(esp.wrapping_add(0xa4)) as f64; let n = fs.len() - 1; fs[n] = fdiv(b, fs[n]); }
        fs.push(m.f32(esp.wrapping_add(0x70)) as f64);
        { let b = f32::from_bits(0x41200000) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        { let v = fs.pop().unwrap(); m.set_f32(esp.wrapping_add(0xc), v as f32); }
        fs.push(m.f32(esp.wrapping_add(0x74)) as f64);
        { let b = f32::from_bits(0x41200000) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        edx = m.u32(esp.wrapping_add(0xc));
        m.set_u32(esp.wrapping_add(0x9c), edx);
        { let v = fs.pop().unwrap(); m.set_f32(esp.wrapping_add(0x10), v as f32); }
        eax = m.u32(esp.wrapping_add(0x10));
        { let b = f32::from_bits(0x41200000) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        m.set_u32(esp.wrapping_add(0xa0), eax);
        { let v = fs.pop().unwrap(); m.set_f32(esp.wrapping_add(0x14), v as f32); }
        ecx = m.u32(esp.wrapping_add(0x14));
        m.set_u32(esp.wrapping_add(0xa4), ecx);
        pc = 0x57f051; continue;
    }
    0x57f051 => {
        fs.push(m.f32(esp.wrapping_add(0x98)) as f64);
        { let a = fs[fs.len() - 1 - 0]; let b = f32::from_bits(0x44610000) as f64; sw = fcom(a, b); }
        fs.pop();
        eax = (eax & 0xffff0000) | sw;
        zf = ((eax >> 8) & 0x41) == 0;
        if !zf { pc = 0x57f0d6; continue; }
        fs.push(m.f32(esp.wrapping_add(0x98)) as f64);
        { let n = fs.len() - 1; fs[n] = fs[n].sqrt(); }
        fs.push(m.f32(esp.wrapping_add(0xa8)) as f64);
        { let b = fs[fs.len() - 1 - 1]; let n = fs.len() - 1; fs[n] = fdiv(fs[n], b); }
        { let v = fs.pop().unwrap(); m.set_f32(esp.wrapping_add(0x70), v as f32); }
        fs.push(m.f32(esp.wrapping_add(0xac)) as f64);
        { let b = fs[fs.len() - 1 - 1]; let n = fs.len() - 1; fs[n] = fdiv(fs[n], b); }
        { let v = fs.pop().unwrap(); m.set_f32(esp.wrapping_add(0x74), v as f32); }
        { let b = m.f32(esp.wrapping_add(0xb0)) as f64; let n = fs.len() - 1; fs[n] = fdiv(b, fs[n]); }
        fs.push(m.f32(esp.wrapping_add(0x70)) as f64);
        { let b = f32::from_bits(0x41200000) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        { let v = fs.pop().unwrap(); m.set_f32(esp.wrapping_add(0xc), v as f32); }
        fs.push(m.f32(esp.wrapping_add(0x74)) as f64);
        { let b = f32::from_bits(0x41200000) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        edx = m.u32(esp.wrapping_add(0xc));
        m.set_u32(esp.wrapping_add(0xa8), edx);
        { let v = fs.pop().unwrap(); m.set_f32(esp.wrapping_add(0x10), v as f32); }
        eax = m.u32(esp.wrapping_add(0x10));
        { let b = f32::from_bits(0x41200000) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        m.set_u32(esp.wrapping_add(0xac), eax);
        { let v = fs.pop().unwrap(); m.set_f32(esp.wrapping_add(0x14), v as f32); }
        ecx = m.u32(esp.wrapping_add(0x14));
        m.set_u32(esp.wrapping_add(0xb0), ecx);
        pc = 0x57f0d6; continue;
    }
    0x57f0d6 => {
        esp = esp.wrapping_sub(4); m.set_u32(esp, 0x43160000);
        edx = esp.wrapping_add(0x5c);
        esp = esp.wrapping_sub(4); m.set_u32(esp, 0x67d408);
        esp = esp.wrapping_sub(4); m.set_u32(esp, edx);
        call!(0x4e2821);
        esp = esp.wrapping_sub(4); m.set_u32(esp, 0x43160000);
        eax = esp.wrapping_add(0x74);
        esp = esp.wrapping_sub(4); m.set_u32(esp, 0x67d3f0);
        esp = esp.wrapping_sub(4); m.set_u32(esp, eax);
        call!(0x4e2821);
        esp = esp.wrapping_sub(4); m.set_u32(esp, 0x41f00000);
        ecx = esp.wrapping_add(0xb8);
        esp = esp.wrapping_sub(4); m.set_u32(esp, 0x67d3d8);
        esp = esp.wrapping_sub(4); m.set_u32(esp, ecx);
        call!(0x4e2821);
        esp = esp.wrapping_sub(4); m.set_u32(esp, 0x41f00000);
        edx = esp.wrapping_add(0xd0);
        esp = esp.wrapping_sub(4); m.set_u32(esp, 0x67d3c0);
        esp = esp.wrapping_sub(4); m.set_u32(esp, edx);
        call!(0x4e2821);
        fs.push(m.f32(esp.wrapping_add(0x94)) as f64);
        { let b = m.f32(esp.wrapping_add(0x60)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        fs.push(m.f32(esp.wrapping_add(0x98)) as f64);
        { let b = m.f32(esp.wrapping_add(0x60)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        esp = esp.wrapping_add(0x2c);
        { let v = fs.pop().unwrap(); m.set_f32(esp.wrapping_add(0x14), v as f32); }
        fs.push(m.f32(esp.wrapping_add(0x70)) as f64);
        { let b = m.f32(esp.wrapping_add(0x34)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        { let v = fs.pop().unwrap(); m.set_f32(esp.wrapping_add(0x18), v as f32); }
        fs.push(m.f32(esp.wrapping_add(0x5c)) as f64);
        { let b = m.f32(esp.wrapping_add(0x80)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        fs.push(m.f32(esp.wrapping_add(0x60)) as f64);
        { let b = m.f32(esp.wrapping_add(0x80)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        { let v = fs.pop().unwrap(); m.set_f32(esp.wrapping_add(0x78), v as f32); }
        fs.push(m.f32(esp.wrapping_add(0x64)) as f64);
        { let b = m.f32(esp.wrapping_add(0x80)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        { let v = fs.pop().unwrap(); m.set_f32(esp.wrapping_add(0x7c), v as f32); }
        { let t = fs[fs.len() - 1 - 0]; let n = fs.len() - 1 - 1; let s = fs[n]; fs[n] = fadd(s, t); }
        fs.pop();
        fs.push(m.f32(esp.wrapping_add(0x14)) as f64);
        { let b = m.f32(esp.wrapping_add(0x78)) as f64; let n = fs.len() - 1; fs[n] = fadd(fs[n], b); }
        { let v = fs.pop().unwrap(); m.set_f32(esp.wrapping_add(0x4c), v as f32); }
        fs.push(m.f32(esp.wrapping_add(0x18)) as f64);
        { let b = m.f32(esp.wrapping_add(0x7c)) as f64; let n = fs.len() - 1; fs[n] = fadd(fs[n], b); }
        { let v = fs.pop().unwrap(); m.set_f32(esp.wrapping_add(0x50), v as f32); }
        fs.push(m.f32(esp.wrapping_add(0x34)) as f64);
        { let b = m.f32(esp.wrapping_add(0x80)) as f64; let n = fs.len() - 1; fs[n] = fadd(fs[n], b); }
        { let v = fs.pop().unwrap(); m.set_f32(esp.wrapping_add(0x58), v as f32); }
        { let b = m.f32(esp.wrapping_add(0x58)) as f64; let n = fs.len() - 1; fs[n] = fdiv(fs[n], b); }
        { let v = fs.pop().unwrap(); m.set_f32(esp.wrapping_add(0x38), v as f32); }
        fs.push(m.f32(esp.wrapping_add(0x4c)) as f64);
        { let b = m.f32(esp.wrapping_add(0x58)) as f64; let n = fs.len() - 1; fs[n] = fdiv(fs[n], b); }
        eax = m.u32(esp.wrapping_add(0x38));
        m.set_u32(esp.wrapping_add(0x170), eax);
        { let v = fs.pop().unwrap(); m.set_f32(esp.wrapping_add(0x3c), v as f32); }
        fs.push(m.f32(esp.wrapping_add(0x50)) as f64);
        { let b = m.f32(esp.wrapping_add(0x58)) as f64; let n = fs.len() - 1; fs[n] = fdiv(fs[n], b); }
        ecx = m.u32(esp.wrapping_add(0x3c));
        m.set_u32(esp.wrapping_add(0x174), ecx);
        { let v = fs.pop().unwrap(); m.set_f32(esp.wrapping_add(0x40), v as f32); }
        fs.push(m.f32(esp.wrapping_add(0x64)) as f64);
        { let b = m.f32(esp.wrapping_add(0x40)) as f64; let n = fs.len() - 1; fs[n] = fsub(fs[n], b); }
        edx = m.u32(esp.wrapping_add(0x40));
        m.set_u32(esp.wrapping_add(0x178), edx);
        { let v = fs.pop().unwrap(); m.set_f32(esp, v as f32); }
        fs.push(m.f32(esp.wrapping_add(0x60)) as f64);
        { let b = m.f32(esp.wrapping_add(0x3c)) as f64; let n = fs.len() - 1; fs[n] = fsub(fs[n], b); }
        esp = esp.wrapping_sub(4); m.set_u32(esp, ecx);
        { let v = fs.pop().unwrap(); m.set_f32(esp, v as f32); }
        fs.push(m.f32(esp.wrapping_add(0x60)) as f64);
        { let b = m.f32(esp.wrapping_add(0x3c)) as f64; let n = fs.len() - 1; fs[n] = fsub(fs[n], b); }
        esp = esp.wrapping_sub(4); m.set_u32(esp, ecx);
        ecx = esp.wrapping_add(0x18);
        { let v = fs.pop().unwrap(); m.set_f32(esp, v as f32); }
        call!(0x4bdef0);
        fs.push(m.f32(esp.wrapping_add(0x6c)) as f64);
        ecx = m.u32(esp.wrapping_add(0x10));
        eax = m.u32(esp.wrapping_add(0xc));
        { let b = m.f32(esp.wrapping_add(0x174)) as f64; let n = fs.len() - 1; fs[n] = fsub(fs[n], b); }
        esp = esp.wrapping_sub(4); m.set_u32(esp, ecx);
        edx = m.u32(esp.wrapping_add(0x18));
        m.set_u32(esp.wrapping_add(0x60), ecx);
        m.set_u32(esp.wrapping_add(0x5c), eax);
        m.set_u32(esp.wrapping_add(0x64), edx);
        { let v = fs.pop().unwrap(); m.set_f32(esp, v as f32); }
        fs.push(m.f32(esp.wrapping_add(0x6c)) as f64);
        { let b = m.f32(esp.wrapping_add(0x174)) as f64; let n = fs.len() - 1; fs[n] = fsub(fs[n], b); }
        esp = esp.wrapping_sub(4); m.set_u32(esp, ecx);
        { let v = fs.pop().unwrap(); m.set_f32(esp, v as f32); }
        fs.push(m.f32(esp.wrapping_add(0x6c)) as f64);
        { let b = m.f32(esp.wrapping_add(0x174)) as f64; let n = fs.len() - 1; fs[n] = fsub(fs[n], b); }
        esp = esp.wrapping_sub(4); m.set_u32(esp, ecx);
        ecx = esp.wrapping_add(0x18);
        { let v = fs.pop().unwrap(); m.set_f32(esp, v as f32); }
        call!(0x4bdef0);
        fs.push(m.f32(esp.wrapping_add(0x13c)) as f64);
        { let b = m.f32(esp.wrapping_add(0x18)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        fs.push(m.f32(esp.wrapping_add(0x144)) as f64);
        { let b = m.f32(esp.wrapping_add(0x20)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        eax = m.u32(esp.wrapping_add(0xc));
        ecx = m.u32(esp.wrapping_add(0x10));
        edx = m.u32(esp.wrapping_add(0x14));
        m.set_u32(esp.wrapping_add(0x64), eax);
        { let t = fs[fs.len() - 1 - 0]; let n = fs.len() - 1 - 1; let s = fs[n]; fs[n] = fadd(s, t); }
        fs.pop();
        fs.push(m.f32(esp.wrapping_add(0x140)) as f64);
        { let b = m.f32(esp.wrapping_add(0x1c)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        m.set_u32(esp.wrapping_add(0x68), ecx);
        m.set_u32(esp.wrapping_add(0x6c), edx);
        { let t = fs[fs.len() - 1 - 0]; let n = fs.len() - 1 - 1; let s = fs[n]; fs[n] = fadd(s, t); }
        fs.pop();
        { let a = fs[fs.len() - 1 - 0]; let b = f64::from_bits(0x0000000000000000); sw = fcom(a, b); }
        fs.pop();
        eax = (eax & 0xffff0000) | sw;
        zf = ((eax >> 8) & 0x41) == 0;
        if !zf { pc = 0x57f2bf; continue; }
        eax = esp.wrapping_add(0x24);
        esp = esp.wrapping_sub(4); m.set_u32(esp, 0xbf800000);
        esp = esp.wrapping_sub(4); m.set_u32(esp, eax);
        ecx = esp.wrapping_add(0x20);
        call!(0x5ab620);
        pc = 0x57f2bf; continue;
    }
    0x57f2bf => {
        fs.push(m.f32(esp.wrapping_add(0x144)) as f64);
        { let b = m.f32(esp.wrapping_add(0xdc)) as f64; let n = fs.len() - 1; fs[n] = fdiv(fs[n], b); }
        esp = esp.wrapping_sub(4); m.set_u32(esp, ecx);
        { let v = fs.pop().unwrap(); m.set_f32(esp, v as f32); }
        fs.push(m.f32(esp.wrapping_add(0x144)) as f64);
        { let b = m.f32(esp.wrapping_add(0xe0)) as f64; let n = fs.len() - 1; fs[n] = fdiv(fs[n], b); }
        esp = esp.wrapping_sub(4); m.set_u32(esp, ecx);
        { let v = fs.pop().unwrap(); m.set_f32(esp, v as f32); }
        fs.push(m.f32(esp.wrapping_add(0x144)) as f64);
        { let b = m.f32(esp.wrapping_add(0xe4)) as f64; let n = fs.len() - 1; fs[n] = fdiv(fs[n], b); }
        esp = esp.wrapping_sub(4); m.set_u32(esp, ecx);
        ecx = esp.wrapping_add(0x40);
        { let v = fs.pop().unwrap(); m.set_f32(esp, v as f32); }
        call!(0x4bdef0);
        fs.push(m.f32(esp.wrapping_add(0x18c)) as f64);
        { let b = m.f32(esp.wrapping_add(0xd8)) as f64; let n = fs.len() - 1; fs[n] = fdiv(fs[n], b); }
        esp = esp.wrapping_sub(4); m.set_u32(esp, ecx);
        { let v = fs.pop().unwrap(); m.set_f32(esp, v as f32); }
        fs.push(m.f32(esp.wrapping_add(0x18c)) as f64);
        { let b = m.f32(esp.wrapping_add(0xdc)) as f64; let n = fs.len() - 1; fs[n] = fdiv(fs[n], b); }
        esp = esp.wrapping_sub(4); m.set_u32(esp, ecx);
        { let v = fs.pop().unwrap(); m.set_f32(esp, v as f32); }
        fs.push(m.f32(esp.wrapping_add(0x18c)) as f64);
        { let b = m.f32(esp.wrapping_add(0xe0)) as f64; let n = fs.len() - 1; fs[n] = fdiv(fs[n], b); }
        esp = esp.wrapping_sub(4); m.set_u32(esp, ecx);
        ecx = esp.wrapping_add(0x50);
        { let v = fs.pop().unwrap(); m.set_f32(esp, v as f32); }
        call!(0x4bdef0);
        fs.push(m.f32(esp.wrapping_add(0x18)) as f64);
        { let b = m.f32(esp.wrapping_add(0x5c)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        fs.push(m.f32(esp.wrapping_add(0x58)) as f64);
        { let b = m.f32(esp.wrapping_add(0x1c)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        esp = esp.wrapping_sub(4); m.set_u32(esp, ecx);
        { let t = fs[fs.len() - 1 - 0]; let n = fs.len() - 1 - 1; let s = fs[n]; fs[n] = fsub(s, t); }
        fs.pop();
        { let v = fs.pop().unwrap(); m.set_f32(esp, v as f32); }
        fs.push(m.f32(esp.wrapping_add(0x5c)) as f64);
        { let b = m.f32(esp.wrapping_add(0x24)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        fs.push(m.f32(esp.wrapping_add(0x1c)) as f64);
        { let b = m.f32(esp.wrapping_add(0x64)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        esp = esp.wrapping_sub(4); m.set_u32(esp, ecx);
        { let t = fs[fs.len() - 1 - 0]; let n = fs.len() - 1 - 1; let s = fs[n]; fs[n] = fsub(s, t); }
        fs.pop();
        { let v = fs.pop().unwrap(); m.set_f32(esp, v as f32); }
        fs.push(m.f32(esp.wrapping_add(0x68)) as f64);
        { let b = m.f32(esp.wrapping_add(0x24)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        fs.push(m.f32(esp.wrapping_add(0x64)) as f64);
        { let b = m.f32(esp.wrapping_add(0x28)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        esp = esp.wrapping_sub(4); m.set_u32(esp, ecx);
        ecx = esp.wrapping_add(0xd0);
        { let t = fs[fs.len() - 1 - 0]; let n = fs.len() - 1 - 1; let s = fs[n]; fs[n] = fsub(s, t); }
        fs.pop();
        { let v = fs.pop().unwrap(); m.set_f32(esp, v as f32); }
        call!(0x4bdef0);
        fs.push(m.f32(esp.wrapping_add(0xc4)) as f64);
        { let b = m.f32(esp.wrapping_add(0x1c)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        fs.push(m.f32(esp.wrapping_add(0x18)) as f64);
        { let b = m.f32(esp.wrapping_add(0xc8)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        esp = esp.wrapping_sub(4); m.set_u32(esp, ecx);
        { let t = fs[fs.len() - 1 - 0]; let n = fs.len() - 1 - 1; let s = fs[n]; fs[n] = fsub(s, t); }
        fs.pop();
        { let v = fs.pop().unwrap(); m.set_f32(esp, v as f32); }
        fs.push(m.f32(esp.wrapping_add(0x1c)) as f64);
        { let b = m.f32(esp.wrapping_add(0xd0)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        fs.push(m.f32(esp.wrapping_add(0xc8)) as f64);
        { let b = m.f32(esp.wrapping_add(0x24)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        esp = esp.wrapping_sub(4); m.set_u32(esp, ecx);
        { let t = fs[fs.len() - 1 - 0]; let n = fs.len() - 1 - 1; let s = fs[n]; fs[n] = fsub(s, t); }
        fs.pop();
        { let v = fs.pop().unwrap(); m.set_f32(esp, v as f32); }
        fs.push(m.f32(esp.wrapping_add(0xd0)) as f64);
        { let b = m.f32(esp.wrapping_add(0x28)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        fs.push(m.f32(esp.wrapping_add(0xd4)) as f64);
        { let b = m.f32(esp.wrapping_add(0x24)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        esp = esp.wrapping_sub(4); m.set_u32(esp, ecx);
        ecx = esp.wrapping_add(0x18);
        { let t = fs[fs.len() - 1 - 0]; let n = fs.len() - 1 - 1; let s = fs[n]; fs[n] = fsub(s, t); }
        fs.pop();
        { let v = fs.pop().unwrap(); m.set_f32(esp, v as f32); }
        call!(0x4bdef0);
        ecx = m.u32(esp.wrapping_add(0xc));
        edx = m.u32(esp.wrapping_add(0x10));
        eax = m.u32(esp.wrapping_add(0x14));
        m.set_u32(esp.wrapping_add(0x208), ecx);
        fs.push(m.f32(esp.wrapping_add(0x84)) as f64);
        m.set_u32(esp.wrapping_add(0x20c), edx);
        m.set_u32(esp.wrapping_add(0x210), eax);
        { let b = m.f32(esp.wrapping_add(0x34)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        fs.push(m.f32(esp.wrapping_add(0x3c)) as f64);
        { let b = m.f32(esp.wrapping_add(0x8c)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        esp = esp.wrapping_sub(4); m.set_u32(esp, ecx);
        { let t = fs[fs.len() - 1 - 0]; let n = fs.len() - 1 - 1; let s = fs[n]; fs[n] = fadd(s, t); }
        fs.pop();
        fs.push(m.f32(esp.wrapping_add(0x3c)) as f64);
        { let b = m.f32(esp.wrapping_add(0x8c)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        { let t = fs[fs.len() - 1 - 0]; let n = fs.len() - 1 - 1; let s = fs[n]; fs[n] = fadd(s, t); }
        fs.pop();
        { let v = fs[fs.len() - 1 - 0]; fs.push(v); }
        { let b = m.f32(esp.wrapping_add(0x40)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        { let v = fs.pop().unwrap(); m.set_f32(esp, v as f32); }
        esp = esp.wrapping_sub(4); m.set_u32(esp, ecx);
        { let v = fs[fs.len() - 1 - 0]; fs.push(v); }
        { let b = m.f32(esp.wrapping_add(0x40)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        { let v = fs.pop().unwrap(); m.set_f32(esp, v as f32); }
        esp = esp.wrapping_sub(4); m.set_u32(esp, ecx);
        ecx = esp.wrapping_add(0x18);
        { let b = m.f32(esp.wrapping_add(0x40)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        { let v = fs.pop().unwrap(); m.set_f32(esp, v as f32); }
        call!(0x4bdef0);
        fs.push(m.f32(esp.wrapping_add(0x18)) as f64);
        { let b = m.f32(esp.wrapping_add(0x88)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        fs.push(m.f32(esp.wrapping_add(0x84)) as f64);
        { let b = m.f32(esp.wrapping_add(0x1c)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        ecx = m.u32(esp.wrapping_add(0xc));
        edx = m.u32(esp.wrapping_add(0x10));
        esp = esp.wrapping_sub(4); m.set_u32(esp, ecx);
        eax = m.u32(esp.wrapping_add(0x18));
        { let t = fs[fs.len() - 1 - 0]; let n = fs.len() - 1 - 1; let s = fs[n]; fs[n] = fsub(s, t); }
        fs.pop();
        m.set_u32(esp.wrapping_add(0x194), ecx);
        m.set_u32(esp.wrapping_add(0x198), edx);
        m.set_u32(esp.wrapping_add(0x19c), eax);
        { let v = fs.pop().unwrap(); m.set_f32(esp, v as f32); }
        fs.push(m.f32(esp.wrapping_add(0x88)) as f64);
        { let b = m.f32(esp.wrapping_add(0x24)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        fs.push(m.f32(esp.wrapping_add(0x1c)) as f64);
        { let b = m.f32(esp.wrapping_add(0x90)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        esp = esp.wrapping_sub(4); m.set_u32(esp, ecx);
        { let t = fs[fs.len() - 1 - 0]; let n = fs.len() - 1 - 1; let s = fs[n]; fs[n] = fsub(s, t); }
        fs.pop();
        { let v = fs.pop().unwrap(); m.set_f32(esp, v as f32); }
        fs.push(m.f32(esp.wrapping_add(0x24)) as f64);
        { let b = m.f32(esp.wrapping_add(0x94)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        fs.push(m.f32(esp.wrapping_add(0x28)) as f64);
        { let b = m.f32(esp.wrapping_add(0x90)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        esp = esp.wrapping_sub(4); m.set_u32(esp, ecx);
        ecx = esp.wrapping_add(0xd0);
        { let t = fs[fs.len() - 1 - 0]; let n = fs.len() - 1 - 1; let s = fs[n]; fs[n] = fsub(s, t); }
        fs.pop();
        { let v = fs.pop().unwrap(); m.set_f32(esp, v as f32); }
        call!(0x4bdef0);
        fs.push(m.f32(esp.wrapping_add(0xc4)) as f64);
        { let b = m.f32(esp.wrapping_add(0x1c)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        fs.push(m.f32(esp.wrapping_add(0x18)) as f64);
        { let b = m.f32(esp.wrapping_add(0xc8)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        esp = esp.wrapping_sub(4); m.set_u32(esp, ecx);
        { let t = fs[fs.len() - 1 - 0]; let n = fs.len() - 1 - 1; let s = fs[n]; fs[n] = fsub(s, t); }
        fs.pop();
        { let v = fs.pop().unwrap(); m.set_f32(esp, v as f32); }
        fs.push(m.f32(esp.wrapping_add(0x1c)) as f64);
        { let b = m.f32(esp.wrapping_add(0xd0)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        fs.push(m.f32(esp.wrapping_add(0xc8)) as f64);
        { let b = m.f32(esp.wrapping_add(0x24)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        esp = esp.wrapping_sub(4); m.set_u32(esp, ecx);
        { let t = fs[fs.len() - 1 - 0]; let n = fs.len() - 1 - 1; let s = fs[n]; fs[n] = fsub(s, t); }
        fs.pop();
        { let v = fs.pop().unwrap(); m.set_f32(esp, v as f32); }
        fs.push(m.f32(esp.wrapping_add(0xd0)) as f64);
        { let b = m.f32(esp.wrapping_add(0x28)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        fs.push(m.f32(esp.wrapping_add(0xd4)) as f64);
        { let b = m.f32(esp.wrapping_add(0x24)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        esp = esp.wrapping_sub(4); m.set_u32(esp, ecx);
        ecx = esp.wrapping_add(0x7c);
        { let t = fs[fs.len() - 1 - 0]; let n = fs.len() - 1 - 1; let s = fs[n]; fs[n] = fsub(s, t); }
        fs.pop();
        { let v = fs.pop().unwrap(); m.set_f32(esp, v as f32); }
        call!(0x4bdef0);
        fs.push(m.f32(esp.wrapping_add(0x74)) as f64);
        { let b = m.f32(esp.wrapping_add(0x34)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        fs.push(m.f32(esp.wrapping_add(0x70)) as f64);
        { let b = m.f32(esp.wrapping_add(0x38)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        esp = esp.wrapping_sub(4); m.set_u32(esp, ecx);
        { let t = fs[fs.len() - 1 - 0]; let n = fs.len() - 1 - 1; let s = fs[n]; fs[n] = fsub(s, t); }
        fs.pop();
        { let v = fs.pop().unwrap(); m.set_f32(esp, v as f32); }
        fs.push(m.f32(esp.wrapping_add(0x74)) as f64);
        { let b = m.f32(esp.wrapping_add(0x40)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        fs.push(m.f32(esp.wrapping_add(0x7c)) as f64);
        { let b = m.f32(esp.wrapping_add(0x38)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        esp = esp.wrapping_sub(4); m.set_u32(esp, ecx);
        { let t = fs[fs.len() - 1 - 0]; let n = fs.len() - 1 - 1; let s = fs[n]; fs[n] = fsub(s, t); }
        fs.pop();
        { let v = fs.pop().unwrap(); m.set_f32(esp, v as f32); }
        fs.push(m.f32(esp.wrapping_add(0x80)) as f64);
        { let b = m.f32(esp.wrapping_add(0x40)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        fs.push(m.f32(esp.wrapping_add(0x7c)) as f64);
        { let b = m.f32(esp.wrapping_add(0x44)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        esp = esp.wrapping_sub(4); m.set_u32(esp, ecx);
        ecx = esp.wrapping_add(0x18);
        { let t = fs[fs.len() - 1 - 0]; let n = fs.len() - 1 - 1; let s = fs[n]; fs[n] = fsub(s, t); }
        fs.pop();
        { let v = fs.pop().unwrap(); m.set_f32(esp, v as f32); }
        call!(0x4bdef0);
        fs.push(m.f32(esp.wrapping_add(0xc)) as f64);
        { let b = m.f32(esp.wrapping_add(0x38)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        fs.push(m.f32(esp.wrapping_add(0x10)) as f64);
        { let b = m.f32(esp.wrapping_add(0x34)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        esp = esp.wrapping_sub(4); m.set_u32(esp, ecx);
        { let t = fs[fs.len() - 1 - 0]; let n = fs.len() - 1 - 1; let s = fs[n]; fs[n] = fsub(s, t); }
        fs.pop();
        { let v = fs.pop().unwrap(); m.set_f32(esp, v as f32); }
        fs.push(m.f32(esp.wrapping_add(0x18)) as f64);
        { let b = m.f32(esp.wrapping_add(0x38)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        fs.push(m.f32(esp.wrapping_add(0x10)) as f64);
        { let b = m.f32(esp.wrapping_add(0x40)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        esp = esp.wrapping_sub(4); m.set_u32(esp, ecx);
        { let t = fs[fs.len() - 1 - 0]; let n = fs.len() - 1 - 1; let s = fs[n]; fs[n] = fsub(s, t); }
        fs.pop();
        { let v = fs.pop().unwrap(); m.set_f32(esp, v as f32); }
        fs.push(m.f32(esp.wrapping_add(0x18)) as f64);
        { let b = m.f32(esp.wrapping_add(0x44)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        fs.push(m.f32(esp.wrapping_add(0x1c)) as f64);
        { let b = m.f32(esp.wrapping_add(0x40)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        esp = esp.wrapping_sub(4); m.set_u32(esp, ecx);
        ecx = esp.wrapping_add(0x30);
        { let t = fs[fs.len() - 1 - 0]; let n = fs.len() - 1 - 1; let s = fs[n]; fs[n] = fsub(s, t); }
        fs.pop();
        { let v = fs.pop().unwrap(); m.set_f32(esp, v as f32); }
        call!(0x4bdef0);
        fs.push(m.f32(esp.wrapping_add(0x8c)) as f64);
        ecx = m.u32(esp.wrapping_add(0x24));
        edx = m.u32(esp.wrapping_add(0x28));
        { let b = m.f32(esp.wrapping_add(0x198)) as f64; let n = fs.len() - 1; fs[n] = fsub(fs[n], b); }
        esp = esp.wrapping_sub(4); m.set_u32(esp, ecx);
        eax = m.u32(esp.wrapping_add(0x30));
        m.set_u32(esp.wrapping_add(0x14c), ecx);
        m.set_u32(esp.wrapping_add(0x150), edx);
        m.set_u32(esp.wrapping_add(0x154), eax);
        { let v = fs.pop().unwrap(); m.set_f32(esp, v as f32); }
        fs.push(m.f32(esp.wrapping_add(0x8c)) as f64);
        { let b = m.f32(esp.wrapping_add(0x198)) as f64; let n = fs.len() - 1; fs[n] = fsub(fs[n], b); }
        esp = esp.wrapping_sub(4); m.set_u32(esp, ecx);
        { let v = fs.pop().unwrap(); m.set_f32(esp, v as f32); }
        fs.push(m.f32(esp.wrapping_add(0x8c)) as f64);
        { let b = m.f32(esp.wrapping_add(0x198)) as f64; let n = fs.len() - 1; fs[n] = fsub(fs[n], b); }
        esp = esp.wrapping_sub(4); m.set_u32(esp, ecx);
        ecx = esp.wrapping_add(0x30);
        { let v = fs.pop().unwrap(); m.set_f32(esp, v as f32); }
        call!(0x4bdef0);
        fs.push(m.f32(esp.wrapping_add(0x2c)) as f64);
        { let b = m.f32(esp.wrapping_add(0x150)) as f64; let n = fs.len() - 1; fs[n] = fsub(fs[n], b); }
        esp = esp.wrapping_sub(4); m.set_u32(esp, ecx);
        { let v = fs.pop().unwrap(); m.set_f32(esp, v as f32); }
        fs.push(m.f32(esp.wrapping_add(0x2c)) as f64);
        { let b = m.f32(esp.wrapping_add(0x150)) as f64; let n = fs.len() - 1; fs[n] = fsub(fs[n], b); }
        esp = esp.wrapping_sub(4); m.set_u32(esp, ecx);
        { let v = fs.pop().unwrap(); m.set_f32(esp, v as f32); }
        fs.push(m.f32(esp.wrapping_add(0x2c)) as f64);
        { let b = m.f32(esp.wrapping_add(0x150)) as f64; let n = fs.len() - 1; fs[n] = fsub(fs[n], b); }
        esp = esp.wrapping_sub(4); m.set_u32(esp, ecx);
        ecx = esp.wrapping_add(0x18);
        { let v = fs.pop().unwrap(); m.set_f32(esp, v as f32); }
        call!(0x4bdef0);
        fs.push(m.f32(esp.wrapping_add(0x18)) as f64);
        ecx = m.u32(esp.wrapping_add(0xc));
        edx = m.u32(esp.wrapping_add(0x10));
        eax = m.u32(esp.wrapping_add(0x14));
        m.set_u32(esp.wrapping_add(0x1fc), ecx);
        { let b = m.f32(esp.wrapping_add(0x68)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        fs.push(m.f32(esp.wrapping_add(0x64)) as f64);
        m.set_u32(esp.wrapping_add(0x200), edx);
        m.set_u32(esp.wrapping_add(0x204), eax);
        { let b = m.f32(esp.wrapping_add(0x1c)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        esp = esp.wrapping_sub(4); m.set_u32(esp, ecx);
        { let t = fs[fs.len() - 1 - 0]; let n = fs.len() - 1 - 1; let s = fs[n]; fs[n] = fsub(s, t); }
        fs.pop();
        { let v = fs.pop().unwrap(); m.set_f32(esp, v as f32); }
        fs.push(m.f32(esp.wrapping_add(0x68)) as f64);
        { let b = m.f32(esp.wrapping_add(0x24)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        fs.push(m.f32(esp.wrapping_add(0x1c)) as f64);
        { let b = m.f32(esp.wrapping_add(0x70)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        esp = esp.wrapping_sub(4); m.set_u32(esp, ecx);
        { let t = fs[fs.len() - 1 - 0]; let n = fs.len() - 1 - 1; let s = fs[n]; fs[n] = fsub(s, t); }
        fs.pop();
        { let v = fs.pop().unwrap(); m.set_f32(esp, v as f32); }
        fs.push(m.f32(esp.wrapping_add(0x74)) as f64);
        { let b = m.f32(esp.wrapping_add(0x24)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        fs.push(m.f32(esp.wrapping_add(0x70)) as f64);
        { let b = m.f32(esp.wrapping_add(0x28)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        esp = esp.wrapping_sub(4); m.set_u32(esp, ecx);
        ecx = esp.wrapping_add(0x18);
        { let t = fs[fs.len() - 1 - 0]; let n = fs.len() - 1 - 1; let s = fs[n]; fs[n] = fsub(s, t); }
        fs.pop();
        { let v = fs.pop().unwrap(); m.set_f32(esp, v as f32); }
        call!(0x4bdef0);
        fs.push(m.f32(esp.wrapping_add(0xc)) as f64);
        { let b = m.f32(esp.wrapping_add(0x1c)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        fs.push(m.f32(esp.wrapping_add(0x18)) as f64);
        { let b = m.f32(esp.wrapping_add(0x10)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        esp = esp.wrapping_sub(4); m.set_u32(esp, ecx);
        { let t = fs[fs.len() - 1 - 0]; let n = fs.len() - 1 - 1; let s = fs[n]; fs[n] = fsub(s, t); }
        fs.pop();
        { let v = fs.pop().unwrap(); m.set_f32(esp, v as f32); }
        fs.push(m.f32(esp.wrapping_add(0x1c)) as f64);
        { let b = m.f32(esp.wrapping_add(0x18)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        fs.push(m.f32(esp.wrapping_add(0x10)) as f64);
        { let b = m.f32(esp.wrapping_add(0x24)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        esp = esp.wrapping_sub(4); m.set_u32(esp, ecx);
        { let t = fs[fs.len() - 1 - 0]; let n = fs.len() - 1 - 1; let s = fs[n]; fs[n] = fsub(s, t); }
        fs.pop();
        { let v = fs.pop().unwrap(); m.set_f32(esp, v as f32); }
        fs.push(m.f32(esp.wrapping_add(0x18)) as f64);
        { let b = m.f32(esp.wrapping_add(0x28)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        fs.push(m.f32(esp.wrapping_add(0x1c)) as f64);
        { let b = m.f32(esp.wrapping_add(0x24)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        esp = esp.wrapping_sub(4); m.set_u32(esp, ecx);
        ecx = esp.wrapping_add(0x30);
        { let t = fs[fs.len() - 1 - 0]; let n = fs.len() - 1 - 1; let s = fs[n]; fs[n] = fsub(s, t); }
        fs.pop();
        { let v = fs.pop().unwrap(); m.set_f32(esp, v as f32); }
        call!(0x4bdef0);
        fs.push(m.f32(esp.wrapping_add(0x44)) as f64);
        { let b = m.f32(esp.wrapping_add(0xb4)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        fs.push(m.f32(esp.wrapping_add(0x4c)) as f64);
        { let b = m.f32(esp.wrapping_add(0xbc)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        ecx = m.u32(esp.wrapping_add(0x24));
        edx = m.u32(esp.wrapping_add(0x28));
        esp = esp.wrapping_sub(4); m.set_u32(esp, ecx);
        eax = m.u32(esp.wrapping_add(0x30));
        { let t = fs[fs.len() - 1 - 0]; let n = fs.len() - 1 - 1; let s = fs[n]; fs[n] = fadd(s, t); }
        fs.pop();
        fs.push(m.f32(esp.wrapping_add(0x4c)) as f64);
        { let b = m.f32(esp.wrapping_add(0xbc)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        m.set_u32(esp.wrapping_add(0x218), ecx);
        m.set_u32(esp.wrapping_add(0x21c), edx);
        m.set_u32(esp.wrapping_add(0x220), eax);
        { let t = fs[fs.len() - 1 - 0]; let n = fs.len() - 1 - 1; let s = fs[n]; fs[n] = fadd(s, t); }
        fs.pop();
        fs.push(m.f32(esp.wrapping_add(0x50)) as f64);
        { let b = fs[fs.len() - 1 - 1]; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        { let v = fs.pop().unwrap(); m.set_f32(esp, v as f32); }
        fs.push(m.f32(esp.wrapping_add(0x4c)) as f64);
        { let b = fs[fs.len() - 1 - 1]; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        esp = esp.wrapping_sub(4); m.set_u32(esp, ecx);
        { let v = fs.pop().unwrap(); m.set_f32(esp, v as f32); }
        fs.push(m.f32(esp.wrapping_add(0x4c)) as f64);
        { let b = fs[fs.len() - 1 - 1]; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        esp = esp.wrapping_sub(4); m.set_u32(esp, ecx);
        ecx = esp.wrapping_add(0x30);
        { let v = fs.pop().unwrap(); m.set_f32(esp, v as f32); }
        fs.pop();
        call!(0x4bdef0);
        ecx = m.u32(esp.wrapping_add(0x24));
        edx = m.u32(esp.wrapping_add(0x28));
        eax = m.u32(esp.wrapping_add(0x2c));
        m.set_u32(esp.wrapping_add(0x178), ecx);
        fs.push(m.f32(esp.wrapping_add(0x18)) as f64);
        { let b = m.f32(esp.wrapping_add(0xb8)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        m.set_u32(esp.wrapping_add(0x17c), edx);
        m.set_u32(esp.wrapping_add(0x180), eax);
        fs.push(m.f32(esp.wrapping_add(0xb4)) as f64);
        { let b = m.f32(esp.wrapping_add(0x1c)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        esp = esp.wrapping_sub(4); m.set_u32(esp, ecx);
        { let t = fs[fs.len() - 1 - 0]; let n = fs.len() - 1 - 1; let s = fs[n]; fs[n] = fsub(s, t); }
        fs.pop();
        { let v = fs.pop().unwrap(); m.set_f32(esp, v as f32); }
        fs.push(m.f32(esp.wrapping_add(0xb8)) as f64);
        { let b = m.f32(esp.wrapping_add(0x24)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        fs.push(m.f32(esp.wrapping_add(0x1c)) as f64);
        { let b = m.f32(esp.wrapping_add(0xc0)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        esp = esp.wrapping_sub(4); m.set_u32(esp, ecx);
        { let t = fs[fs.len() - 1 - 0]; let n = fs.len() - 1 - 1; let s = fs[n]; fs[n] = fsub(s, t); }
        fs.pop();
        { let v = fs.pop().unwrap(); m.set_f32(esp, v as f32); }
        fs.push(m.f32(esp.wrapping_add(0x24)) as f64);
        { let b = m.f32(esp.wrapping_add(0xc4)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        fs.push(m.f32(esp.wrapping_add(0x28)) as f64);
        { let b = m.f32(esp.wrapping_add(0xc0)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        esp = esp.wrapping_sub(4); m.set_u32(esp, ecx);
        ecx = esp.wrapping_add(0x18);
        { let t = fs[fs.len() - 1 - 0]; let n = fs.len() - 1 - 1; let s = fs[n]; fs[n] = fsub(s, t); }
        fs.pop();
        { let v = fs.pop().unwrap(); m.set_f32(esp, v as f32); }
        call!(0x4bdef0);
        fs.push(m.f32(esp.wrapping_add(0xc)) as f64);
        { let b = m.f32(esp.wrapping_add(0x1c)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        fs.push(m.f32(esp.wrapping_add(0x18)) as f64);
        { let b = m.f32(esp.wrapping_add(0x10)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        esp = esp.wrapping_sub(4); m.set_u32(esp, ecx);
        { let t = fs[fs.len() - 1 - 0]; let n = fs.len() - 1 - 1; let s = fs[n]; fs[n] = fsub(s, t); }
        fs.pop();
        { let v = fs.pop().unwrap(); m.set_f32(esp, v as f32); }
        fs.push(m.f32(esp.wrapping_add(0x1c)) as f64);
        { let b = m.f32(esp.wrapping_add(0x18)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        fs.push(m.f32(esp.wrapping_add(0x10)) as f64);
        { let b = m.f32(esp.wrapping_add(0x24)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        esp = esp.wrapping_sub(4); m.set_u32(esp, ecx);
        { let t = fs[fs.len() - 1 - 0]; let n = fs.len() - 1 - 1; let s = fs[n]; fs[n] = fsub(s, t); }
        fs.pop();
        { let v = fs.pop().unwrap(); m.set_f32(esp, v as f32); }
        fs.push(m.f32(esp.wrapping_add(0x18)) as f64);
        { let b = m.f32(esp.wrapping_add(0x28)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        fs.push(m.f32(esp.wrapping_add(0x1c)) as f64);
        { let b = m.f32(esp.wrapping_add(0x24)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        esp = esp.wrapping_sub(4); m.set_u32(esp, ecx);
        ecx = esp.wrapping_add(0x7c);
        { let t = fs[fs.len() - 1 - 0]; let n = fs.len() - 1 - 1; let s = fs[n]; fs[n] = fsub(s, t); }
        fs.pop();
        { let v = fs.pop().unwrap(); m.set_f32(esp, v as f32); }
        call!(0x4bdef0);
        fs.push(m.f32(esp.wrapping_add(0x74)) as f64);
        { let b = m.f32(esp.wrapping_add(0x44)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        fs.push(m.f32(esp.wrapping_add(0x70)) as f64);
        { let b = m.f32(esp.wrapping_add(0x48)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        esp = esp.wrapping_sub(4); m.set_u32(esp, ecx);
        { let t = fs[fs.len() - 1 - 0]; let n = fs.len() - 1 - 1; let s = fs[n]; fs[n] = fsub(s, t); }
        fs.pop();
        { let v = fs.pop().unwrap(); m.set_f32(esp, v as f32); }
        fs.push(m.f32(esp.wrapping_add(0x74)) as f64);
        { let b = m.f32(esp.wrapping_add(0x50)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        fs.push(m.f32(esp.wrapping_add(0x7c)) as f64);
        { let b = m.f32(esp.wrapping_add(0x48)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        esp = esp.wrapping_sub(4); m.set_u32(esp, ecx);
        { let t = fs[fs.len() - 1 - 0]; let n = fs.len() - 1 - 1; let s = fs[n]; fs[n] = fsub(s, t); }
        fs.pop();
        { let v = fs.pop().unwrap(); m.set_f32(esp, v as f32); }
        fs.push(m.f32(esp.wrapping_add(0x80)) as f64);
        { let b = m.f32(esp.wrapping_add(0x50)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        fs.push(m.f32(esp.wrapping_add(0x7c)) as f64);
        { let b = m.f32(esp.wrapping_add(0x54)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        esp = esp.wrapping_sub(4); m.set_u32(esp, ecx);
        ecx = esp.wrapping_add(0x90);
        { let t = fs[fs.len() - 1 - 0]; let n = fs.len() - 1 - 1; let s = fs[n]; fs[n] = fsub(s, t); }
        fs.pop();
        { let v = fs.pop().unwrap(); m.set_f32(esp, v as f32); }
        call!(0x4bdef0);
        fs.push(m.f32(esp.wrapping_add(0x84)) as f64);
        { let b = m.f32(esp.wrapping_add(0x48)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        fs.push(m.f32(esp.wrapping_add(0x88)) as f64);
        { let b = m.f32(esp.wrapping_add(0x44)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        esp = esp.wrapping_sub(4); m.set_u32(esp, ecx);
        { let t = fs[fs.len() - 1 - 0]; let n = fs.len() - 1 - 1; let s = fs[n]; fs[n] = fsub(s, t); }
        fs.pop();
        { let v = fs.pop().unwrap(); m.set_f32(esp, v as f32); }
        fs.push(m.f32(esp.wrapping_add(0x90)) as f64);
        { let b = m.f32(esp.wrapping_add(0x48)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        fs.push(m.f32(esp.wrapping_add(0x88)) as f64);
        { let b = m.f32(esp.wrapping_add(0x50)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        esp = esp.wrapping_sub(4); m.set_u32(esp, ecx);
        { let t = fs[fs.len() - 1 - 0]; let n = fs.len() - 1 - 1; let s = fs[n]; fs[n] = fsub(s, t); }
        fs.pop();
        { let v = fs.pop().unwrap(); m.set_f32(esp, v as f32); }
        fs.push(m.f32(esp.wrapping_add(0x90)) as f64);
        { let b = m.f32(esp.wrapping_add(0x54)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        fs.push(m.f32(esp.wrapping_add(0x94)) as f64);
        { let b = m.f32(esp.wrapping_add(0x50)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        esp = esp.wrapping_sub(4); m.set_u32(esp, ecx);
        ecx = esp.wrapping_add(0x30);
        { let t = fs[fs.len() - 1 - 0]; let n = fs.len() - 1 - 1; let s = fs[n]; fs[n] = fsub(s, t); }
        fs.pop();
        { let v = fs.pop().unwrap(); m.set_f32(esp, v as f32); }
        call!(0x4bdef0);
        fs.push(m.f32(esp.wrapping_add(0xbc)) as f64);
        ecx = m.u32(esp.wrapping_add(0x24));
        edx = m.u32(esp.wrapping_add(0x28));
        { let b = m.f32(esp.wrapping_add(0x180)) as f64; let n = fs.len() - 1; fs[n] = fsub(fs[n], b); }
        esp = esp.wrapping_sub(4); m.set_u32(esp, ecx);
        eax = m.u32(esp.wrapping_add(0x30));
        m.set_u32(esp.wrapping_add(0x164), ecx);
        m.set_u32(esp.wrapping_add(0x168), edx);
        m.set_u32(esp.wrapping_add(0x16c), eax);
        { let v = fs.pop().unwrap(); m.set_f32(esp, v as f32); }
        fs.push(m.f32(esp.wrapping_add(0xbc)) as f64);
        { let b = m.f32(esp.wrapping_add(0x180)) as f64; let n = fs.len() - 1; fs[n] = fsub(fs[n], b); }
        esp = esp.wrapping_sub(4); m.set_u32(esp, ecx);
        { let v = fs.pop().unwrap(); m.set_f32(esp, v as f32); }
        fs.push(m.f32(esp.wrapping_add(0xbc)) as f64);
        { let b = m.f32(esp.wrapping_add(0x180)) as f64; let n = fs.len() - 1; fs[n] = fsub(fs[n], b); }
        esp = esp.wrapping_sub(4); m.set_u32(esp, ecx);
        ecx = esp.wrapping_add(0x30);
        { let v = fs.pop().unwrap(); m.set_f32(esp, v as f32); }
        call!(0x4bdef0);
        fs.push(m.f32(esp.wrapping_add(0x2c)) as f64);
        { let b = m.f32(esp.wrapping_add(0x168)) as f64; let n = fs.len() - 1; fs[n] = fsub(fs[n], b); }
        esp = esp.wrapping_sub(4); m.set_u32(esp, ecx);
        { let v = fs.pop().unwrap(); m.set_f32(esp, v as f32); }
        fs.push(m.f32(esp.wrapping_add(0x2c)) as f64);
        { let b = m.f32(esp.wrapping_add(0x168)) as f64; let n = fs.len() - 1; fs[n] = fsub(fs[n], b); }
        esp = esp.wrapping_sub(4); m.set_u32(esp, ecx);
        { let v = fs.pop().unwrap(); m.set_f32(esp, v as f32); }
        fs.push(m.f32(esp.wrapping_add(0x2c)) as f64);
        { let b = m.f32(esp.wrapping_add(0x168)) as f64; let n = fs.len() - 1; fs[n] = fsub(fs[n], b); }
        esp = esp.wrapping_sub(4); m.set_u32(esp, ecx);
        ecx = esp.wrapping_add(0x18);
        { let v = fs.pop().unwrap(); m.set_f32(esp, v as f32); }
        call!(0x4bdef0);
        fs.push(m.f32(esp.wrapping_add(0x18)) as f64);
        { let b = m.f32(esp.wrapping_add(0x38)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        fs.push(m.f32(esp.wrapping_add(0x34)) as f64);
        { let b = m.f32(esp.wrapping_add(0x1c)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        ecx = m.u32(esp.wrapping_add(0xc));
        edx = m.u32(esp.wrapping_add(0x10));
        esp = esp.wrapping_sub(4); m.set_u32(esp, ecx);
        eax = m.u32(esp.wrapping_add(0x18));
        { let t = fs[fs.len() - 1 - 0]; let n = fs.len() - 1 - 1; let s = fs[n]; fs[n] = fsub(s, t); }
        fs.pop();
        m.set_u32(esp.wrapping_add(0x224), ecx);
        m.set_u32(esp.wrapping_add(0x228), edx);
        m.set_u32(esp.wrapping_add(0x22c), eax);
        { let v = fs.pop().unwrap(); m.set_f32(esp, v as f32); }
        fs.push(m.f32(esp.wrapping_add(0x38)) as f64);
        { let b = m.f32(esp.wrapping_add(0x24)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        fs.push(m.f32(esp.wrapping_add(0x1c)) as f64);
        { let b = m.f32(esp.wrapping_add(0x40)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        esp = esp.wrapping_sub(4); m.set_u32(esp, ecx);
        { let t = fs[fs.len() - 1 - 0]; let n = fs.len() - 1 - 1; let s = fs[n]; fs[n] = fsub(s, t); }
        fs.pop();
        { let v = fs.pop().unwrap(); m.set_f32(esp, v as f32); }
        fs.push(m.f32(esp.wrapping_add(0x44)) as f64);
        { let b = m.f32(esp.wrapping_add(0x24)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        fs.push(m.f32(esp.wrapping_add(0x40)) as f64);
        { let b = m.f32(esp.wrapping_add(0x28)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        esp = esp.wrapping_sub(4); m.set_u32(esp, ecx);
        ecx = esp.wrapping_add(0x30);
        { let t = fs[fs.len() - 1 - 0]; let n = fs.len() - 1 - 1; let s = fs[n]; fs[n] = fsub(s, t); }
        fs.pop();
        { let v = fs.pop().unwrap(); m.set_f32(esp, v as f32); }
        call!(0x4bdef0);
        fs.push(m.f32(esp.wrapping_add(0x18)) as f64);
        ecx = m.u32(esp.wrapping_add(0x24));
        edx = m.u32(esp.wrapping_add(0x28));
        eax = m.u32(esp.wrapping_add(0x2c));
        m.set_u32(esp.wrapping_add(0xf4), ecx);
        { let b = m.f32(esp.wrapping_add(0x48)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        fs.push(m.f32(esp.wrapping_add(0x44)) as f64);
        m.set_u32(esp.wrapping_add(0xf8), edx);
        m.set_u32(esp.wrapping_add(0xfc), eax);
        { let b = m.f32(esp.wrapping_add(0x1c)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        esp = esp.wrapping_sub(4); m.set_u32(esp, ecx);
        { let t = fs[fs.len() - 1 - 0]; let n = fs.len() - 1 - 1; let s = fs[n]; fs[n] = fsub(s, t); }
        fs.pop();
        { let v = fs.pop().unwrap(); m.set_f32(esp, v as f32); }
        fs.push(m.f32(esp.wrapping_add(0x48)) as f64);
        { let b = m.f32(esp.wrapping_add(0x24)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        fs.push(m.f32(esp.wrapping_add(0x1c)) as f64);
        { let b = m.f32(esp.wrapping_add(0x50)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        esp = esp.wrapping_sub(4); m.set_u32(esp, ecx);
        { let t = fs[fs.len() - 1 - 0]; let n = fs.len() - 1 - 1; let s = fs[n]; fs[n] = fsub(s, t); }
        fs.pop();
        { let v = fs.pop().unwrap(); m.set_f32(esp, v as f32); }
        fs.push(m.f32(esp.wrapping_add(0x54)) as f64);
        { let b = m.f32(esp.wrapping_add(0x24)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        fs.push(m.f32(esp.wrapping_add(0x50)) as f64);
        { let b = m.f32(esp.wrapping_add(0x28)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        esp = esp.wrapping_sub(4); m.set_u32(esp, ecx);
        ecx = esp.wrapping_add(0x30);
        { let t = fs[fs.len() - 1 - 0]; let n = fs.len() - 1 - 1; let s = fs[n]; fs[n] = fsub(s, t); }
        fs.pop();
        { let v = fs.pop().unwrap(); m.set_f32(esp, v as f32); }
        call!(0x4bdef0);
        ecx = m.u32(esp.wrapping_add(0x24));
        edx = m.u32(esp.wrapping_add(0x28));
        eax = m.u32(esp.wrapping_add(0x2c));
        m.set_u32(esp.wrapping_add(0x10c), ecx);
        ecx = esp.wrapping_add(0xf4);
        m.set_u32(esp.wrapping_add(0x110), edx);
        esp = esp.wrapping_sub(4); m.set_u32(esp, ecx);
        m.set_u32(esp.wrapping_add(0x118), eax);
        call!(0x5ab4d0);
        { let v = fs.pop().unwrap(); m.set_f32(esp.wrapping_add(0x58), v as f32); }
        edx = esp.wrapping_add(0x110);
        esp = esp.wrapping_sub(4); m.set_u32(esp, edx);
        call!(0x5ab4d0);
        { let v = fs.pop().unwrap(); m.set_f32(esp.wrapping_add(0xa0), v as f32); }
        fs.push(m.f32(esp.wrapping_add(0x68)) as f64);
        esp = esp.wrapping_add(0x4);
        { let n = fs.len() - 1; fs[n] = -fs[n]; }
        { let v = fs.pop().unwrap(); m.set_f32(esp, v as f32); }
        fs.push(m.f32(esp.wrapping_add(0x60)) as f64);
        esp = esp.wrapping_sub(4); m.set_u32(esp, ecx);
        { let n = fs.len() - 1; fs[n] = -fs[n]; }
        { let v = fs.pop().unwrap(); m.set_f32(esp, v as f32); }
        fs.push(m.f32(esp.wrapping_add(0x60)) as f64);
        esp = esp.wrapping_sub(4); m.set_u32(esp, ecx);
        ecx = esp.wrapping_add(0x30);
        { let n = fs.len() - 1; fs[n] = -fs[n]; }
        { let v = fs.pop().unwrap(); m.set_f32(esp, v as f32); }
        call!(0x4bdef0);
        fs.push(m.f32(esp.wrapping_add(0x28)) as f64);
        { let b = m.f32(esp.wrapping_add(0x1c)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        fs.push(m.f32(esp.wrapping_add(0x2c)) as f64);
        { let b = m.f32(esp.wrapping_add(0x20)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        m.set_u32(esp.wrapping_add(0x80), 0x1);
        { let t = fs[fs.len() - 1 - 0]; let n = fs.len() - 1 - 1; let s = fs[n]; fs[n] = fadd(s, t); }
        fs.pop();
        fs.push(m.f32(esp.wrapping_add(0x18)) as f64);
        { let b = m.f32(esp.wrapping_add(0x24)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        { let t = fs[fs.len() - 1 - 0]; let n = fs.len() - 1 - 1; let s = fs[n]; fs[n] = fadd(s, t); }
        fs.pop();
        { let v = fs.pop().unwrap(); m.set_f32(esp.wrapping_add(0xb4), v as f32); }
        fs.push(m.f32(esp.wrapping_add(0xf4)) as f64);
        { let b = m.f32(esp.wrapping_add(0x148)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        fs.push(m.f32(esp.wrapping_add(0xfc)) as f64);
        { let b = m.f32(esp.wrapping_add(0x150)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        { let t = fs[fs.len() - 1 - 0]; let n = fs.len() - 1 - 1; let s = fs[n]; fs[n] = fadd(s, t); }
        fs.pop();
        fs.push(m.f32(esp.wrapping_add(0xf8)) as f64);
        { let b = m.f32(esp.wrapping_add(0x14c)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        { let t = fs[fs.len() - 1 - 0]; let n = fs.len() - 1 - 1; let s = fs[n]; fs[n] = fadd(s, t); }
        fs.pop();
        { let a = fs[fs.len() - 1 - 0]; let b = f64::from_bits(0x0000000000000000); sw = fcom(a, b); }
        fs.pop();
        eax = (eax & 0xffff0000) | sw;
        zf = ((eax >> 8) & 0x41) == 0;
        if zf { pc = 0x57fb4e; continue; }
        m.set_u32(esp.wrapping_add(0x80), 0xffffffff);
        pc = 0x57fb4e; continue;
    }
    0x57fb4e => {
        eax = esp.wrapping_add(0x148);
        esp = esp.wrapping_sub(4); m.set_u32(esp, eax);
        call!(0x5ab4d0);
        { let b = m.i32(esp.wrapping_add(0x84)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        { let v = fs.pop().unwrap(); m.set_f32(esp.wrapping_add(0x88), v as f32); }
        fs.push(m.f32(esp.wrapping_add(0xec)) as f64);
        { let b = m.f32(esp.wrapping_add(0xe8)) as f64; let n = fs.len() - 1; fs[n] = fadd(fs[n], b); }
        { let b = m.f32(esp.wrapping_add(0xe4)) as f64; let n = fs.len() - 1; fs[n] = fadd(fs[n], b); }
        { let b = f64::from_bits(0x3fd51eb851eb851f); let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        { let v = fs.pop().unwrap(); m.set_f32(esp.wrapping_add(0xc8), v as f32); }
        fs.push(m.f32(esp.wrapping_add(0x70)) as f64);
        { let n = fs.len() - 1; fs[n] = -fs[n]; }
        { let v = fs.pop().unwrap(); m.set_f32(esp, v as f32); }
        fs.push(m.f32(esp.wrapping_add(0x6c)) as f64);
        esp = esp.wrapping_sub(4); m.set_u32(esp, ecx);
        { let n = fs.len() - 1; fs[n] = -fs[n]; }
        { let v = fs.pop().unwrap(); m.set_f32(esp, v as f32); }
        fs.push(m.f32(esp.wrapping_add(0x6c)) as f64);
        esp = esp.wrapping_sub(4); m.set_u32(esp, ecx);
        ecx = esp.wrapping_add(0x30);
        { let n = fs.len() - 1; fs[n] = -fs[n]; }
        { let v = fs.pop().unwrap(); m.set_f32(esp, v as f32); }
        call!(0x4bdef0);
        fs.push(m.f32(esp.wrapping_add(0x2c)) as f64);
        { let b = m.f32(esp.wrapping_add(0x20)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        fs.push(m.f32(esp.wrapping_add(0x28)) as f64);
        { let b = m.f32(esp.wrapping_add(0x1c)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        m.set_u32(esp.wrapping_add(0xec), 0x1);
        { let t = fs[fs.len() - 1 - 0]; let n = fs.len() - 1 - 1; let s = fs[n]; fs[n] = fadd(s, t); }
        fs.pop();
        fs.push(m.f32(esp.wrapping_add(0x24)) as f64);
        { let b = m.f32(esp.wrapping_add(0x18)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        { let t = fs[fs.len() - 1 - 0]; let n = fs.len() - 1 - 1; let s = fs[n]; fs[n] = fadd(s, t); }
        fs.pop();
        { let v = fs.pop().unwrap(); m.set_f32(esp.wrapping_add(0x80), v as f32); }
        fs.push(m.f32(esp.wrapping_add(0x10c)) as f64);
        { let b = m.f32(esp.wrapping_add(0x160)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        fs.push(m.f32(esp.wrapping_add(0x114)) as f64);
        { let b = m.f32(esp.wrapping_add(0x168)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        { let t = fs[fs.len() - 1 - 0]; let n = fs.len() - 1 - 1; let s = fs[n]; fs[n] = fadd(s, t); }
        fs.pop();
        fs.push(m.f32(esp.wrapping_add(0x110)) as f64);
        { let b = m.f32(esp.wrapping_add(0x164)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        { let t = fs[fs.len() - 1 - 0]; let n = fs.len() - 1 - 1; let s = fs[n]; fs[n] = fadd(s, t); }
        fs.pop();
        { let a = fs[fs.len() - 1 - 0]; let b = f64::from_bits(0x0000000000000000); sw = fcom(a, b); }
        fs.pop();
        eax = (eax & 0xffff0000) | sw;
        zf = ((eax >> 8) & 0x41) == 0;
        if zf { pc = 0x57fc25; continue; }
        m.set_u32(esp.wrapping_add(0xec), 0xffffffff);
        pc = 0x57fc25; continue;
    }
    0x57fc25 => {
        ecx = esp.wrapping_add(0x160);
        esp = esp.wrapping_sub(4); m.set_u32(esp, ecx);
        call!(0x5ab4d0);
        { let b = m.i32(esp.wrapping_add(0xf0)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        esp = esp.wrapping_add(0x4);
        edx = esp.wrapping_add(0x1cc);
        eax = esp.wrapping_add(0xe0);
        ecx = esp.wrapping_add(0x22c);
        esp = esp.wrapping_sub(4); m.set_u32(esp, edx);
        esp = esp.wrapping_sub(4); m.set_u32(esp, eax);
        edx = esp.wrapping_add(0x33c);
        esp = esp.wrapping_sub(4); m.set_u32(esp, ecx);
        esp = esp.wrapping_sub(4); m.set_u32(esp, edx);
        { let v = fs.pop().unwrap(); m.set_f32(esp.wrapping_add(0x44), v as f32); }
        fs.push(m.f32(esp.wrapping_add(0x118)) as f64);
        { let b = m.f32(esp.wrapping_add(0x114)) as f64; let n = fs.len() - 1; fs[n] = fadd(fs[n], b); }
        { let b = m.f32(esp.wrapping_add(0x110)) as f64; let n = fs.len() - 1; fs[n] = fadd(fs[n], b); }
        { let b = f64::from_bits(0x3fd51eb851eb851f); let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        { let v = fs.pop().unwrap(); m.set_f32(esp.wrapping_add(0x54), v as f32); }
        call!(0x4e3170);
        esp = esp.wrapping_add(0xc);
        esp = esp.wrapping_sub(4); m.set_u32(esp, eax);
        eax = esp.wrapping_add(0x2dc);
        esp = esp.wrapping_sub(4); m.set_u32(esp, eax);
        call!(0x5aba40);
        esp = esp.wrapping_add(0xc);
        ecx = esp.wrapping_add(0x19c);
        edx = esp.wrapping_add(0x100);
        eax = esp.wrapping_add(0x304);
        esp = esp.wrapping_sub(4); m.set_u32(esp, ecx);
        esp = esp.wrapping_sub(4); m.set_u32(esp, edx);
        ecx = esp.wrapping_add(0x33c);
        esp = esp.wrapping_sub(4); m.set_u32(esp, eax);
        esp = esp.wrapping_sub(4); m.set_u32(esp, ecx);
        call!(0x4e3170);
        esp = esp.wrapping_add(0xc);
        edx = esp.wrapping_add(0x2a8);
        esp = esp.wrapping_sub(4); m.set_u32(esp, eax);
        esp = esp.wrapping_sub(4); m.set_u32(esp, edx);
        call!(0x5aba40);
        fs.push(m.f32(esp.wrapping_add(0x60)) as f64);
        { let a = fs[fs.len() - 1 - 0]; let b = f64::from_bits(0x3f50624dd2f1a9fc); sw = fcom(a, b); }
        fs.pop();
        eax = m.u32(0x702650);
        ecx = m.u32(0x702654);
        edx = m.u32(0x702658);
        m.set_u32(esp.wrapping_add(0x304), eax);
        m.set_u32(esp.wrapping_add(0x2d4), eax);
        esp = esp.wrapping_add(0xc);
        eax = (eax & 0xffff0000) | sw;
        m.set_u32(esp.wrapping_add(0x2fc), ecx);
        m.set_u32(esp.wrapping_add(0x300), edx);
        zf = ((eax >> 8) & 0x41) == 0;
        m.set_u32(esp.wrapping_add(0x2cc), ecx);
        m.set_u32(esp.wrapping_add(0x2d0), edx);
        if !zf { pc = 0x57fe1d; continue; }
        eax = m.u32(esp.wrapping_add(0x54));
        ecx = esp.wrapping_add(0x24);
        esp = esp.wrapping_sub(4); m.set_u32(esp, eax);
        esp = esp.wrapping_sub(4); m.set_u32(esp, ecx);
        ecx = esp.wrapping_add(0xfc);
        call!(0x5ab660);
        fs.push(m.f32(esp.wrapping_add(0xe8)) as f64);
        { let b = m.f32(esp.wrapping_add(0x24c)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        fs.push(m.f32(esp.wrapping_add(0xe4)) as f64);
        { let b = m.f32(esp.wrapping_add(0x240)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        esp = esp.wrapping_sub(4); m.set_u32(esp, ecx);
        { let t = fs[fs.len() - 1 - 0]; let n = fs.len() - 1 - 1; let s = fs[n]; fs[n] = fadd(s, t); }
        fs.pop();
        fs.push(m.f32(esp.wrapping_add(0xe4)) as f64);
        { let b = m.f32(esp.wrapping_add(0x238)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        { let t = fs[fs.len() - 1 - 0]; let n = fs.len() - 1 - 1; let s = fs[n]; fs[n] = fadd(s, t); }
        fs.pop();
        { let v = fs.pop().unwrap(); m.set_f32(esp, v as f32); }
        fs.push(m.f32(esp.wrapping_add(0xec)) as f64);
        { let b = m.f32(esp.wrapping_add(0x24c)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        fs.push(m.f32(esp.wrapping_add(0xe8)) as f64);
        { let b = m.f32(esp.wrapping_add(0x240)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        esp = esp.wrapping_sub(4); m.set_u32(esp, ecx);
        { let t = fs[fs.len() - 1 - 0]; let n = fs.len() - 1 - 1; let s = fs[n]; fs[n] = fadd(s, t); }
        fs.pop();
        fs.push(m.f32(esp.wrapping_add(0xe8)) as f64);
        { let b = m.f32(esp.wrapping_add(0x238)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        { let t = fs[fs.len() - 1 - 0]; let n = fs.len() - 1 - 1; let s = fs[n]; fs[n] = fadd(s, t); }
        fs.pop();
        { let v = fs.pop().unwrap(); m.set_f32(esp, v as f32); }
        fs.push(m.f32(esp.wrapping_add(0xf0)) as f64);
        { let b = m.f32(esp.wrapping_add(0x24c)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        fs.push(m.f32(esp.wrapping_add(0xec)) as f64);
        { let b = m.f32(esp.wrapping_add(0x240)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        esp = esp.wrapping_sub(4); m.set_u32(esp, ecx);
        ecx = esp.wrapping_add(0x30);
        { let t = fs[fs.len() - 1 - 0]; let n = fs.len() - 1 - 1; let s = fs[n]; fs[n] = fadd(s, t); }
        fs.pop();
        fs.push(m.f32(esp.wrapping_add(0x238)) as f64);
        { let b = m.f32(esp.wrapping_add(0xec)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        { let t = fs[fs.len() - 1 - 0]; let n = fs.len() - 1 - 1; let s = fs[n]; fs[n] = fadd(s, t); }
        fs.pop();
        { let v = fs.pop().unwrap(); m.set_f32(esp, v as f32); }
        call!(0x4bdef0);
        edx = esp.wrapping_add(0x2d4);
        eax = esp.wrapping_add(0xf4);
        esp = esp.wrapping_sub(4); m.set_u32(esp, edx);
        ecx = esp.wrapping_add(0x28);
        esp = esp.wrapping_sub(4); m.set_u32(esp, eax);
        esp = esp.wrapping_sub(4); m.set_u32(esp, ecx);
        call!(0x4cb2b0);
        fs.push(m.f32(eax.wrapping_add(0x8)) as f64);
        { let b = m.f32(esp.wrapping_add(0x108)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        fs.push(m.f32(eax.wrapping_add(0x4)) as f64);
        { let b = m.f32(esp.wrapping_add(0x104)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        esp = esp.wrapping_add(0xc);
        { let t = fs[fs.len() - 1 - 0]; let n = fs.len() - 1 - 1; let s = fs[n]; fs[n] = fadd(s, t); }
        fs.pop();
        fs.push(m.f32(esp.wrapping_add(0xf4)) as f64);
        { let b = m.f32(eax) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        { let t = fs[fs.len() - 1 - 0]; let n = fs.len() - 1 - 1; let s = fs[n]; fs[n] = fadd(s, t); }
        fs.pop();
        { let n = fs.len() - 1; fs[n] = fs[n].abs(); }
        { let v = fs.pop().unwrap(); m.set_f32(esp.wrapping_add(0xc4), v as f32); }
        pc = 0x57fe1d; continue;
    }
    0x57fe1d => {
        fs.push(m.f32(esp.wrapping_add(0x98)) as f64);
        { let a = fs[fs.len() - 1 - 0]; let b = f64::from_bits(0x3f50624dd2f1a9fc); sw = fcom(a, b); }
        fs.pop();
        eax = (eax & 0xffff0000) | sw;
        zf = ((eax >> 8) & 0x41) == 0;
        if !zf { pc = 0x57feac; continue; }
        edx = m.u32(esp.wrapping_add(0x98));
        eax = esp.wrapping_add(0x24);
        esp = esp.wrapping_sub(4); m.set_u32(esp, edx);
        esp = esp.wrapping_sub(4); m.set_u32(esp, eax);
        ecx = esp.wrapping_add(0x114);
        call!(0x5ab660);
        ecx = esp.wrapping_add(0x10c);
        edx = esp.wrapping_add(0x304);
        esp = esp.wrapping_sub(4); m.set_u32(esp, ecx);
        eax = esp.wrapping_add(0x104);
        esp = esp.wrapping_sub(4); m.set_u32(esp, edx);
        ecx = esp.wrapping_add(0x2c);
        esp = esp.wrapping_sub(4); m.set_u32(esp, eax);
        esp = esp.wrapping_sub(4); m.set_u32(esp, ecx);
        call!(0x4cb2b0);
        esp = esp.wrapping_add(0xc);
        esp = esp.wrapping_sub(4); m.set_u32(esp, eax);
        call!(0x610f00);
        esp = esp.wrapping_add(0x8);
        edx = esp.wrapping_add(0x10c);
        eax = esp.wrapping_add(0x2a4);
        ecx = esp.wrapping_add(0x10c);
        esp = esp.wrapping_sub(4); m.set_u32(esp, edx);
        esp = esp.wrapping_sub(4); m.set_u32(esp, eax);
        edx = esp.wrapping_add(0x2c);
        esp = esp.wrapping_sub(4); m.set_u32(esp, ecx);
        esp = esp.wrapping_sub(4); m.set_u32(esp, edx);
        fs.pop();
        call!(0x4cb2b0);
        esp = esp.wrapping_add(0xc);
        esp = esp.wrapping_sub(4); m.set_u32(esp, eax);
        call!(0x610f00);
        esp = esp.wrapping_add(0x8);
        { let n = fs.len() - 1; fs[n] = fs[n].abs(); }
        pc = 0x57feb0; continue;
        pc = 0x57feac; continue;
    }
    0x57feac => {
        fs.push(m.f32(esp.wrapping_add(0x44)) as f64);
        pc = 0x57feb0; continue;
    }
    0x57feb0 => {
        fs.push(m.f32(esp.wrapping_add(0x54)) as f64);
        { let b = m.f32(esp.wrapping_add(0xdc)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        esp = esp.wrapping_sub(4); m.set_u32(esp, ecx);
        eax = esp.wrapping_add(0x1c);
        ecx = esp.wrapping_add(0x26c);
        { let b = m.f32(esp.wrapping_add(0x80)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        { let b = m.f32(esp.wrapping_add(0xc8)) as f64; let n = fs.len() - 1; fs[n] = fdiv(fs[n], b); }
        { let v = fs.pop().unwrap(); m.set_f32(esp.wrapping_add(0xc8), v as f32); }
        fs.push(m.f32(esp.wrapping_add(0x9c)) as f64);
        { let b = m.f32(esp.wrapping_add(0xdc)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        { let b = m.f32(esp.wrapping_add(0x34)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        { let b = fs[fs.len() - 1 - 1]; let n = fs.len() - 1; fs[n] = fdiv(fs[n], b); }
        { let v = fs.pop().unwrap(); m.set_f32(esp.wrapping_add(0x48), v as f32); }
        fs.pop();
        fs.push(m.f32(esp.wrapping_add(0x48)) as f64);
        { let b = m.f32(esp.wrapping_add(0x9c)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        { let b = f32::from_bits(0x3f800000) as f64; let n = fs.len() - 1; fs[n] = fadd(fs[n], b); }
        { let b = m.f32(esp.wrapping_add(0x80)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        { let b = m.f32(esp.wrapping_add(0x34)) as f64; let n = fs.len() - 1; fs[n] = fdiv(fs[n], b); }
        fs.push(m.f32(esp.wrapping_add(0xc8)) as f64);
        { let b = m.f32(esp.wrapping_add(0x58)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        { let t = fs[fs.len() - 1 - 0]; let n = fs.len() - 1 - 1; let s = fs[n]; fs[n] = fadd(s, t); }
        fs.pop();
        fs.push(m.f32(esp.wrapping_add(0x38)) as f64);
        { let b = m.f32(esp.wrapping_add(0x9c)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        { let b = m.f32(esp.wrapping_add(0xdc)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        { let b = m.f32(esp.wrapping_add(0x84)) as f64; let n = fs.len() - 1; fs[n] = fadd(fs[n], b); }
        fs.push(m.f32(esp.wrapping_add(0xf4)) as f64);
        { let b = f32::from_bits(0x3f800000) as f64; let n = fs.len() - 1; fs[n] = fadd(fs[n], b); }
        { let t = fs[fs.len() - 1 - 0]; let n = fs.len() - 1 - 1; let s = fs[n]; fs[n] = fmul(s, t); }
        fs.pop();
        fs.push(m.f32(esp.wrapping_add(0xf4)) as f64);
        { let n = fs.len() - 1; fs[n] = -fs[n]; }
        { let b = f32::from_bits(0x3f800000) as f64; let n = fs.len() - 1; fs[n] = fsub(fs[n], b); }
        { let b = m.f32(esp.wrapping_add(0x88)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        { let b = m.f32(esp.wrapping_add(0x58)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        { let b = m.f32(esp.wrapping_add(0xe0)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        { let t = fs[fs.len() - 1 - 0]; let n = fs.len() - 1 - 1; let s = fs[n]; fs[n] = fadd(s, t); }
        fs.pop();
        { let v = fs[fs.len() - 1 - 1]; fs.push(v); }
        { let b = m.f32(esp.wrapping_add(0xf4)) as f64; let n = fs.len() - 1; fs[n] = fsub(fs[n], b); }
        { let b = m.f32(esp.wrapping_add(0xb8)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        { let t = fs[fs.len() - 1 - 0]; let n = fs.len() - 1 - 1; let s = fs[n]; fs[n] = fadd(s, t); }
        fs.pop();
        { let n = fs.len() - 1; fs.swap(n, n - 1); }
        { let b = f32::from_bits(0x3f800000) as f64; let n = fs.len() - 1; fs[n] = fadd(fs[n], b); }
        { let t = fs[fs.len() - 1 - 0]; let n = fs.len() - 1 - 1; let s = fs[n]; fs[n] = fdiv(s, t); }
        fs.pop();
        { let b = f64::from_bits(0x3fc999999999999a); let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        { let v = fs[fs.len() - 1 - 0]; fs.push(v); }
        { let b = m.f32(esp.wrapping_add(0xb8)) as f64; let n = fs.len() - 1; fs[n] = fsub(fs[n], b); }
        { let v = fs.pop().unwrap(); m.set_f32(esp.wrapping_add(0xb8), v as f32); }
        fs.push(m.f32(esp.wrapping_add(0x80)) as f64);
        { let b = m.f32(esp.wrapping_add(0x34)) as f64; let n = fs.len() - 1; fs[n] = fdiv(fs[n], b); }
        { let b = m.f32(esp.wrapping_add(0xb8)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        { let b = m.f32(esp.wrapping_add(0x84)) as f64; let n = fs.len() - 1; fs[n] = fsub(b, fs[n]); }
        { let v = fs.pop().unwrap(); m.set_f32(esp.wrapping_add(0xf0), v as f32); }
        { let n = fs.len() - 1; fs[n] = fs[n].abs(); }
        { let v = fs.pop().unwrap(); m.set_f32(esp, v as f32); }
        esp = esp.wrapping_sub(4); m.set_u32(esp, eax);
        esp = esp.wrapping_sub(4); m.set_u32(esp, ecx);
        call!(0x4bdd60);
        edx = esp.wrapping_add(0x214);
        eax = esp.wrapping_add(0x274);
        esp = esp.wrapping_sub(4); m.set_u32(esp, edx);
        ecx = esp.wrapping_add(0x164);
        esp = esp.wrapping_sub(4); m.set_u32(esp, eax);
        esp = esp.wrapping_sub(4); m.set_u32(esp, ecx);
        call!(0x4bddb0);
        fs.push(m.f32(esp.wrapping_add(0x104)) as f64);
        esp = esp.wrapping_add(0x14);
        edx = esp.wrapping_add(0x1c);
        { let n = fs.len() - 1; fs[n] = fs[n].abs(); }
        { let v = fs.pop().unwrap(); m.set_f32(esp, v as f32); }
        eax = esp.wrapping_add(0x28);
        esp = esp.wrapping_sub(4); m.set_u32(esp, edx);
        esp = esp.wrapping_sub(4); m.set_u32(esp, eax);
        call!(0x4cb270);
        esp = esp.wrapping_add(0x8);
        esp = esp.wrapping_sub(4); m.set_u32(esp, eax);
        ecx = esp.wrapping_add(0x264);
        esp = esp.wrapping_sub(4); m.set_u32(esp, ecx);
        call!(0x4bdd60);
        edx = esp.wrapping_add(0x220);
        eax = esp.wrapping_add(0x268);
        esp = esp.wrapping_sub(4); m.set_u32(esp, edx);
        ecx = esp.wrapping_add(0x128);
        esp = esp.wrapping_sub(4); m.set_u32(esp, eax);
        esp = esp.wrapping_sub(4); m.set_u32(esp, ecx);
        call!(0x4bddb0);
        fs.push(m.f32(esp.wrapping_add(0xdc)) as f64);
        { let b = m.f32(esp.wrapping_add(0xf4)) as f64; let n = fs.len() - 1; fs[n] = fdiv(fs[n], b); }
        esp = esp.wrapping_add(0x14);
        edx = esp.wrapping_add(0xf8);
        eax = esp.wrapping_add(0x284);
        { let b = m.f32(esp.wrapping_add(0xb8)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        { let b = m.f32(esp.wrapping_add(0x88)) as f64; let n = fs.len() - 1; fs[n] = fadd(fs[n], b); }
        { let v = fs.pop().unwrap(); m.set_f32(esp, v as f32); }
        esp = esp.wrapping_sub(4); m.set_u32(esp, edx);
        esp = esp.wrapping_sub(4); m.set_u32(esp, eax);
        call!(0x4bdd60);
        esp = esp.wrapping_add(0xc);
        ecx = esp.wrapping_add(0x280);
        edx = esp.wrapping_add(0x1fc);
        eax = esp.wrapping_add(0x190);
        esp = esp.wrapping_sub(4); m.set_u32(esp, ecx);
        esp = esp.wrapping_sub(4); m.set_u32(esp, edx);
        ecx = esp.wrapping_add(0x2c);
        esp = esp.wrapping_sub(4); m.set_u32(esp, eax);
        esp = esp.wrapping_sub(4); m.set_u32(esp, ecx);
        call!(0x4bddb0);
        esp = esp.wrapping_add(0xc);
        edx = esp.wrapping_add(0x29c);
        esp = esp.wrapping_sub(4); m.set_u32(esp, eax);
        esp = esp.wrapping_sub(4); m.set_u32(esp, edx);
        call!(0x4bddb0);
        eax = esp.wrapping_add(0x238);
        ecx = esp.wrapping_add(0x2a4);
        esp = esp.wrapping_sub(4); m.set_u32(esp, eax);
        edx = esp.wrapping_add(0x140);
        esp = esp.wrapping_sub(4); m.set_u32(esp, ecx);
        esp = esp.wrapping_sub(4); m.set_u32(esp, edx);
        call!(0x4cb2b0);
        fs.push(m.f32(esp.wrapping_add(0x104)) as f64);
        { let b = m.f32(esp.wrapping_add(0x98)) as f64; let n = fs.len() - 1; fs[n] = fsub(fs[n], b); }
        fs.push(m.f32(esp.wrapping_add(0x5c)) as f64);
        { let b = m.f32(esp.wrapping_add(0xf0)) as f64; let n = fs.len() - 1; fs[n] = fdiv(fs[n], b); }
        esp = esp.wrapping_add(0x14);
        eax = esp.wrapping_add(0x110);
        ecx = esp.wrapping_add(0x278);
        { let t = fs[fs.len() - 1 - 0]; let n = fs.len() - 1 - 1; let s = fs[n]; fs[n] = fmul(s, t); }
        fs.pop();
        { let b = m.f32(esp.wrapping_add(0x38)) as f64; let n = fs.len() - 1; fs[n] = fadd(fs[n], b); }
        { let v = fs.pop().unwrap(); m.set_f32(esp, v as f32); }
        esp = esp.wrapping_sub(4); m.set_u32(esp, eax);
        esp = esp.wrapping_sub(4); m.set_u32(esp, ecx);
        call!(0x4bdd60);
        esp = esp.wrapping_add(0xc);
        edx = esp.wrapping_add(0x274);
        eax = esp.wrapping_add(0x220);
        ecx = esp.wrapping_add(0x178);
        esp = esp.wrapping_sub(4); m.set_u32(esp, edx);
        esp = esp.wrapping_sub(4); m.set_u32(esp, eax);
        edx = esp.wrapping_add(0x2c);
        esp = esp.wrapping_sub(4); m.set_u32(esp, ecx);
        esp = esp.wrapping_sub(4); m.set_u32(esp, edx);
        call!(0x4bddb0);
        esp = esp.wrapping_add(0xc);
        esp = esp.wrapping_sub(4); m.set_u32(esp, eax);
        eax = esp.wrapping_add(0x294);
        esp = esp.wrapping_sub(4); m.set_u32(esp, eax);
        call!(0x4bddb0);
        ecx = esp.wrapping_add(0x310);
        edx = esp.wrapping_add(0x298);
        esp = esp.wrapping_sub(4); m.set_u32(esp, ecx);
        eax = esp.wrapping_add(0x134);
        esp = esp.wrapping_sub(4); m.set_u32(esp, edx);
        esp = esp.wrapping_sub(4); m.set_u32(esp, eax);
        call!(0x4cb2b0);
        esp = esp.wrapping_add(0x18);
        ecx = esp.wrapping_add(0x16c);
        edx = esp.wrapping_add(0x24);
        esp = esp.wrapping_sub(4); m.set_u32(esp, ecx);
        esp = esp.wrapping_sub(4); m.set_u32(esp, edx);
        ecx = esp.wrapping_add(0x15c);
        call!(0x5ab710);
        eax = esp.wrapping_add(0x16c);
        ecx = esp.wrapping_add(0x24);
        esp = esp.wrapping_sub(4); m.set_u32(esp, eax);
        esp = esp.wrapping_sub(4); m.set_u32(esp, ecx);
        ecx = esp.wrapping_add(0x120);
        call!(0x5ab710);
        eax = ebx.wrapping_add(0xcf0);
        edx = esp.wrapping_add(0xe0);
        esp = esp.wrapping_sub(4); m.set_u32(esp, eax);
        esp = esp.wrapping_sub(4); m.set_u32(esp, eax);
        eax = esp.wrapping_add(0x2c);
        esp = esp.wrapping_sub(4); m.set_u32(esp, edx);
        esp = esp.wrapping_sub(4); m.set_u32(esp, eax);
        call!(0x4e30f4);
        esp = esp.wrapping_add(0xc);
        esp = esp.wrapping_sub(4); m.set_u32(esp, eax);
        call!(0x610f00);
        eax = ebx.wrapping_add(0xce4);
        { let v = fs.pop().unwrap(); m.set_f64(esp.wrapping_add(0x3c), v); }
        esp = esp.wrapping_sub(4); m.set_u32(esp, eax);
        esp = esp.wrapping_sub(4); m.set_u32(esp, eax);
        call!(0x610f00);
        { let b = m.f32(esp.wrapping_add(0x8c)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        esp = esp.wrapping_add(0x10);
        eax = esi.wrapping_add(0xcf0);
        ecx = esp.wrapping_add(0x100);
        edx = esp.wrapping_add(0x24);
        { let b = m.f64(esp.wrapping_add(0x34)); let n = fs.len() - 1; fs[n] = fadd(fs[n], b); }
        esp = esp.wrapping_sub(4); m.set_u32(esp, eax);
        esp = esp.wrapping_sub(4); m.set_u32(esp, eax);
        esp = esp.wrapping_sub(4); m.set_u32(esp, ecx);
        esp = esp.wrapping_sub(4); m.set_u32(esp, edx);
        { let b = f64::from_bits(0x3fe0000000000000); let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        { let v = fs.pop().unwrap(); m.set_f32(esp.wrapping_add(0x54), v as f32); }
        call!(0x4e30f4);
        esp = esp.wrapping_add(0xc);
        esp = esp.wrapping_sub(4); m.set_u32(esp, eax);
        call!(0x610f00);
        eax = esi.wrapping_add(0xce4);
        { let v = fs.pop().unwrap(); m.set_f64(esp.wrapping_add(0x3c), v); }
        esp = esp.wrapping_sub(4); m.set_u32(esp, eax);
        esp = esp.wrapping_sub(4); m.set_u32(esp, eax);
        call!(0x610f00);
        { let b = m.f32(esp.wrapping_add(0x40)) as f64; let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        esp = esp.wrapping_add(0x10);
        eax = esp.wrapping_add(0x130);
        ecx = esp.wrapping_add(0x130);
        edx = esp.wrapping_add(0xe0);
        { let b = m.f64(esp.wrapping_add(0x34)); let n = fs.len() - 1; fs[n] = fadd(fs[n], b); }
        esp = esp.wrapping_sub(4); m.set_u32(esp, eax);
        esp = esp.wrapping_sub(4); m.set_u32(esp, ecx);
        eax = esp.wrapping_add(0x2c);
        esp = esp.wrapping_sub(4); m.set_u32(esp, edx);
        { let b = f64::from_bits(0x3fe0000000000000); let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        esp = esp.wrapping_sub(4); m.set_u32(esp, eax);
        { let v = fs.pop().unwrap(); m.set_f32(esp.wrapping_add(0x44), v as f32); }
        fs.push(m.f32(esp.wrapping_add(0x8c)) as f64);
        { let v = fs.pop().unwrap(); m.set_f64(esp.wrapping_add(0xc4), v); }
        call!(0x4e30f4);
        esp = esp.wrapping_add(0xc);
        esp = esp.wrapping_sub(4); m.set_u32(esp, eax);
        call!(0x610f00);
        ecx = esp.wrapping_add(0x15c);
        edx = esp.wrapping_add(0x15c);
        { let v = fs.pop().unwrap(); m.set_f64(esp.wrapping_add(0x8c), v); }
        esp = esp.wrapping_sub(4); m.set_u32(esp, ecx);
        esp = esp.wrapping_sub(4); m.set_u32(esp, edx);
        call!(0x610f00);
        esp = esp.wrapping_add(0x10);
        { let b = m.f64(esp.wrapping_add(0xb4)); let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        eax = esp.wrapping_add(0x124);
        ecx = esp.wrapping_add(0x124);
        esp = esp.wrapping_sub(4); m.set_u32(esp, eax);
        edx = esp.wrapping_add(0x104);
        { let b = m.f64(esp.wrapping_add(0x88)); let n = fs.len() - 1; fs[n] = fadd(fs[n], b); }
        esp = esp.wrapping_sub(4); m.set_u32(esp, ecx);
        eax = esp.wrapping_add(0x2c);
        esp = esp.wrapping_sub(4); m.set_u32(esp, edx);
        esp = esp.wrapping_sub(4); m.set_u32(esp, eax);
        { let b = f64::from_bits(0x3fe0000000000000); let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        { let v = fs.pop().unwrap(); m.set_f32(esp.wrapping_add(0x94), v as f32); }
        fs.push(m.f32(esp.wrapping_add(0x44)) as f64);
        { let b = m.f32(esp.wrapping_add(0x54)) as f64; let n = fs.len() - 1; fs[n] = fadd(fs[n], b); }
        { let v = fs.pop().unwrap(); m.set_f32(esp.wrapping_add(0x44), v as f32); }
        fs.push(m.f32(esp.wrapping_add(0x40)) as f64);
        { let v = fs.pop().unwrap(); m.set_f64(esp.wrapping_add(0x54), v); }
        call!(0x4e30f4);
        esp = esp.wrapping_add(0xc);
        esp = esp.wrapping_sub(4); m.set_u32(esp, eax);
        call!(0x610f00);
        ecx = esp.wrapping_add(0x120);
        edx = esp.wrapping_add(0x120);
        { let v = fs.pop().unwrap(); m.set_f64(esp.wrapping_add(0xcc), v); }
        esp = esp.wrapping_sub(4); m.set_u32(esp, ecx);
        esp = esp.wrapping_sub(4); m.set_u32(esp, edx);
        call!(0x610f00);
        { let b = m.f64(esp.wrapping_add(0x54)); let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        esp = esp.wrapping_add(0x10);
        { let b = m.f64(esp.wrapping_add(0xc4)); let n = fs.len() - 1; fs[n] = fadd(fs[n], b); }
        { let b = f64::from_bits(0x3fe0000000000000); let n = fs.len() - 1; fs[n] = fmul(fs[n], b); }
        { let b = m.f32(esp.wrapping_add(0x84)) as f64; let n = fs.len() - 1; fs[n] = fadd(fs[n], b); }
        { let b = m.f32(esp.wrapping_add(0x34)) as f64; let n = fs.len() - 1; fs[n] = fdiv(fs[n], b); }
        { let v = fs.pop().unwrap(); m.set_f32(esp.wrapping_add(0x30), v as f32); }
        fs.push(m.f32(esp.wrapping_add(0x34)) as f64);
        { let a = fs[fs.len() - 1 - 0]; let b = f64::from_bits(0x0000000000000000); sw = fcom(a, b); }
        fs.pop();
        eax = (eax & 0xffff0000) | sw;
        zf = ((eax >> 8) & 0x41) == 0;
        if !zf { pc = 0x580391; continue; }
        fs.push(m.f32(esp.wrapping_add(0x30)) as f64);
        { let a = fs[fs.len() - 1 - 0]; let b = f64::from_bits(0x3ff028f5c28f5c29); sw = fcom(a, b); }
        fs.pop();
        eax = (eax & 0xffff0000) | sw;
        zf = ((eax >> 8) & 0x41) == 0;
        if !zf { pc = 0x580391; continue; }
        fs.push(m.f32(esp.wrapping_add(0x30)) as f64);
        { let b = f64::from_bits(0x3fb999999999999a); let n = fs.len() - 1; fs[n] = fadd(fs[n], b); }
        ecx = esp.wrapping_add(0x24);
        { let b = f64::from_bits(0x3ff0000000000000); let n = fs.len() - 1; fs[n] = fdiv(b, fs[n]); }
        { let v = fs.pop().unwrap(); m.set_f32(esp.wrapping_add(0x30), v as f32); }
        eax = m.u32(esp.wrapping_add(0x30));
        esp = esp.wrapping_sub(4); m.set_u32(esp, eax);
        esp = esp.wrapping_sub(4); m.set_u32(esp, ecx);
        ecx = esp.wrapping_add(0x15c);
        call!(0x5ab620);
        edx = m.u32(esp.wrapping_add(0x30));
        eax = esp.wrapping_add(0x24);
        esp = esp.wrapping_sub(4); m.set_u32(esp, edx);
        esp = esp.wrapping_sub(4); m.set_u32(esp, eax);
        ecx = esp.wrapping_add(0x120);
        call!(0x5ab620);
        ecx = m.u32(esp.wrapping_add(0x30));
        edx = esp.wrapping_add(0x24);
        esp = esp.wrapping_sub(4); m.set_u32(esp, ecx);
        esp = esp.wrapping_sub(4); m.set_u32(esp, edx);
        ecx = esp.wrapping_add(0x138);
        call!(0x5ab620);
        eax = m.u32(esp.wrapping_add(0x30));
        ecx = esp.wrapping_add(0x24);
        esp = esp.wrapping_sub(4); m.set_u32(esp, eax);
        esp = esp.wrapping_sub(4); m.set_u32(esp, ecx);
        ecx = esp.wrapping_add(0x12c);
        call!(0x5ab620);
        pc = 0x580391; continue;
    }
    0x580391 => {
        edx = esp.wrapping_add(0x130);
        esp = esp.wrapping_sub(4); m.set_u32(esp, edx);
        call!(0x4e794f);
        eax = esp.wrapping_add(0x128);
        esp = esp.wrapping_sub(4); m.set_u32(esp, eax);
        call!(0x4e794f);
        ecx = esp.wrapping_add(0x15c);
        esp = esp.wrapping_sub(4); m.set_u32(esp, 0x43160000);
        esp = esp.wrapping_sub(4); m.set_u32(esp, ecx);
        call!(0x4e28b0);
        esp = esp.wrapping_add(0x10);
        zf = (eax & eax) == 0;
        if !zf { pc = 0x580413; continue; }
        edx = esp.wrapping_add(0x118);
        esp = esp.wrapping_sub(4); m.set_u32(esp, 0x43160000);
        esp = esp.wrapping_sub(4); m.set_u32(esp, edx);
        call!(0x4e28b0);
        esp = esp.wrapping_add(0x8);
        zf = (eax & eax) == 0;
        if !zf { pc = 0x580413; continue; }
        eax = esp.wrapping_add(0x130);
        esp = esp.wrapping_sub(4); m.set_u32(esp, 0x41f00000);
        esp = esp.wrapping_sub(4); m.set_u32(esp, eax);
        call!(0x4e28b0);
        esp = esp.wrapping_add(0x8);
        zf = (eax & eax) == 0;
        if !zf { pc = 0x580413; continue; }
        ecx = esp.wrapping_add(0x124);
        esp = esp.wrapping_sub(4); m.set_u32(esp, 0x41f00000);
        esp = esp.wrapping_sub(4); m.set_u32(esp, ecx);
        call!(0x4e28b0);
        esp = esp.wrapping_add(0x8);
        zf = (eax & eax) == 0;
        if zf { pc = 0x580851; continue; }
        pc = 0x580413; continue;
    }
    0x580413 => {
        edx = m.u32(esp.wrapping_add(0x48));
        eax = m.u32(esp.wrapping_add(0x44));
        ecx = m.u32(esp.wrapping_add(0xb8));
        esp = esp.wrapping_sub(4); m.set_u32(esp, edx);
        edx = m.u32(esp.wrapping_add(0xb8));
        esp = esp.wrapping_sub(4); m.set_u32(esp, eax);
        fs.push(m.f32(esp.wrapping_add(0xf8)) as f64);
        esp = esp.wrapping_sub(4); m.set_u32(esp, ecx);
        esp = esp.wrapping_sub(4); m.set_u32(esp, edx);
        esp = esp.wrapping_sub(0x8);
        eax = esp.wrapping_add(0x764);
        { let v = fs.pop().unwrap(); m.set_f64(esp, v); }
        fs.push(m.f32(edi.wrapping_add(0x48)) as f64);
        esp = esp.wrapping_sub(0x8);
        { let v = fs.pop().unwrap(); m.set_f64(esp, v); }
        fs.push(m.f32(edi.wrapping_add(0x44)) as f64);
        esp = esp.wrapping_sub(0x8);
        { let v = fs.pop().unwrap(); m.set_f64(esp, v); }
        fs.push(m.f32(edi.wrapping_add(0x40)) as f64);
        esp = esp.wrapping_sub(0x8);
        { let v = fs.pop().unwrap(); m.set_f64(esp, v); }
        fs.push(m.f32(edi.wrapping_add(0x3c)) as f64);
        esp = esp.wrapping_sub(0x8);
        { let v = fs.pop().unwrap(); m.set_f64(esp, v); }
        fs.push(m.f32(edi.wrapping_add(0x38)) as f64);
        esp = esp.wrapping_sub(0x8);
        { let v = fs.pop().unwrap(); m.set_f64(esp, v); }
        fs.push(m.f32(edi.wrapping_add(0x34)) as f64);
        esp = esp.wrapping_sub(0x8);
        { let v = fs.pop().unwrap(); m.set_f64(esp, v); }
        esp = esp.wrapping_sub(4); m.set_u32(esp, 0x67d390);
        esp = esp.wrapping_sub(4); m.set_u32(esp, eax);
        call!(0x61cca9);
        fs.push(m.f32(esp.wrapping_add(0x130)) as f64);
        esp = esp.wrapping_add(0x48);
        { let v = fs.pop().unwrap(); m.set_f64(esp.wrapping_add(0x3c), v); }
        fs.push(m.f32(esp.wrapping_add(0x200)) as f64);
        { let v = fs.pop().unwrap(); m.set_f64(esp, v); }
        fs.push(m.f32(esp.wrapping_add(0x1fc)) as f64);
        esp = esp.wrapping_sub(0x8);
        { let v = fs.pop().unwrap(); m.set_f64(esp, v); }
        fs.push(m.f32(esp.wrapping_add(0x200)) as f64);
        esp = esp.wrapping_sub(0x8);
        { let v = fs.pop().unwrap(); m.set_f64(esp, v); }
        fs.push(m.f32(esp.wrapping_add(0x204)) as f64);
        esp = esp.wrapping_sub(0x8);
        { let v = fs.pop().unwrap(); m.set_f64(esp, v); }
        fs.push(m.f32(esp.wrapping_add(0x208)) as f64);
        esp = esp.wrapping_sub(0x8);
        { let v = fs.pop().unwrap(); m.set_f64(esp, v); }
        fs.push(m.f32(esp.wrapping_add(0x20c)) as f64);
        esp = esp.wrapping_sub(0x8);
        { let v = fs.pop().unwrap(); m.set_f64(esp, v); }
        fs.push(m.f32(esp.wrapping_add(0x210)) as f64);
        esp = esp.wrapping_sub(0x8);
        { let v = fs.pop().unwrap(); m.set_f64(esp, v); }
        fs.push(m.f32(esp.wrapping_add(0x214)) as f64);
        esp = esp.wrapping_sub(0x8);
        { let v = fs.pop().unwrap(); m.set_f64(esp, v); }
        fs.push(m.f32(esp.wrapping_add(0x218)) as f64);
        esp = esp.wrapping_sub(0x8);
        { let v = fs.pop().unwrap(); m.set_f64(esp, v); }
        fs.push(m.f32(esp.wrapping_add(0x21c)) as f64);
        esp = esp.wrapping_sub(0x8);
        { let v = fs.pop().unwrap(); m.set_f64(esp, v); }
        fs.push(m.f32(esp.wrapping_add(0x220)) as f64);
        esp = esp.wrapping_sub(0x8);
        { let v = fs.pop().unwrap(); m.set_f64(esp, v); }
        fs.push(m.f32(esp.wrapping_add(0x224)) as f64);
        esp = esp.wrapping_sub(0x8);
        { let v = fs.pop().unwrap(); m.set_f64(esp, v); }
        fs.push(m.f32(ebx.wrapping_add(0xcf8)) as f64);
        esp = esp.wrapping_sub(0x8);
        { let v = fs.pop().unwrap(); m.set_f64(esp, v); }
        fs.push(m.f32(ebx.wrapping_add(0xcf4)) as f64);
        esp = esp.wrapping_sub(0x8);
        { let v = fs.pop().unwrap(); m.set_f64(esp, v); }
        fs.push(m.f32(ebx.wrapping_add(0xcf0)) as f64);
        esp = esp.wrapping_sub(0x8);
        { let v = fs.pop().unwrap(); m.set_f64(esp, v); }
        fs.push(m.f32(ebx.wrapping_add(0xcec)) as f64);
        esp = esp.wrapping_sub(0x8);
        eax = m.u32(esp.wrapping_add(0xb8));
        ecx = m.u32(esp.wrapping_add(0xb4));
        { let v = fs.pop().unwrap(); m.set_f64(esp, v); }
        fs.push(m.f32(ebx.wrapping_add(0xce8)) as f64);
        esp = esp.wrapping_sub(0x8);
        { let v = fs.pop().unwrap(); m.set_f64(esp, v); }
        fs.push(m.f32(ebx.wrapping_add(0xce4)) as f64);
        esp = esp.wrapping_sub(0x8);
        { let v = fs.pop().unwrap(); m.set_f64(esp, v); }
        fs.push(m.f32(esp.wrapping_add(0x178)) as f64);
        esp = esp.wrapping_sub(0x8);
        { let v = fs.pop().unwrap(); m.set_f64(esp, v); }
        fs.push(m.f32(esp.wrapping_add(0x13c)) as f64);
        esp = esp.wrapping_sub(4); m.set_u32(esp, eax);
        esp = esp.wrapping_sub(4); m.set_u32(esp, ecx);
        esp = esp.wrapping_sub(4); m.set_u32(esp, eax);
        esp = esp.wrapping_sub(4); m.set_u32(esp, ecx);
        esp = esp.wrapping_sub(0x8);
        ecx = m.u32(ebx.wrapping_add(0x140));
        eax = esp.wrapping_add(0xbe4);
        { let v = fs.pop().unwrap(); m.set_f64(esp, v); }
        fs.push(m.f32(esp.wrapping_add(0x150)) as f64);
        esp = esp.wrapping_sub(0x8);
        edx = m.u32(ecx.wrapping_add(0xc));
        { let v = fs.pop().unwrap(); m.set_f64(esp, v); }
        fs.push(m.f32(esp.wrapping_add(0x154)) as f64);
        esp = esp.wrapping_sub(0x8);
        { let v = fs.pop().unwrap(); m.set_f64(esp, v); }
        fs.push(m.f32(esp.wrapping_add(0x120)) as f64);
        esp = esp.wrapping_sub(0x8);
        { let v = fs.pop().unwrap(); m.set_f64(esp, v); }
        fs.push(m.f32(esp.wrapping_add(0x124)) as f64);
        esp = esp.wrapping_sub(0x8);
        { let v = fs.pop().unwrap(); m.set_f64(esp, v); }
        fs.push(m.f32(esp.wrapping_add(0x128)) as f64);
        esp = esp.wrapping_sub(0x8);
        { let v = fs.pop().unwrap(); m.set_f64(esp, v); }
        esp = esp.wrapping_sub(4); m.set_u32(esp, edx);
        esp = esp.wrapping_sub(4); m.set_u32(esp, 0x67d318);
        esp = esp.wrapping_sub(4); m.set_u32(esp, eax);
        call!(0x61cca9);
        fs.push(m.f32(esp.wrapping_add(0x1e4)) as f64);
        esp = esp.wrapping_add(0xdc);
        { let v = fs.pop().unwrap(); m.set_f64(esp.wrapping_add(0x3c), v); }
        fs.push(m.f32(esp.wrapping_add(0x1d0)) as f64);
        { let v = fs.pop().unwrap(); m.set_f64(esp, v); }
        fs.push(m.f32(esp.wrapping_add(0x1cc)) as f64);
        esp = esp.wrapping_sub(0x8);
        { let v = fs.pop().unwrap(); m.set_f64(esp, v); }
        fs.push(m.f32(esp.wrapping_add(0x1d0)) as f64);
        esp = esp.wrapping_sub(0x8);
        { let v = fs.pop().unwrap(); m.set_f64(esp, v); }
        fs.push(m.f32(esp.wrapping_add(0x1d4)) as f64);
        esp = esp.wrapping_sub(0x8);
        { let v = fs.pop().unwrap(); m.set_f64(esp, v); }
        fs.push(m.f32(esp.wrapping_add(0x1d8)) as f64);
        esp = esp.wrapping_sub(0x8);
        { let v = fs.pop().unwrap(); m.set_f64(esp, v); }
        fs.push(m.f32(esp.wrapping_add(0x1dc)) as f64);
        esp = esp.wrapping_sub(0x8);
        { let v = fs.pop().unwrap(); m.set_f64(esp, v); }
        fs.push(m.f32(esp.wrapping_add(0x1e0)) as f64);
        esp = esp.wrapping_sub(0x8);
        { let v = fs.pop().unwrap(); m.set_f64(esp, v); }
        fs.push(m.f32(esp.wrapping_add(0x1e4)) as f64);
        esp = esp.wrapping_sub(0x8);
        { let v = fs.pop().unwrap(); m.set_f64(esp, v); }
        fs.push(m.f32(esp.wrapping_add(0x1e8)) as f64);
        esp = esp.wrapping_sub(0x8);
        { let v = fs.pop().unwrap(); m.set_f64(esp, v); }
        fs.push(m.f32(esp.wrapping_add(0x1ec)) as f64);
        esp = esp.wrapping_sub(0x8);
        { let v = fs.pop().unwrap(); m.set_f64(esp, v); }
        fs.push(m.f32(esp.wrapping_add(0x1f0)) as f64);
        esp = esp.wrapping_sub(0x8);
        { let v = fs.pop().unwrap(); m.set_f64(esp, v); }
        fs.push(m.f32(esp.wrapping_add(0x1f4)) as f64);
        esp = esp.wrapping_sub(0x8);
        { let v = fs.pop().unwrap(); m.set_f64(esp, v); }
        fs.push(m.f32(esi.wrapping_add(0xcf8)) as f64);
        esp = esp.wrapping_sub(0x8);
        eax = m.u32(esp.wrapping_add(0xa0));
        ecx = m.u32(esp.wrapping_add(0x9c));
        { let v = fs.pop().unwrap(); m.set_f64(esp, v); }
        fs.push(m.f32(esi.wrapping_add(0xcf4)) as f64);
        esp = esp.wrapping_sub(0x8);
        { let v = fs.pop().unwrap(); m.set_f64(esp, v); }
        fs.push(m.f32(esi.wrapping_add(0xcf0)) as f64);
        esp = esp.wrapping_sub(0x8);
        { let v = fs.pop().unwrap(); m.set_f64(esp, v); }
        fs.push(m.f32(esi.wrapping_add(0xcec)) as f64);
        esp = esp.wrapping_sub(0x8);
        { let v = fs.pop().unwrap(); m.set_f64(esp, v); }
        fs.push(m.f32(esi.wrapping_add(0xce8)) as f64);
        esp = esp.wrapping_sub(0x8);
        { let v = fs.pop().unwrap(); m.set_f64(esp, v); }
        fs.push(m.f32(esi.wrapping_add(0xce4)) as f64);
        esp = esp.wrapping_sub(0x8);
        { let v = fs.pop().unwrap(); m.set_f64(esp, v); }
        fs.push(m.f32(esp.wrapping_add(0x198)) as f64);
        esp = esp.wrapping_sub(0x8);
        { let v = fs.pop().unwrap(); m.set_f64(esp, v); }
        fs.push(m.f32(esp.wrapping_add(0x148)) as f64);
        esp = esp.wrapping_sub(4); m.set_u32(esp, eax);
        esp = esp.wrapping_sub(4); m.set_u32(esp, ecx);
        esp = esp.wrapping_sub(4); m.set_u32(esp, eax);
        esp = esp.wrapping_sub(4); m.set_u32(esp, ecx);
        esp = esp.wrapping_sub(0x8);
        ecx = m.u32(esi.wrapping_add(0x140));
        eax = esp.wrapping_add(0x414);
        { let v = fs.pop().unwrap(); m.set_f64(esp, v); }
        fs.push(m.f32(esp.wrapping_add(0x15c)) as f64);
        esp = esp.wrapping_sub(0x8);
        edx = m.u32(ecx.wrapping_add(0xc));
        { let v = fs.pop().unwrap(); m.set_f64(esp, v); }
        fs.push(m.f32(esp.wrapping_add(0x160)) as f64);
        esp = esp.wrapping_sub(0x8);
        { let v = fs.pop().unwrap(); m.set_f64(esp, v); }
        fs.push(m.f32(esp.wrapping_add(0x12c)) as f64);
        esp = esp.wrapping_sub(0x8);
        { let v = fs.pop().unwrap(); m.set_f64(esp, v); }
        fs.push(m.f32(esp.wrapping_add(0x130)) as f64);
        esp = esp.wrapping_sub(0x8);
        { let v = fs.pop().unwrap(); m.set_f64(esp, v); }
        fs.push(m.f32(esp.wrapping_add(0x134)) as f64);
        esp = esp.wrapping_sub(0x8);
        { let v = fs.pop().unwrap(); m.set_f64(esp, v); }
        esp = esp.wrapping_sub(4); m.set_u32(esp, edx);
        esp = esp.wrapping_sub(4); m.set_u32(esp, 0x67d2a0);
        esp = esp.wrapping_sub(4); m.set_u32(esp, eax);
        call!(0x61cca9);
        fs.push(m.f32(esp.wrapping_add(0x210)) as f64);
        esp = esp.wrapping_add(0xe4);
        ecx = esp.wrapping_add(0x364);
        edx = esp.wrapping_add(0xb34);
        eax = esp.wrapping_add(0x74c);
        esp = esp.wrapping_sub(4); m.set_u32(esp, ecx);
        esp = esp.wrapping_sub(4); m.set_u32(esp, edx);
        esp = esp.wrapping_sub(4); m.set_u32(esp, eax);
        esp = esp.wrapping_sub(0x8);
        { let v = fs.pop().unwrap(); m.set_f64(esp, v); }
        fs.push(m.f32(esp.wrapping_add(0x13c)) as f64);
        esp = esp.wrapping_sub(0x8);
        { let v = fs.pop().unwrap(); m.set_f64(esp, v); }
        fs.push(m.f32(esp.wrapping_add(0x140)) as f64);
        esp = esp.wrapping_sub(0x8);
        { let v = fs.pop().unwrap(); m.set_f64(esp, v); }
        fs.push(m.f32(esp.wrapping_add(0x144)) as f64);
        esp = esp.wrapping_sub(0x8);
        { let v = fs.pop().unwrap(); m.set_f64(esp, v); }
        fs.push(m.f32(esp.wrapping_add(0x148)) as f64);
        esp = esp.wrapping_sub(0x8);
        { let v = fs.pop().unwrap(); m.set_f64(esp, v); }
        fs.push(m.f32(esp.wrapping_add(0x14c)) as f64);
        esp = esp.wrapping_sub(0x8);
        { let v = fs.pop().unwrap(); m.set_f64(esp, v); }
        fs.push(m.f32(esp.wrapping_add(0x174)) as f64);
        esp = esp.wrapping_sub(0x8);
        { let v = fs.pop().unwrap(); m.set_f64(esp, v); }
        fs.push(m.f32(esp.wrapping_add(0x178)) as f64);
        esp = esp.wrapping_sub(0x8);
        { let v = fs.pop().unwrap(); m.set_f64(esp, v); }
        fs.push(m.f32(esp.wrapping_add(0x17c)) as f64);
        esp = esp.wrapping_sub(0x8);
        ecx = esp.wrapping_add(0xf70);
        { let v = fs.pop().unwrap(); m.set_f64(esp, v); }
        fs.push(m.f32(esp.wrapping_add(0x1b0)) as f64);
        esp = esp.wrapping_sub(0x8);
        { let v = fs.pop().unwrap(); m.set_f64(esp, v); }
        fs.push(m.f32(esp.wrapping_add(0x1b4)) as f64);
        esp = esp.wrapping_sub(0x8);
        { let v = fs.pop().unwrap(); m.set_f64(esp, v); }
        fs.push(m.f32(esp.wrapping_add(0x1b8)) as f64);
        esp = esp.wrapping_sub(0x8);
        { let v = fs.pop().unwrap(); m.set_f64(esp, v); }
        esp = esp.wrapping_sub(4); m.set_u32(esp, 0x67d250);
        esp = esp.wrapping_sub(4); m.set_u32(esp, ecx);
        call!(0x61cca9);
        edx = esp.wrapping_add(0xf90);
        esp = esp.wrapping_sub(4); m.set_u32(esp, edx);
        call!(0x4e260c);
        esp = esp.wrapping_add(0x78);
        pc = 0x580851; continue;
    }
    0x580851 => {
        eax = m.u32(ebx);
        ecx = esp.wrapping_add(0x154);
        esp = esp.wrapping_sub(4); m.set_u32(esp, 0x67d23c);
        esp = esp.wrapping_sub(4); m.set_u32(esp, ecx);
        ecx = ebx;
        callv!(eax, 0x90);
        edx = m.u32(ebx);
        eax = esp.wrapping_add(0x130);
        esp = esp.wrapping_sub(4); m.set_u32(esp, 0x67d228);
        esp = esp.wrapping_sub(4); m.set_u32(esp, eax);
        ecx = ebx;
        callv!(edx, 0x94);
        edx = m.u32(esi);
        eax = esp.wrapping_add(0x118);
        esp = esp.wrapping_sub(4); m.set_u32(esp, 0x67d214);
        esp = esp.wrapping_sub(4); m.set_u32(esp, eax);
        ecx = esi;
        callv!(edx, 0x90);
        edx = m.u32(esi);
        eax = esp.wrapping_add(0x124);
        esp = esp.wrapping_sub(4); m.set_u32(esp, 0x67d200);
        esp = esp.wrapping_sub(4); m.set_u32(esp, eax);
        ecx = esi;
        callv!(edx, 0x94);
        edi = m.u32(esp); esp = esp.wrapping_add(4);
        esi = m.u32(esp); esp = esp.wrapping_add(4);
        ebx = m.u32(esp); esp = esp.wrapping_add(4);
        esp = ebp;
        ebp = m.u32(esp); esp = esp.wrapping_add(4);
        break;
    }
            t => panic!("pc {t:x}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unhex(s: &str) -> Vec<u8> {
        (0..s.len() / 2).map(|i| u8::from_str_radix(&s[2 * i..2 * i + 2], 16).unwrap()).collect()
    }
    fn hx(m: &Mem, a: u32, n: usize) -> String {
        m.read(a, n).iter().map(|b| format!("{b:02x}")).collect()
    }

    const A: u32 = 0x2a010000;
    const B: u32 = 0x2a012000;
    const REC: u32 = 0x2a015000;

    struct Ow {
        ev: Vec<String>,
    }
    impl ItemVirtuals for Ow {
        fn virt(&mut self, m: &mut Mem, this: u32, slot: u32, args: &[u32]) {
            let nm = if slot == 0x90 { "v90" } else { "v94" };
            let who = if this == A { "a" } else { "b" };
            self.ev.push(format!("{nm}|{who}|{}|{:x}", hx(m, args[0], 12), args[1]));
        }
    }
    impl ResolveOwner for Ow {
        fn report(&mut self, m: &Mem, vec: u32, text: u32, limit: f32) {
            self.ev.push(format!("rep|{}|{:x}|{:x}", hx(m, vec, 12), text, limit.to_bits()));
        }
        fn log(&mut self) {
            self.ev.push("log".to_string());
        }
    }

    #[test]
    fn native_contact_resolve() {
        let mut n = 0;
        for (c, line) in include_str!("../../../tools/emu/golden_contact_eae0.tsv").lines().enumerate() {
            let v: Vec<&str> = line.split('\t').collect();
            let mut m = Mem::new();
            for it in v[0].split(',').filter(|s| !s.is_empty()) {
                let (o, x) = it.split_once(':').unwrap();
                m.write(u32::from_str_radix(o, 16).unwrap(), &unhex(x));
            }
            let (a0, b0, r0) = (m.read(A, 0x1000), m.read(B, 0x1000), m.read(REC, 0x100));
            let mut ow = Ow { ev: vec![] };
            contact_resolve(&mut m, A, REC, B, &mut ow);
            let d = |a: &[u8], b: &[u8]| -> String {
                (0..a.len())
                    .step_by(4)
                    .filter(|&i| a[i..i + 4] != b[i..i + 4])
                    .map(|i| format!("{:x}:{}", i, b[i..i + 4].iter().map(|x| format!("{x:02x}")).collect::<String>()))
                    .collect::<Vec<_>>()
                    .join(",")
            };
            assert_eq!(d(&a0, &m.read(A, 0x1000)), v[1], "case {c}: body A");
            assert_eq!(d(&b0, &m.read(B, 0x1000)), v[2], "case {c}: body B");
            assert_eq!(d(&r0, &m.read(REC, 0x100)), v[3], "case {c}: record");
            assert_eq!(ow.ev.join(";"), v[4], "case {c}: events");
            n += 1;
        }
        assert_eq!(n, 800);
    }
}

//! `_Car_V::run1` (`0x4fdc80`): one simulation step of a vehicle.

use crate::contact::{self, World};
use crate::matrix as mx;
use crate::mem::Mem;

const SCRATCH: u32 = 0x2ff8_0000;

fn f(m: &Mem, a: u32) -> f64 {
    m.f32(a) as f64
}

fn r(x: f64) -> f32 {
    x as f32
}

fn v3(m: &Mem, a: u32) -> [f64; 3] {
    [f(m, a), f(m, a + 4), f(m, a + 8)]
}

fn set3(m: &mut Mem, a: u32, v: [f32; 3]) {
    m.set_f32(a, v[0]);
    m.set_f32(a + 4, v[1]);
    m.set_f32(a + 8, v[2]);
}

fn copy_words(m: &mut Mem, dst: u32, src: u32, n: u32) {
    for i in 0..n {
        let w = m.u32(src + 4 * i);
        m.set_u32(dst + 4 * i, w);
    }
}

/// Debugging aid: with `HT2_STEP_STAGE=n` the step stops after stage `n` and dumps the vehicle
/// memory to `HT2_STEP_DUMP` (compared against the original stopped at the same point).
fn stage(m: &Mem, this: u32, n: u32) -> bool {
    match std::env::var("HT2_STEP_STAGE").ok().and_then(|v| v.parse::<u32>().ok()) {
        Some(s) if s == n => {
            if let Ok(path) = std::env::var("HT2_STEP_DUMP") {
                let _ = std::fs::write(path, m.read(this, 0x6000));
            }
            true
        }
        _ => false,
    }
}

fn zero3(m: &mut Mem, a: u32) {
    copy_words(m, a, 0x702650, 3);
}

/// `0x4e5b22`: query the segment `p2`..`p3` against the world object `obj`.
pub fn fn_004e5b22(m: &mut Mem, world: &mut dyn World, obj: u32, p2: u32, p3: u32, rec: u32) -> u32 {
    if obj == 0 {
        return 0;
    }
    copy_words(m, rec + 4, crate::globals::MAT_IDENTITY, 12);
    copy_words(m, rec + 0x4c, p2, 3);
    copy_words(m, rec + 0x58, p3, 3);
    m.set_u32(rec + 0x74, 0);
    m.set_u32(rec, 0);
    m.set_u32(rec + 0x68, 0);
    world.collide(m, obj, 0, rec)
}

/// `0x4fdc80` (`_Car_V::run1`): advance the vehicle `this` by one step: driver input, engine
/// and gearbox, steering, tyres, forces, integration of the velocities, the new body transform
/// from the wheel contacts, and the bookkeeping for the next step.
pub fn fn_004fdc80(m: &mut Mem, world: &mut dyn World, this: u32) {
    let dt = crate::globals::dt(m);
    let dt64 = dt as f64;
    let t = r(crate::globals::time_hi(m) - f(m, this + 0x17e0));
    m.set_f32(this + 0x3bd4, t);
    crate::chassis::fn_00501930(m, this);
    if stage(m, this, 10) {
        return;
    }
    crate::input::fn_00500850(m, this);
    if stage(m, this, 11) {
        return;
    }
    m.set_u32(this + 0x3c84, 0);
    m.set_u32(this + 0x3c58, 0);
    zero3(m, this + 0x3c5c);
    m.set_u32(this + 0x3c88, 0);
    let mass = f(m, this + 0x2640);
    let g = v3(m, this + 0xd40);
    set3(m, this + 0xc54, [r(mass * g[0]), r(mass * g[1]), r(mass * g[2])]);
    let count = m.i32(this + 0xdd4).max(0) as u32;
    for i in 0..count {
        for o in [0xe3c, 0x3b90, 0xe7c, 0xe5c] {
            m.set_u32(this + o + 4 * i, 0);
        }
    }
    if m.i16(this + 0x2748) != 0 {
        m.set_u32(this + 0x3344, 0);
        m.set_i16(this + 0x2748, 0);
        for o in [0x271c, 0x29e0, 0x2720, 0x29dc] {
            m.set_u32(this + o, 0);
        }
        m.set_f32(this + 0x29d8, 0.05);
        m.set_u32(this + 0x29e8, 0);
        crate::chassis::fn_004eba10(m, this + 0x2700);
    }
    crate::carv::fn_004f0450(m, this);
    crate::motion::fn_004f0ce0(m, this);
    let inv: [f64; 12] = core::array::from_fn(|k| f(m, this + 0x900 + 4 * k as u32));
    for i in 0..count {
        m.set_u32(this + 0xd00 + 4 * i, 0);
        zero3(m, this + 0x20f4 + 12 * i);
        let p = v3(m, this + 0x64 + 12 * i);
        set3(
            m,
            this + 0x1844 + 12 * i,
            [
                r(inv[6] * p[2] + p[0] * inv[0] + p[1] * inv[3] + inv[9]),
                r(p[0] * inv[1] + inv[7] * p[2] + p[1] * inv[4] + inv[10]),
                r(p[1] * inv[5] + p[0] * inv[2] + p[2] * inv[8] + inv[11]),
            ],
        );
        m.wr8(this + 0x19c4 + i, 0);
    }
    let n = v3(m, this + 0xd0);
    set3(
        m,
        this + 0x1008,
        [
            r(inv[3] * n[1] + inv[6] * n[2] + n[0] * inv[0]),
            r(inv[4] * n[1] + inv[1] * n[0] + inv[7] * n[2]),
            r(inv[5] * n[1] + inv[2] * n[0] + inv[8] * n[2]),
        ],
    );
    // steering
    let steer = crate::carv::fn_004f0090(m, this);
    m.set_f32(this + 0x1a20, steer);
    let rot = SCRATCH;
    mx::mat_rot_z(m, rot, steer);
    crate::chassis::fn_004f2630(m, this, rot);
    crate::carv::fn_004f2490(m, this);
    let s = (steer as f64).sin();
    let s32 = s as f32;
    let mut q = s.abs() / f(m, this + 0x1f60);
    if m.rd8(this + 0x19b5) != 0 {
        q = 0.0;
    }
    let v = f(m, this + 0xdc8).abs();
    let a1 = r(q * v);
    let a2 = r(q * v * v);
    let c1 = s32 > 0.0;
    let d9 = v3(m, this + 0x9fc);
    let pa = v3(m, this + 0xa2c);
    let c2 = d9[2] * pa[2] + d9[1] * pa[1] + pa[0] * d9[0] > 0.0;
    let sign: f64 = if c1 != c2 { -1.0 } else { 1.0 };
    let w = sign * f(m, this + 0xdc4) * q;
    set3(m, this + 0xc24, [r(w * n[0]), r(w * n[1]), r(w * n[2])]);
    let cx = n[1] * d9[2] - d9[1] * n[2];
    let cy = r(d9[0] * n[2] - n[0] * d9[2]) as f64;
    let cz = r(d9[1] * n[0] - d9[0] * n[1]) as f64;
    let k = r(sign * mass * a2 as f64) as f64;
    let c = [r(cx * k) as f64, r(cy * k) as f64, r(cz * k) as f64];
    let rt: [f64; 9] = core::array::from_fn(|i| f(m, rot + 4 * i as u32));
    set3(
        m,
        this + 0xc78,
        [
            r(rt[0] * c[0] + rt[6] * c[2] + rt[3] * c[1]),
            r(rt[1] * c[0] + rt[7] * c[2] + rt[4] * c[1]),
            r(rt[2] * c[0] + rt[8] * c[2] + rt[5] * c[1]),
        ],
    );
    crate::chassis::fn_004f35f0(m, this);
    if stage(m, this, 1) {
        return;
    }
    // external pushes
    let (fs, ts) = (SCRATCH + 0x40, SCRATCH + 0x50);
    zero3(m, fs);
    zero3(m, ts);
    for k in 0..m.i32(this + 0x27c0).max(0) as u32 {
        let fp = this + 0x2804 + 12 * k;
        mx::vec_add_in_place(m, fs, SCRATCH + 0x60, fp);
        let p = v3(m, fp);
        let rx = p[0] * inv[0] + p[2] * inv[6] + inv[3] * p[1];
        let ry = inv[7] * p[2] + inv[4] * p[1] + p[0] * inv[1];
        let rz = inv[2] * p[0] + inv[8] * p[2] + inv[5] * p[1];
        let l = v3(m, this + 0x27c8 + 12 * k);
        let tx = r(rz * l[1] - ry * l[2]) as f64;
        let ty = r(rx * l[2] - rz * l[0]) as f64;
        let tz = ry * l[0] - rx * l[1];
        let t0 = v3(m, ts);
        set3(m, ts, [r(t0[0] + tx), r(ty + t0[1]), r(tz + t0[2])]);
    }
    // wheel forces
    zero3(m, this + 0xc6c);
    zero3(m, this + 0xcc0);
    let mut any_air = false;
    for i in 0..count {
        let air = m.i16(this + 0x19a4 + 2 * i) != 0;
        any_air = any_air || air;
        let (mut fl, mut fr) = ([0.0f64; 3], [0.0f64; 3]);
        if !air {
            fl = v3(m, this + 0x21b4 + 12 * i);
            fr = v3(m, this + 0xf08 + 12 * i);
        }
        let sum = SCRATCH + 0x70;
        set3(m, sum, [r(fr[0] + fl[0]), r(fr[1] + fl[1]), r(fr[2] + fl[2])]);
        mx::vec_add_in_place(m, this + 0xc6c, SCRATCH + 0x80, sum);
        if !air {
            let vv = v3(m, this + 0x2094 + 12 * i);
            let wl = v3(m, this + 0x104 + 0xf8 * i);
            let a = vv[2] * wl[1] - vv[1] * wl[2];
            let b = vv[0] * wl[2] - vv[2] * wl[0];
            let cc = vv[1] * wl[0] - wl[1] * vv[0];
            let out = this + 0x2034 + 12 * i;
            set3(m, out, [r(b * wl[2] - cc * wl[1]), r(cc * wl[0] - a * wl[2]), r(a * wl[1] - b * wl[0])]);
            mx::vec_add_in_place(m, this + 0xc6c, SCRATCH + 0x80, out);
        }
    }
    let (c78, c6c) = (v3(m, this + 0xc78), v3(m, this + 0xc6c));
    let sx = c78[0] + c6c[0];
    let sy = r(c6c[1] + c78[1]) as f64;
    let sz = r(c6c[2] + c78[2]) as f64;
    let vel0 = v3(m, this + 0xd40);
    set3(m, this + 0xd58, [r(vel0[0] - sx / mass), r(vel0[1] - r(sy / mass) as f64), r(vel0[2] - r(sz / mass) as f64)]);
    let vel = this + 0x40;
    let mut base = vel0;
    if !any_air {
        let d = vel0[2] * n[2] + vel0[1] * n[1] + vel0[0] * n[0];
        let pnx = r(d * n[0]) as f64;
        let pny = r(d * n[1]) as f64;
        let pnz = d * n[2];
        let a = vel0[2] * n[1] - n[2] * vel0[1];
        let b = n[2] * vel0[0] - n[0] * vel0[2];
        let cc = n[0] * vel0[1] - vel0[0] * n[1];
        let tx = r(b * n[2] - cc * n[1]) as f64;
        let ty = r(cc * n[0] - a * n[2]) as f64;
        let tz = a * n[1] - b * n[0];
        let hx = r(tx * 0.5) as f64;
        let hy = r(ty * 0.5) as f64;
        let hz = tz * 0.5;
        base = [r(hx + pnx) as f64, r(pny + hy) as f64, r(pnz + hz) as f64];
    }
    let mt = f(m, this + 0x278c) + mass;
    let fsv = v3(m, fs);
    let ext = [r(fsv[0] / mass) as f64, r(fsv[1] / mass) as f64, r(fsv[2] / mass) as f64];
    let c6c = v3(m, this + 0xc6c);
    let wf = [r(c6c[0] / mt) as f64, r(c6c[1] / mt) as f64, c6c[2] / mt];
    let cv = [r(base[0] + wf[0]) as f64, r(wf[1] + base[1]) as f64, wf[2] + base[2]];
    let dv = [r(cv[0] + ext[0]) as f64, r(ext[1] + cv[1]) as f64, cv[2] + ext[2]];
    let e = SCRATCH + 0x90;
    set3(m, e, [r(dv[0] * dt64), r(dv[1] * dt64), r(dv[2] * dt64)]);
    mx::vec_add_in_place(m, vel, SCRATCH + 0xa0, e);
    if stage(m, this, 2) {
        return;
    }
    if !(any_air && m.rd8(this + 0x19b5) != 0 && m.rd8(this + 0x19b7) != 0) {
        let vv = v3(m, vel);
        let a = vv[2] * n[1] - n[2] * vv[1];
        let b = n[2] * vv[0] - n[0] * vv[2];
        let cc = n[0] * vv[1] - vv[0] * n[1];
        set3(m, vel, [r(b * n[2] - cc * n[1]), r(cc * n[0] - a * n[2]), r(a * n[1] - b * n[0])]);
        let len = crate::math::vec_len(m, vel);
        m.set_f32(this + 0xdc0, len as f32);
        let mut comp = 0.0f64;
        if m.rd8(this + 0x19b6) != 0 || m.rd8(this + 0x19b8) != 0 {
            for i in 0..count {
                let ei = f(m, this + 0xe18 + 4 * i);
                if comp < ei {
                    comp = ei;
                }
            }
        } else {
            comp = f(m, this + 0x1f7c) * 0.5;
        }
        let snap = !(n[2] < 0.8) && comp <= f(m, this + 0x1f7c) * 4.0;
        if snap {
            let rr = v3(m, this + 0xa38);
            let vv = v3(m, vel);
            let dir: f64 = if vv[2] * rr[2] + vv[1] * rr[1] + rr[0] * vv[0] > 0.0 { 1.0 } else { -1.0 };
            let t = [r(len * rr[0]) as f64, r(len * rr[1]) as f64, r(len * rr[2]) as f64];
            set3(m, vel, [r(t[0] * dir), r(t[1] * dir), r(t[2] * dir)]);
        } else {
            let len = crate::math::vec_len(m, vel);
            m.set_f32(this + 0xdc0, len as f32);
        }
        if m.u32(this + 0x1abc) != 0 {
            let vv = v3(m, vel);
            let a8 = v3(m, this + 0xa08);
            if m.f32(this + 0xdc4) < 0.1 || vv[2] * a8[2] + vv[1] * a8[1] + vv[0] * a8[0] < 0.0 {
                crate::chassis::fn_004efc10(m, this);
                zero3(m, this + 0xc6c);
                zero3(m, this + 0xa14);
            }
        }
    }
    // rotation of the body for this step
    let mtx = this + 0x10;
    let rotm = SCRATCH + 0x100;
    if m.rd8(this + 0x19b5) != 0 {
        let (ry, rx, rz, m1) = (SCRATCH + 0x140, SCRATCH + 0x180, SCRATCH + 0x1c0, SCRATCH + 0x200);
        mx::mat_rot_y(m, ry, r(dt64 * f(m, this + 0x5c)));
        mx::mat_rot_x(m, rx, r(dt64 * f(m, this + 0x58)));
        mx::mat_rot_z(m, rz, r(dt64 * f(m, this + 0x60)));
        mx::mat_mul(m, m1, rz, rx);
        mx::mat_mul(m, rotm, m1, ry);
    } else {
        let c = v3(m, this + 0xc24);
        set3(
            m,
            this + 0x58,
            [
                r(inv[6] * c[2] + inv[3] * c[1] + c[0] * inv[0]),
                r(inv[1] * c[0] + inv[7] * c[2] + inv[4] * c[1]),
                r(inv[2] * c[0] + inv[8] * c[2] + inv[5] * c[1]),
            ],
        );
        let om = v3(m, this + 0x58);
        let it = v3(m, this + 0x1f48);
        let tq = SCRATCH + 0x240;
        set3(m, tq, [r(om[0] * it[0]), r(om[1] * it[1]), r(om[2] * it[2])]);
        mx::rotate(m, this + 0x4c, tq, mtx);
        mx::mat_rot_z(m, rotm, r(sign * a1 as f64 * dt64));
    }
    if m.u32(0x697670) != 0 {
        let (rx, out) = (SCRATCH + 0x280, SCRATCH + 0x2c0);
        mx::mat_rot_x(m, rx, 0.17453292);
        mx::mat_mul(m, out, rotm, rx);
        copy_words(m, rotm, out, 12);
    }
    let vv = v3(m, vel);
    let (tr, m1, m2) = (SCRATCH + 0x300, SCRATCH + 0x340, SCRATCH + 0x380);
    mx::mat_translation(m, tr, r(dt64 * vv[0]), r(dt64 * vv[1]), r(dt64 * vv[2]));
    mx::mat_mul(m, m1, rotm, mtx);
    mx::mat_mul(m, m2, m1, tr);
    copy_words(m, mtx, m2, 12);
    mx::rotate(m, this + 0xa2c, 0x702620, mtx);
    let (x1, x2) = (SCRATCH + 0x3c0, SCRATCH + 0x3d0);
    mx::vec_cross(m, x1, this + 0xd0, this + 0xa2c);
    mx::vec_cross(m, x2, x1, this + 0xd0);
    mx::vec_unit(m, this + 0xa38, x2);
    if stage(m, this, 3) {
        return;
    }
    contact::fn_00505530(m, world, this, mtx);
    if stage(m, this, 4) {
        return;
    }
    let newm = SCRATCH + 0x400;
    crate::chassis::fn_004fca10(m, this, newm);
    copy_words(m, mtx, newm, 12);
    if stage(m, this, 5) {
        return;
    }
    crate::math::mat_inverse(m, this + 0x900, mtx);
    let lw = SCRATCH + 0x440;
    mx::rotate(m, lw, this + 0x4c, this + 0x900);
    let l = [m.f32(lw), m.f32(lw + 4), m.f32(lw + 8)];
    crate::math::vec_div(m, this + 0x58, l, this + 0x1f48);
    for o in 0..8 {
        m.set_u32(this + 0x3be8 + 4 * o, 0);
    }
    m.set_u32(this + 0x3c08, 0);
    m.set_u32(this + 0x3c0c, 0);
    let angles = SCRATCH + 0x480;
    let cur: [f32; 12] = core::array::from_fn(|k| m.f32(mtx + 4 * k as u32));
    crate::chassis::fn_005de950(m, cur, angles, angles + 8, angles + 16, angles + 24, angles + 32, angles + 40);
    let rec = SCRATCH + 0x500;
    contact::rec_init(m, rec);
    let room = m.u32(this + 0x1a10);
    if fn_004e5b22(m, world, room, this + 0x8f4, this + 0x34, rec) != 0 {
        copy_words(m, this + 0xf4, rec + 0x68, 1);
    }
    crate::motion::fn_004f0ce0(m, this);
    crate::chassis::fn_004f6b70(m, this, angles);
    if crate::carv::fn_004f0000(m, this) != 0 {
        for i in 0..count {
            copy_words(m, this + 0xdd8 + 4 * i, this + 0x1f9c + 4 * i, 1);
        }
        m.set_u32(this + 0x3344, 1);
        return;
    }
    for k in 0..m.i32(this + 0x27c0).max(0) as u32 {
        zero3(m, this + 0x2804 + 12 * k);
    }
    m.set_u32(this + 0x27c0, 0);
}

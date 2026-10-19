//! The "rigid body" step of `_Car_V` (physics mode `+0x2638` 2 and 3, `0x4fef20`): the body
//! integrated with contacts, sliding along the walls of the room (`0x506ae0`). The game switches
//! a vehicle into it when it touches other objects or leaves the ground (`0x548ec0`).

use crate::mem::Mem;

const SCRATCH: u32 = 0x2ffe_0000;

fn f(m: &Mem, a: u32) -> f64 {
    m.f32(a) as f64
}

fn r(x: f64) -> f32 {
    x as f32
}

/// `0x4f0160`: the angle of the steered wheels as the rear axle sees it (`+0x1a24`, from the
/// steering angle `+0x1a20` and the ratio `+0x1f78 / +0x1f60`), and the sines of both angles and
/// of their difference (`+0x1a50`, `+0x1a54`, `+0x1a58`).
pub fn fn_004f0160(m: &mut Mem, this: u32) {
    let steer = f(m, this + 0x1a20);
    let a = (f(m, this + 0x1f78) / f(m, this + 0x1f60) * steer.tan()).atan2(1.0);
    m.set_f32(this + 0x1a24, r(a));
    m.set_f32(this + 0x1a50, r(steer.sin()));
    m.set_f32(this + 0x1a54, r(a.sin()));
    m.set_f32(this + 0x1a58, r((steer - a).sin()));
}

/// `0x4f0410`: the mean of `+0x1a30` and `+0x1a34`.
pub fn fn_004f0410(m: &Mem, this: u32) -> f32 {
    r((f(m, this + 0x1a34) + f(m, this + 0x1a30)) * 0.5)
}

/// `0x4f0430`: the mean of `+0x1a38` and `+0x1a3c`.
pub fn fn_004f0430(m: &Mem, this: u32) -> f32 {
    r((f(m, this + 0x1a3c) + f(m, this + 0x1a38)) * 0.5)
}

/// `0x4f01b0`: the speed of the body along the wheels' plane from the two axles (the steered one
/// through the cosines of both angles, the other directly, whichever is smaller; 1000 times when
/// the wheels are turned across), turned into the body's velocity `+0xdc` and its turn rate
/// `+0xe8` (body axes `+0x900`); the two estimates go to `+0xdcc` and `+0xdd0`.
pub fn fn_004f01b0(m: &mut Mem, this: u32) {
    let g = |m: &Mem, o: u32| f(m, this + o);
    let c1 = r(g(m, 0x1a24).cos()) as f64;
    let c2 = g(m, 0x1a20).cos();
    let d1 = (g(m, 0x18ac) * g(m, 0xaac) + g(m, 0x18a8) * g(m, 0xaa8)) + g(m, 0x18a4) * g(m, 0xaa4);
    let mut x = c2 * d1;
    let d2 = r((g(m, 0x18c4) * g(m, 0xac4) + g(m, 0x18c0) * g(m, 0xac0)) + g(m, 0x18bc) * g(m, 0xabc)) as f64;
    let y = if 0.001 < c1 {
        let inv = 1.0 / c1;
        x *= inv;
        r(inv) as f64 * d2
    } else {
        x *= 1000.0;
        d2 * 1000.0
    };
    let y = r(y) as f64;
    let local = if y.abs() <= x.abs() { y } else { r(x) as f64 };
    m.set_f32(this + 0xdcc, r(x));
    m.set_f32(this + 0xdd0, r(y));
    let tmp = SCRATCH;
    crate::matrix::mat_rot_z(m, tmp, r(g(m, 0x1a24)));
    let mm = |m: &Mem, i: u32| f(m, tmp + 4 * i);
    let (w0, w1, w2) = (g(m, 0xa38), g(m, 0xa3c), g(m, 0xa40));
    let v0 = ((mm(m, 6) * w2 + mm(m, 3) * w1) + w0 * mm(m, 0)) + mm(m, 9);
    let v1 = r(((mm(m, 1) * w0 + mm(m, 7) * w2) + mm(m, 4) * w1) + mm(m, 10)) as f64;
    let v2 = r(((mm(m, 2) * w0 + mm(m, 8) * w2) + mm(m, 5) * w1) + mm(m, 11)) as f64;
    m.set_f32(this + 0xdc, r(v0 * local));
    m.set_f32(this + 0xe0, r(v1 * local));
    m.set_f32(this + 0xe4, r(v2 * local));
    let k = local / g(m, 0x1f60) * g(m, 0x1a20).sin();
    let f1 = r(k * g(m, 0xd0)) as f64;
    let f2 = r(k * g(m, 0xd4)) as f64;
    let k3 = k * g(m, 0xd8);
    m.set_f32(this + 0xe8, r((k3 * g(m, 0x918) + f2 * g(m, 0x90c)) + f1 * g(m, 0x900)));
    m.set_f32(this + 0xec, r((k3 * g(m, 0x91c) + f2 * g(m, 0x910)) + f1 * g(m, 0x904)));
    m.set_f32(this + 0xf0, r((k3 * g(m, 0x920) + f2 * g(m, 0x914)) + f1 * g(m, 0x908)));
}

fn copy3(m: &mut Mem, dst: u32, src: u32) {
    for i in 0..3 {
        let w = m.u32(src + 4 * i);
        m.set_u32(dst + 4 * i, w);
    }
}

fn zero3(m: &mut Mem, dst: u32) {
    copy3(m, dst, crate::globals::VEC_ZERO);
}

fn v3(m: &Mem, a: u32) -> [f64; 3] {
    [f(m, a), f(m, a + 4), f(m, a + 8)]
}

fn set3(m: &mut Mem, a: u32, v: [f64; 3]) {
    m.set_f32(a, r(v[0]));
    m.set_f32(a + 4, r(v[1]));
    m.set_f32(a + 8, r(v[2]));
}

/// `0x4ef620`: clear the motion of the body: velocity `+0x40`, the velocity at the axle
/// `+0xdc`, the sideways part `+0x9f0`, `+0xa08`, `+0xc6c`, `+0xc90`, `+0xc9c` and the two speeds
/// `+0xdc0`, `+0xdc4`; the forward direction `+0x9fc` becomes the axle direction `+0xa38`, the
/// stop distance `+0xd7c` 10 and `+0xd58` a copy of `+0xd40`.
pub fn fn_004ef620(m: &mut Mem, this: u32) {
    for o in [0x40, 0xa08, 0xdc, 0x9f0] {
        zero3(m, this + o);
    }
    m.set_u32(this + 0xdc0, 0);
    m.set_u32(this + 0xdc4, 0);
    for o in [0xc90, 0xc6c, 0xc9c] {
        zero3(m, this + o);
    }
    copy3(m, this + 0x9fc, this + 0xa38);
    let a = v3(m, this + 0xa38);
    let k = f(m, this + 0x9f0) * a[0] + f(m, this + 0x9f4) * a[1] + f(m, this + 0x9f8) * a[2];
    m.set_u32(this + 0xdc8, 0);
    set3(m, this + 0xa20, [k * a[0], k * a[1], k * a[2]]);
    m.set_u32(this + 0xd7c, 0x4120_0000);
    copy3(m, this + 0xd58, this + 0xd40);
}

/// `0x4ef7c0`: start a step of the body: the force `+0x4c` (the body-frame `+0x58` times
/// `+0x1f48`, in world axes; none in mode 0), its copy `+0xc3c`, other accumulators cleared,
/// `+0x960` the identity; the forward axis `+0xa2c` and the direction `+0xa38` across the axle
/// line `+0xd0`; the speed `+0xdc0` of `+0x40`, its part across the axle `+0x9f0` (length
/// `+0xdc4`, direction `+0x9fc`) and how much of it goes along `+0xa38` (`+0xdc8`, `+0xa20`).
pub fn fn_004ef7c0(m: &mut Mem, this: u32) {
    let g = |m: &Mem, o: u32| f(m, this + o);
    if m.i32(this + 0x2638) == 0 {
        zero3(m, this + 0x58);
        zero3(m, this + 0x4c);
    } else {
        m.set_u32(this + 0x60, 0);
        let v = [r(g(m, 0x58) * g(m, 0x1f48)) as f64, r(g(m, 0x5c) * g(m, 0x1f4c)) as f64, r(g(m, 0x60) * g(m, 0x1f50)) as f64];
        let x = (g(m, 0x1c) * v[1] + g(m, 0x28) * v[2]) + v[0] * g(m, 0x10);
        let y = (g(m, 0x20) * v[1] + g(m, 0x2c) * v[2]) + g(m, 0x14) * v[0];
        let z = (g(m, 0x24) * v[1] + g(m, 0x30) * v[2]) + g(m, 0x18) * v[0];
        set3(m, this + 0x4c, [x, y, z]);
    }
    copy3(m, this + 0xc3c, this + 0x4c);
    for o in [0xc24, 0xe8, 0xd64, 0xcd8, 0xcc0] {
        zero3(m, this + o);
    }
    for i in 0..12 {
        let w = m.u32(crate::globals::MAT_IDENTITY + 4 * i);
        m.set_u32(this + 0x960 + 4 * i, w);
    }
    let y = v3(m, crate::globals::VEC_Y);
    let a = [
        r((y[2] * g(m, 0x28) + y[1] * g(m, 0x1c)) + y[0] * g(m, 0x10)) as f64,
        r((y[2] * g(m, 0x2c) + y[1] * g(m, 0x20)) + y[0] * g(m, 0x14)) as f64,
        r((y[2] * g(m, 0x30) + y[1] * g(m, 0x24)) + y[0] * g(m, 0x18)) as f64,
    ];
    set3(m, this + 0xa2c, a);
    let d = v3(m, this + 0xd0);
    // (a x d) x d, normalised
    let cx = a[2] * d[1] - d[2] * a[1];
    let cy = r(d[2] * a[0] - a[2] * d[0]) as f64;
    let cz = r(a[1] * d[0] - a[0] * d[1]) as f64;
    let n = [cy * d[2] - cz * d[1], cz * d[0] - cx * d[2], cx * d[1] - cy * d[0]];
    set3(m, this + 0xa38, n);
    crate::math::vec_normalize(m, this + 0xa38);
    let speed = crate::math::vec_len(m, this + 0x40);
    m.set_f32(this + 0xdc0, r(speed));
    let v = v3(m, this + 0x40);
    let w = [r(v[1] * d[2] - d[1] * v[2]) as f64, r(d[0] * v[2] - d[2] * v[0]) as f64, r(d[1] * v[0] - d[0] * v[1]) as f64];
    let across = [w[2] * d[1] - w[1] * d[2], w[0] * d[2] - w[2] * d[0], w[1] * d[0] - w[0] * d[1]];
    set3(m, this + 0x9f0, across);
    let len = crate::math::vec_len(m, this + 0x9f0);
    m.set_f32(this + 0xdc4, r(len));
    if len <= 0.01 {
        copy3(m, this + 0x9fc, this + 0xa38);
    } else {
        crate::matrix::vec_div_scalar(m, this + 0x9fc, this + 0x9f0, r(len));
    }
    let k = r(crate::matrix::vec_dot(m, this + 0x9f0, this + 0xa38));
    m.set_f32(this + 0xdc8, k);
    crate::matrix::vec_scale(m, this + 0xa20, this + 0xa38, k);
}

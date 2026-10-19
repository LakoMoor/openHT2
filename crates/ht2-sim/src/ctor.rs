//! The constructor of a vehicle, `_Car_V::_Car_V` (`0x4ebd00`), and the member constructors and
//! set-up functions it calls.
//!
//! The constructor writes what the original writes, in the same order, and leaves alone what it
//! leaves alone (the golden test fills the object with a pattern beforehand to check that).

use crate::contact::{rec_init, World};
use crate::mem::Mem;
use crate::tech_system as ts;

/// Size of the vehicle object.
pub const CAR_SIZE: u32 = 0x4030;

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

fn zero(m: &mut Mem, a: u32, words: u32) {
    for i in 0..words {
        m.set_u32(a + 4 * i, 0);
    }
}

fn identity(m: &mut Mem, a: u32) {
    copy_words(m, a, crate::globals::MAT_IDENTITY, 12);
}

const ZERO3: u32 = 0x702650;
const UP: u32 = 0x702630;

/// `0x4ee4d0`: a wheel contact record (0xf8 bytes): no contact, normal up.
pub fn wheel_init(m: &mut Mem, w: u32) {
    zero(m, w, 6);
    rec_init(m, w + 0x20);
    copy_words(m, w, ZERO3, 3);
    copy_words(m, w + 0xc, UP, 3);
    m.set_i16(w + 0x18, 0);
    m.set_i16(w + 0x1a, 0);
    m.set_u32(w + 0x84, 0);
    m.set_u32(w + 0xf0, 0);
}

/// `0x4f6cf0`: the sub-object at `+0x2224` (body state of the integrator).
pub fn fn_004f6cf0(m: &mut Mem, s: u32) {
    for o in [0x20, 0x50, 0x80, 0xb0, 0xe0] {
        identity(m, s + o);
    }
    zero(m, s + 0x110, (0x1f4 - 0x110) / 4);
    zero(m, s + 0x238, 24);
    zero(m, s + 0x374, 18);
    identity(m, s + 0x3c0);
    zero(m, s + 0x3f0, 6);
    for i in 0..8u32 {
        m.set_u32(s + 0x1f8 + 4 * i - 0x1f8, 0);
        m.set_u32(s + 0x1f8 + 4 * i + 0x20, 0);
        m.set_u32(s + 0x1f8 + 4 * i, 0);
        copy_words(m, s + 0x298 + 12 * i - 0x60, ZERO3, 3);
        copy_words(m, s + 0x298 + 12 * i, ZERO3, 3);
        copy_words(m, s + 0x298 + 12 * i + 0x7c, ZERO3, 3);
    }
    for o in [0x20, 0x50, 0x80, 0xb0, 0xe0] {
        identity(m, s + o);
    }
    for o in [0x110, 0x11c, 0x128, 0x134, 0x140, 0x158, 0x164, 0x170, 0x194, 0x1b8, 0x1c4] {
        copy_words(m, s + o, ZERO3, 3);
    }
    for o in [0x2f8, 0x308, 0x304, 0x300, 0x2fc] {
        m.set_f32(s + o, 0.1);
    }
    m.set_u32(s + 0x1f4, 0);
    m.set_u32(s + 0x30c, 1);
    m.set_u32(s + 0x310, 0);
    copy_words(m, s + 0x374, ZERO3, 3);
    copy_words(m, s + 0x380, ZERO3, 3);
    copy_words(m, s + 0x38c, UP, 3);
    for o in [0x398, 0x3a4, 0x3b0] {
        copy_words(m, s + o, ZERO3, 3);
    }
    m.set_f32(s + 0x3bc, 1.0);
    identity(m, s + 0x3c0);
    m.set_u32(s + 0x408, 1);
}

/// `0x4eb770`: the driver input object at `+0x2700`.
pub fn fn_004eb770(m: &mut Mem, s: u32) {
    zero(m, s + 0x90, 12);
    identity(m, s + 0x90);
    zero(m, s + 200, 15);
    zero(m, s + 0x104, 15);
    zero(m, s + 0x140, 15);
    zero(m, s + 0x17c, 15);
    zero(m, s + 0x1b8, 15);
    zero(m, s + 500, 3);
    zero(m, s + 0x200, 48);
    for o in [6, 4, 0x40, 0x42] {
        m.set_i16(s + o, 0);
    }
    let k = m.i16(0x696bda);
    m.set_u32(s + 0x2d4, 0);
    m.set_i16(s + 8, k);
    m.set_f32(s + 0x24, 0.4);
    m.set_f32(s + 0x28, 0.6);
    m.set_i16(s + 0x4a, 1);
    m.set_u32(s + 0x5c, 0);
    m.set_u32(s + 0x50, 1);
    m.set_u32(s + 0x54, 1);
    m.set_u32(s + 0x4c, 0);
    m.set_f32(s + 100, 0.3);
    m.set_u32(s + 0x58, 1);
    m.set_u32(s + 0x68, 1);
    m.set_u32(s + 0x6c, 0);
    m.set_u32(s + 0x70, 0);
    let up = v3(m, UP);
    let k25 = -2.5f32 as f64;
    for i in 0..5u32 {
        let b = s + 0x104 + 12 * i;
        set3(m, b - 0x3c, [r(up[0] * k25), r(up[1] * k25), r(up[2] * k25)]);
        copy_words(m, b, ZERO3, 3);
        copy_words(m, b + 0x3c, ZERO3, 3);
        copy_words(m, b + 0x78, ZERO3, 3);
        copy_words(m, b + 0xb4, ZERO3, 3);
    }
    m.set_u32(s + 0x74, 1);
    for o in [0xc0, 0xc4, 0x78, 0x88, 0x7c, 0x60, 0x80] {
        m.set_u32(s + o, 0);
    }
    m.wr8(s, 2);
    m.set_u32(s + 0x8c, 0);
    m.set_u32(s + 0x84, 0);
    for i in 0..4u32 {
        for j in 0..4u32 {
            copy_words(m, s + 0x200 + 0x30 * i + 12 * j, ZERO3, 3);
        }
        m.set_u32(s + 0x2c0 + 4 * i, 0);
    }
    m.set_u32(s + 0x2d0, 0);
    crate::chassis::fn_004eba10(m, s);
}

/// Per-vehicle model transforms set up by the vehicle loader (`0x53fdb0`): the model matrix at
/// `+0x1d0` and its inverse at `+0x200` of a 0x5d0-byte entry per vehicle type.
pub const MODEL_TABLE: u32 = 0x6d2200;
pub const MODEL_ENTRY: u32 = 0x5d0;

/// `_Car_V::_Car_V` (`0x4ebd00`): build vehicle `vid` at the transform `mat` in the room `room`.
pub fn fn_004ebd00(m: &mut Mem, world: &mut dyn World, this: u32, room: u32, mat: [f32; 12], vid: i32) {
    let p = |i: u32| this + 4 * i;
    members(m, this);
    m.set_u32(this, 0x64d994); // the virtual table of _Car_V
    let tnow = crate::globals::time_hi(m);
    m.set_f32(p(2), r(tnow));
    m.set_u32(p(3), 0);
    let tech = crate::gearbox::TECH_TABLE + (vid as u32).wrapping_mul(0x970);
    let rf = |m: &Mem, o: u32| m.f32(tech + o);
    m.set_i32(p(0x233), vid);
    m.set_f32(p(0x990), rf(m, 0x60));
    copy_words(m, p(0x7d2), tech + 0x19c, 3);
    {
        let a = [m.f32(0x6974a8), m.f32(0x6974ac), m.f32(0x6974b0)];
        crate::math::vec_div(m, p(0x7d5), a, p(0x7d2));
    }
    identity(m, p(4));
    identity(m, p(0x240));
    // the model matrix of the vehicle type, inverted, for the wheel mounts
    let model = MODEL_TABLE + 0x1b0 + (vid as u32).wrapping_mul(MODEL_ENTRY);
    let inv = SCRATCH;
    crate::math::mat_inverse(m, inv, model);
    let mut n = m.i32(tech + 0x860);
    m.set_i32(p(0x375), n);
    if m.i32(0x6ce33c) <= vid && vid < m.i32(0x6ce340) {
        n += 1;
        m.set_i32(p(0x375), n);
    }
    m.set_i32(p(0xcd0), m.i32(p(0x375)));
    let l: [f64; 12] = core::array::from_fn(|i| f(m, inv + 4 * i as u32));
    let up = v3(m, UP);
    for k in 0..8u32 {
        let travel = rf(m, 0x2ec + 4 * k) as f64;
        let spring = m.u32(tech + 0x30c + 4 * k);
        m.set_f32(p(0x7e7 - 8 + k), r(travel * 4.0 * (0.333333333f32 as f64)));
        m.set_f32(p(0x7e7 + k), travel as f32);
        m.set_u32(p(0x7e7 + 8 + k), spring);
        let wp = v3(m, tech + 0x1a8 + 12 * k);
        let mount = [
            r(l[0] * wp[0] + l[3] * wp[1] + l[6] * wp[2] + l[9]),
            r(l[1] * wp[0] + l[4] * wp[1] + l[7] * wp[2] + l[10]),
            r(l[2] * wp[0] + l[5] * wp[1] + l[8] * wp[2] + l[11]),
        ];
        set3(m, p(0x72f + 3 * k), mount);
        let f2 = r(0.75f32 as f64 * travel + rf(m, 0x64) as f64) as f64;
        let mo = [mount[0] as f64, mount[1] as f64, mount[2] as f64];
        set3(m, p(0x72f + 3 * k - 0x11e), [r(mo[0] - r(f2 * up[0]) as f64), r(mo[1] - r(f2 * up[1]) as f64), r(mo[2] - f2 * up[2])]);
        copy_words(m, p(0x7e7 - 0x98 + k), tech + 0x32c + 4 * k, 1);
    }
    let g = [r(-up[0] * 9.80000019f32 as f64), r(-up[1] * 9.80000019f32 as f64), r(-up[2] * 9.80000019f32 as f64)];
    set3(m, p(0x350), g);
    let mass = f(m, p(0x990));
    set3(m, p(0x315), [r(mass * g[0] as f64), r(mass * g[1] as f64), r(mass * g[2] as f64)]);
    let gz = m.f32(p(0x352)).abs();
    m.set_f32(p(0x98f), gz);
    m.set_f32(p(0x35f), gz);
    let torque = rf(m, 0xc0) as f64;
    m.set_i32(p(0xfbb), 3);
    ts::define(m, this + 0x3ebc, 0, r(torque * 0.15f64 as f32 as f64), torque as f32, 1.0);
    ts::define(m, this + 0x3ebc, 1, r(torque * 0.25f64 as f32 as f64), r(torque * 1.4f64 as f32 as f64), 1.0);
    let (tank, endurance) = (rf(m, 0x6c), rf(m, 0x70));
    m.set_i32(p(0xfc8), 2);
    ts::define(m, this + 0x3ef0, 0, 0.0, tank, endurance);
    ts::define(m, this + 0x3ef0, 1, 0.0, r(tank as f64 + tank as f64), r(endurance as f64 + endurance as f64));
    let fuel = ts::value(m, this + 0x3ef0);
    m.set_f32(p(0xf1e), fuel);
    m.set_u32(p(0xf1f), 0);
    m.set_u32(p(0x800), this);
    for k in 0..4 {
        fn_004eb3c0(m, this + 0x1ffc, 300.0, k);
    }
    let inertia = SCRATCH + 0x40;
    copy_words(m, inertia, p(0x7d2), 3);
    fn_004ee540(m, this, rf(m, 0x60), model, inertia);
    copy_words(m, p(0x991), tech + 0xac, 3);
    let mut minx = 100.0f32;
    let mut maxx = -100.0f32;
    let mut miny = 100.0f32;
    let mut maxy = -100.0f32;
    for k in 0..8u32 {
        let x = m.f32(p(0x730 + 3 * k) - 4);
        let y = m.f32(p(0x730 + 3 * k));
        if x < minx {
            minx = x;
        }
        if maxx < x {
            maxx = x;
        }
        if y < miny {
            miny = y;
        }
        if maxy < y {
            maxy = y;
        }
    }
    copy_words(m, p(0x9ad), UP, 3);
    let xr = r(maxx as f64 - minx as f64) as f64;
    let q = r(xr * xr * 0.5 * 1.2) as f64;
    m.set_f32(p(0x992), r(q * f(m, p(0x992))));
    let yr = r(maxy as f64 - miny as f64) as f64;
    m.set_f32(p(0x993), r(yr * q / xr * f(m, p(0x993))));
    let v = r(xr * 0.333333343f32 as f64 + yr * 0.100000001f32 as f64) as f64;
    set3(m, p(0x9ad), [r(v * up[0]), r(v * up[1]), r(v * up[2])]);
    m.set_f32(p(0x683), 1.6);
    copy_words(m, p(0x6dc), tech + 0x64, 1);
    for k in 0..8u32 {
        copy_words(m, p(0x6e5 - 8 + k), p(0x6dc), 1);
        copy_words(m, p(0x6e5 + k), tech + 0x68, 1);
        let len = crate::math::vec_len(m, p(0x72f + 3 * k));
        m.set_f32(p(0x6e5 + 0x62 + k), len as f32);
        if f(m, p(0x683)) < len {
            m.set_f32(p(0x683), len as f32);
        }
    }
    let mount = |m: &Mem, k: u32| v3(m, p(0x72f + 3 * k));
    let dist = |a: [f64; 3], b: [f64; 3]| {
        let d = [r(a[0] - b[0]) as f64, r(a[1] - b[1]) as f64, r(a[2] - b[2]) as f64];
        (d[2] * d[2] + d[1] * d[1] + d[0] * d[0]).sqrt()
    };
    if m.i32(p(0x375)) < 4 {
        let d = dist(mount(m, 0), mount(m, 1)) as f32;
        for k in 0..5 {
            m.set_f32(p(0x7d8 + k), d);
        }
    } else {
        m.set_f32(p(0x7d9), dist(mount(m, 2), mount(m, 0)) as f32);
        m.set_f32(p(0x7da), dist(mount(m, 0), mount(m, 1)) as f32);
        m.set_f32(p(0x7db), dist(mount(m, 1), mount(m, 3)) as f32);
        m.set_f32(p(0x7dc), dist(mount(m, 3), mount(m, 2)) as f32);
        let a = dist(mount(m, 1), mount(m, 3)) as f32 as f64;
        let b = dist(mount(m, 0), mount(m, 2));
        m.set_f32(p(0x7d8), r((b + a) * 0.5));
        m.set_f32(p(0x7dd), m.f32(p(0x730)).abs());
        m.set_f32(p(0x7de), m.f32(p(0x736)).abs());
        if m.i32(p(0x375)) == 6 {
            let a = dist(mount(m, 1), mount(m, 5)) as f32 as f64;
            let b = dist(mount(m, 0), mount(m, 4));
            m.set_f32(p(0x7d8), r((b + a) * 0.5));
            m.set_f32(p(0x7de), m.f32(p(0x73c)).abs());
        }
    }
    for k in 0..8u32 {
        copy_words(m, p(0x6fd - 0x10 + k), tech + 0x12c + 4 * k, 1);
        copy_words(m, p(0x6fd + k), tech + 0x16c + 4 * k, 1);
        copy_words(m, p(0x6fd - 8 + k), tech + 0x14c + 4 * k, 1);
    }
    m.set_i32(p(0x707), 1);
    m.set_i32(p(0x708), -1);
    m.set_i32(p(0x705), 1);
    let gears = m.i32(tech + 0x90);
    m.set_i32(p(0x706), gears);
    m.set_i32(p(0xef7), gears);
    copy_words(m, p(0x720), tech + 0xcc, 1);
    let axle = f(m, p(0x720));
    let mut g = 0i32;
    while g <= m.i32(p(0x706)) {
        m.set_f32(p(0x711 + g as u32), r(rf(m, 0xd0 + 4 * g as u32) as f64 * axle));
        g += 1;
    }
    m.set_u32(p(0x721), 0);
    copy_words(m, p(0x722), tech + 0x10c, 1);
    copy_words(m, p(0x723), tech + 0xc4, 1);
    let eb = f(m, p(0x723));
    ts::define(m, this + 0x3ff4, 0, 0.0, 0.0, 0.0);
    ts::define(m, this + 0x3ff4, 1, r(eb * 0.25), r(eb + eb), 0.0);
    ts::define(m, this + 0x3fc0, 0, 1.0, 1.0, 1.0);
    ts::define(m, this + 0x3fc0, 1, f32::from_bits(0x3f4c_cccd), f32::from_bits(0x3f4c_cccd), 1.0);
    m.set_u32(p(0x725), 0);
    m.set_f32(p(0x72e), 1.0);
    m.set_f32(p(0x70e), 0.1);
    m.set_f32(p(0x70c), 0.5);
    m.set_u32(p(0x70d), 0x3ba3_d70a);
    m.set_f32(p(0x70f), r(crate::globals::time_hi(m)));
    m.set_f32(p(0x710), 0.5);
    m.set_u32(p(0x70b), 0);
    m.set_u32(p(0x709), 0);
    ts::define(m, this + 0x3f58, 0, 0.1, 1.0, 60.0);
    ts::define(m, this + 0x3f58, 1, 0.5, 1.0, 120.0);
    ts::define(m, this + 0x3f8c, 0, 0.0, 1.0, 10.0);
    ts::define(m, this + 0x3f8c, 1, 0.0, 1.0, 20.0);
    ts::define(m, this + 0x3f24, 0, f32::from_bits(0x3f19_999a), 1.0, 30000.0);
    ts::define(m, this + 0x3f24, 1, f32::from_bits(0x3f4c_cccd), 1.0, 60000.0);
    ts::define(m, this + 0x3f24, 2, f32::from_bits(0x3f4c_cccd), 1.0, 60000.0);
    m.set_u32(p(0xf15), 0);
    m.set_u32(p(0x6b9), room);
    m.set_u32(p(0x3d), room);
    m.set_u32(p(0x684), room);
    // put the vehicle on the ground: query straight down from two units above to three below
    for (i, v) in mat.iter().enumerate() {
        m.set_f32(p(4) + 4 * i as u32, *v);
    }
    let rec = SCRATCH + 0x100;
    rec_init(m, rec);
    identity(m, rec + 4);
    let pos = v3(m, p(0xd));
    set3(m, rec + 0x4c, [r(up[0] + up[0] + pos[0]), r(up[1] + up[1] + pos[1]), r(up[2] + up[2] + pos[2])]);
    let (s1, s2) = (r(up[1] * 3.0) as f64, r(up[2] * 3.0) as f64);
    set3(m, rec + 0x58, [r(pos[0] - up[0] * 3.0), r(pos[1] - s1), r(pos[2] - s2)]);
    m.set_u32(rec + 0x74, 0);
    m.set_u32(rec + 0x78, 0);
    m.set_u32(rec, 0);
    m.set_u32(rec + 0x68, 0);
    let res = if m.i16(p(0x9c2)) == 0 {
        let res = world.collide(m, room, 4, rec);
        if res & 2 != 0 && m.u32(rec + 0x68) == 0 {
            m.set_u32(p(0xf15), 1);
        }
        res
    } else {
        world.room_collide(m, 0, room, 4, rec)
    };
    if res & 1 != 0 {
        let h = r(0.75f32 as f64 * f(m, p(0x7e7)) + (f(m, p(0x6dc)) - f(m, p(0x731)))) as f64;
        let nrm = v3(m, rec + 0x40);
        let hit = v3(m, rec + 0x34);
        set3(m, p(0xd), [r(hit[0] + r(nrm[0] * h) as f64), r(r(nrm[1] * h) as f64 + hit[1]), r(h * nrm[2] + hit[2])]);
    }
    copy_words(m, p(0x6ba), p(4), 12);
    copy_words(m, p(0x234), p(4), 12);
    let invm = SCRATCH + 0x200;
    crate::math::mat_inverse(m, invm, p(4));
    copy_words(m, p(0x240), invm, 12);
    copy_words(m, p(0x24c), invm, 12);
    ctor_wheels(m, world, this);
}

/// The wheel part of the constructor and everything after it.
fn ctor_wheels(m: &mut Mem, world: &mut dyn World, this: u32) {
    let p = |i: u32| this + 4 * i;
    let up = v3(m, UP);
    copy_words(m, p(0x31), ZERO3, 3);
    let mt = |m: &Mem| -> [f64; 12] { core::array::from_fn(|i| f(m, p(4) + 4 * i as u32)) };
    for k in 0..8u32 {
        let w = this + 0xf8 * (k + 1);
        set3(m, w + 0xc, [0.0, 0.0, 1.0]);
        crate::matrix::vec_add_in_place(m, p(0x31), SCRATCH + 0x300, w + 0xc);
        let d = r(0.75f32 as f64 * f(m, p(0x7e7 + k)));
        for o in [0x376, 0x37e, 0x386] {
            m.set_f32(p(o + k), d);
        }
        let fz = r(d as f64 + f(m, p(0x6dc))) as f64;
        let (ux, uy) = (r(fz * up[0]) as f64, r(fz * up[1]) as f64);
        let mo = v3(m, p(0x72f + 3 * k));
        let a = mt(m);
        let px = r(mo[0] * a[0] + a[6] * mo[2] + a[3] * mo[1] + a[9]) as f64;
        let py = r(a[7] * mo[2] + a[4] * mo[1] + a[1] * mo[0] + a[10]) as f64;
        let pz = r(a[8] * mo[2] + a[5] * mo[1] + a[2] * mo[0] + a[11]) as f64;
        set3(m, w, [r(px - ux), r(py - uy), r(pz - fz * up[2])]);
        m.set_i16(w + 0x18, 1);
        m.set_i16(w + 0x1a, 0);
        copy_words(m, this + 0x1018 + 0xf8 * k, w, 0x3e);
        let f2 = r(f(m, p(0x6e5 - 8 + k)) + d as f64) as f64;
        let (ex, ey) = (r(f2 * up[0]) as f64, r(f2 * up[1]) as f64);
        let (bx, by, bz) = (r(mo[0] - ex) as f64, r(mo[1] - ey) as f64, r(mo[2] - f2 * up[2]) as f64);
        let c = [
            r(bx * a[0] + by * a[3] + bz * a[6] + a[9]),
            r(by * a[4] + bz * a[7] + bx * a[1] + a[10]),
            r(by * a[5] + bz * a[8] + bx * a[2] + a[11]),
        ];
        set3(m, p(0x19 + 3 * k), c);
        set3(m, p(0x5f9 + 3 * k), c);
        m.set_i16(this + 0x19a4 + 2 * k, 0);
        for o in [0x29, 0x21, 0x19, 0xb6e, 0x7c, 0x6c] {
            m.set_u32(p(0x376 + k + o), 0);
        }
        m.set_u32(this + 0x3574 + 0xd0 * k, 0);
    }
    let e = |m: &Mem, k: u32| m.f32(this + 0xe18 + 4 * k);
    let a = if e(m, 3) <= e(m, 2) { e(m, 2) } else { e(m, 3) };
    let b = if e(m, 0) < e(m, 1) { e(m, 1) } else { e(m, 0) };
    m.set_f32(p(0x38e), if a <= b { b } else { a });
    crate::math::vec_normalize(m, p(0x31));
    copy_words(m, p(0x34), p(0x31), 3);
    copy_words(m, p(0x757), p(0x6dc), 0x7b);
    fn_004ef480(m, this);
    let n = m.i32(p(0x375)).max(0) as u32;
    for k in 0..n {
        let v = r(-(f(m, p(0x352)) * f(m, p(0x994 + k)) * f(m, p(0x990))));
        m.set_f32(p(0x348 + k), v);
        let v15 = r(v as f64 * 1.5);
        m.set_f32(p(0x726 + k), v15);
        m.set_f32(p(0x7a1 + k), v15);
    }
    for k in 0..n {
        let v = r(f(m, p(0x7e7 + k)) - (f(m, p(0x348 + k)) / f(m, p(0x7ef + k))).abs());
        m.set_f32(p(0x376 + k), v);
        if 0.0 <= v {
            if m.f32(p(0x7e7 + k)) < v {
                copy_words(m, p(0x376 + k), p(0x7e7 + k), 1);
            }
        } else {
            m.set_u32(p(0x376 + k), 0);
        }
    }
    for i in [0x10, 0x11, 0x12, 0x37, 0x38, 0x39, 0x16, 0x17, 0x18, 0x3a, 0x3b, 0x3c, 0x13, 0x14, 0x15, 0x30c, 0x30d, 0x30e, 0x309, 0x30a, 0x30b, 0x31e, 799, 800, 0x363, 0x364, 0x365] {
        m.set_u32(p(i), 0);
    }
    m.set_u32(p(0x36f), 0x5015_02f9);
    for i in [0x353, 0x354, 0x355, 0x374, 0x373, 0x370, 0x371] {
        m.set_u32(p(i), 0);
    }
    let a = mt(m);
    let k3 = v3(m, 0x702620);
    let fx = r(k3[0] * a[0] + k3[1] * a[3] + k3[2] * a[6]);
    let fy = r(k3[0] * a[1] + k3[1] * a[4] + k3[2] * a[7]);
    let fz = r(k3[0] * a[2] + k3[1] * a[5] + k3[2] * a[8]);
    set3(m, p(0x28b), [fx, fy, fz]);
    fn_004f1dd0(m, this, ZERO3);
    fn_004f1ef0(m, this, ZERO3);
    m.set_u32(p(0x3b8), 0);
    m.set_u32(p(0x3b9), 0);
    crate::chassis::fn_004eba10(m, this + 0x2700);
    for i in [0x6c7, 0x6c6, 0x6c8] {
        m.set_u32(p(i), 0);
    }
    for i in [0x5f6, 0x5f7, 0x5f8] {
        m.set_f32(p(i), r(crate::globals::time_hi(m)));
    }
    m.set_u32(p(0xa78), 0);
    m.set_u32(p(0xa77), 0);
    m.set_f32(p(0xa76), 0.05);
    m.set_u32(p(0xa7a), 0);
    copy_words(m, p(0xd2d), p(0x6dc), 1);
    for i in [0xcd1, 0xd33, 0xf06, 0xcce, 0xccf] {
        m.set_u32(p(i), 0);
    }
    let tech = crate::gearbox::TECH_TABLE + (m.i32(p(0x233)) as u32).wrapping_mul(0x970);
    for k in 0..8u32 {
        m.set_u32(p(0xd3c - 8 + k), 0);
        let off = if k / 2 == 0 { 0xb8 } else { 0xbc };
        copy_words(m, p(0xd3c + k), tech + off, 1);
    }
    m.set_u32(p(0xef9), 1);
    for i in [0xf04, 0xf05, 0xf0d, 0xf0e, 0xf0f, 0xf10] {
        m.set_u32(p(i), 0);
    }
    m.set_u32(p(0xef6), 1);
    for i in [0xef5, 0xef4, 0xf14, 0xf20, 0x6c9, 0xf24, 0xf25, 0xf26] {
        m.set_u32(p(i), 0);
    }
    m.set_f32(p(0x6ca), 0.5);
    m.set_u32(p(0x6cb), 0);
    for k in 0..8u32 {
        m.set_f32(p(0xeec - 0x550 + k), 0.5);
        m.set_f32(p(0xeec + k), 0.5);
    }
    m.set_f32(p(0x9ac), 0.5);
    for i in [0x685, 0x686, 0x688, 0x689, 0x68a, 0x696, 0x695, 0x694] {
        m.set_u32(p(i), 0);
    }
    for k in 0..8u32 {
        let rv = crate::globals::rand(m);
        let v = r(rv as f32 as f64 * 9.81747688e-05);
        let b = p(0x3a7 + k);
        m.set_f32(b, v);
        m.set_u32(b + 4 * 8, 0);
        m.set_f32(b + 4 * 0x934, v);
        m.set_u32(b + 4 * 0x2e5, 0);
        m.set_u32(b + 4 * 0x93c, 0);
        m.set_f32(b + 4 * 0x2f8, 1.0);
        m.set_u32(b + 4 * 0x2f0, 0);
    }
    let nrm = v3(m, p(0x34));
    for k in 0..8u32 {
        let b = p(0x292 + 3 * k);
        let q = p(0x661 + k);
        copy_words(m, b + 4 * 0x397, p(0x27c), 3);
        copy_words(m, q - 4 * 8, p(0x372), 1);
        copy_words(m, q, p(0x372), 1);
        copy_words(m, b + 4 * 0x3af, p(0x28e), 3);
        copy_words(m, b - 4, p(0x28e), 3);
        copy_words(m, b + 4 * 0x17, p(0x28e), 3);
        let v = v3(m, b - 4);
        let c = [r(nrm[1] * v[2] - nrm[2] * v[1]), r(v[0] * nrm[2] - v[2] * nrm[0]), r(nrm[0] * v[1] - nrm[1] * v[0])];
        set3(m, b + 4 * 0x2f, c);
        set3(m, b + 4 * 0x47, c);
        m.set_u32(q + 4 * 0x36, 0);
        m.set_f32(q + 4 * 0x3e, 1.0);
        m.set_u32(q + 4 * 0x2b, 0);
        m.set_f32(q - 4 * 0x321, r(f(m, q + 4 * 0x33b) * f(m, q - 4 * 0x319)));
    }
    // decompose and rebuild the transform (removes scaling and shear)
    let cur: [f32; 12] = core::array::from_fn(|i| m.f32(p(4) + 4 * i as u32));
    let d = SCRATCH + 0x400;
    crate::chassis::fn_005de950(m, cur, d, d + 8, d + 16, d + 24, d + 32, d + 40);
    let dv: Vec<f64> = (0..6).map(|i| m.f64(d + 8 * i)).collect();
    crate::matrix::mat_from_euler(m, SCRATCH + 0x440, dv[0], dv[1], dv[2], dv[3], dv[4], dv[5]);
    copy_words(m, p(4), SCRATCH + 0x440, 12);
    identity(m, p(0x801));
    for k in 0..0x1bu32 {
        let b = p(0xf2b + 5 * k);
        m.set_u32(b - 12, 0);
        m.set_u32(b, 0);
        m.set_u32(b + 4, 0x3c88_8889);
        m.set_u32(b - 4, 0);
        m.set_u32(b - 8, 1);
    }
    for k in 0..8u32 {
        let b = p(0xf61 + 5 * k);
        m.set_i32(b - 4, 2);
        m.set_i32(b, -2);
        m.set_i32(b - 4 * 0x29, 3);
        m.set_f32(b - 4 * 0x26, 0.05);
    }
    m.set_u32(p(0xf31), 0x3c36_0b61);
    m.set_i32(p(0xf2e), 2);
    m.set_u32(p(0xf2c), 0x3c88_8889);
    m.set_i32(p(0xf29), 3);
    m.set_i32(p(0xf33), 3);
    m.set_i32(p(0xf34), -3);
    for k in 0..8u32 {
        m.set_i32(p(0xf88 + 5 * k), 1);
    }
    crate::chassis::fn_004efc10(m, this);
    fn_004f1dd0(m, this, ZERO3);
    fn_004f1ef0(m, this, ZERO3);
    let room = m.u32(p(0x6b9));
    fn_004f1240(m, world, this, 2, room, p(0x6ba));
    if m.i16(p(0x44)) != 0 {
        let pos = v3(m, p(0xd));
        let wp = v3(m, p(0x3e));
        let wn = v3(m, p(0x41));
        m.set_f32(p(0x888), r(wn[0] * r(pos[0] - wp[0]) as f64 + r(pos[1] - wp[1]) as f64 * wn[1] + r(pos[2] - wp[2]) as f64 * wn[2]));
    }
    copy_words(m, p(0x885), ZERO3, 3);
    for k in 0..m.i32(p(0x375)).max(0) as u32 {
        crate::matrix::vec_add_in_place(m, p(0x885), SCRATCH + 0x300, this + 0xf8 * (k + 1));
    }
    crate::matrix::vec_scale_in_place(m, p(0x885), SCRATCH + 0x300, 0.25);
    m.set_u32(p(0x33c), 0);
    m.set_u32(p(0x33d), 0);
    m.set_f32(p(0x33e), 1000.0);
    m.set_f32(p(0x33f), 100.0);
    for i in [0x324, 0x336, 0x327, 0x339] {
        copy_words(m, p(i), ZERO3, 3);
    }
    m.set_i32(p(0x98e), 1);
    m.set_i32(p(0x98d), -1);
    m.set_u32(p(0x98c), 0);
    fn_004f7380(m, this, 1);
    let ang = SCRATCH + 0x480;
    let cur: [f32; 12] = core::array::from_fn(|i| m.f32(p(4) + 4 * i as u32));
    crate::chassis::fn_005de950(m, cur, ang, ang + 8, ang + 16, ang + 24, ang + 32, ang + 40);
    crate::chassis::fn_004f6b70(m, this, ang);
    m.set_u32(p(0xf16), 0);
    copy_words(m, p(0xf17), ZERO3, 3);
    m.set_u32(p(0xf21), 0);
    m.set_u32(p(0xf22), 0);
    for k in 0..8u32 {
        m.set_u32(p(0x9b8 - 8 + k), 0);
        m.set_u32(p(0x9b8 + k), 0);
    }
    m.set_u32(p(0xf23), 0);
}

/// Scratch memory standing in for the stack frames of the functions here.
const SCRATCH: u32 = 0x2ff7_0000;

fn get12(m: &Mem, a: u32) -> [f64; 12] {
    core::array::from_fn(|i| f(m, a + 4 * i as u32))
}

/// `0x4eb3c0`: add a random timer (`period` seconds on average, `kind`) to the list at `list`.
/// The node (0x28 bytes, on the heap) is a list item at `+4` with the list at `+0xc`, the period
/// at `+0x18`, the kind at `+0x1c` and the owner (the dword after the list head) at `+0x24`.
pub fn fn_004eb3c0(m: &mut Mem, list: u32, period: f32, kind: i32) {
    let node = m.alloc(0x28);
    let owner = m.u32(list + 4);
    // 0x592c40: link the item at node+4 to the tail of the list
    let item = node + 4;
    m.set_u32(item, 0);
    m.set_u32(item + 4, 0);
    m.set_u32(node, 0x65089c);
    let head = m.u32(list);
    if head != 0 {
        let mut last = head;
        while m.u32(last + 4) != 0 {
            last = m.u32(last + 4);
        }
        m.set_u32(last + 4, item);
        m.set_u32(item, last);
    } else {
        m.set_u32(list, item);
    }
    m.set_u32(node + 0xc, list);
    m.set_f32(node + 0x18, period);
    m.set_u32(node + 0x20, 0);
    m.set_u32(node, 0x64d96c);
    m.set_u32(node + 0x24, owner);
    m.set_i32(node + 0x1c, kind);
    fn_004eb2f0(m, node);
}

/// `0x4eb0c7`: a normally distributed random number (Box-Muller) with mean `mean` and deviation `dev`.
pub fn fn_004eb0c7(m: &mut Mem, mean: f32, dev: f32) -> f64 {
    let u1 = r(crate::globals::rand(m) as f32 as f64 / 32768.0) as f64;
    let u2 = r(crate::globals::rand(m) as f32 as f64 / 32768.0) as f64;
    ((u1.ln() * -2.0).sqrt() * dev as f64) * (u2 * 6.2831852).cos() + mean as f64
}

/// `0x4eb2f0`: schedule the next firing of a timer: now plus a random interval of at least half the period.
pub fn fn_004eb2f0(m: &mut Mem, node: u32) {
    let now = r(crate::globals::time_hi(m));
    m.set_f32(node + 0x10, now);
    let p = m.f32(node + 0x18);
    let mut g = fn_004eb0c7(m, p, r(p as f64 * 0.3));
    let half = p as f64 * 0.5;
    if g < half {
        g = (half - g) + half;
    }
    m.set_f32(node + 0x14, r(g + now as f64));
}

/// `0x4ee540`: mass, inertia and the wheel mounts (from the `vehicle.tech` wheel positions through
/// the inverse of the model matrix `model`), then the static wheel loads and spring rates.
pub fn fn_004ee540(m: &mut Mem, this: u32, mass: f32, model: u32, inertia: u32) {
    m.set_f32(this + 0x2640, mass);
    copy_words(m, this + 0x1f48, inertia, 3);
    let one = [m.f32(0x6974a8), m.f32(0x6974ac), m.f32(0x6974b0)];
    crate::math::vec_div(m, this + 0x1f54, one, this + 0x1f48);
    let inv = SCRATCH + 0x500;
    crate::math::mat_inverse(m, inv, model);
    copy_words(m, this + 0x2790, inv, 12);
    let l = get12(m, inv);
    let up = v3(m, UP);
    let n = m.i32(this + 0xdd4);
    let tech = crate::gearbox::TECH_TABLE + (m.i32(this + 0x8cc) as u32).wrapping_mul(0x970);
    for k in 0..n.max(0) as u32 {
        let w = v3(m, tech + 0x1a8 + 12 * k);
        let mo = this + 0x1cbc + 12 * k;
        set3(
            m,
            mo,
            [
                r(l[0] * w[0] + l[3] * w[1] + l[6] * w[2] + l[9]),
                r(l[4] * w[1] + l[7] * w[2] + l[1] * w[0] + l[10]),
                r(l[5] * w[1] + l[8] * w[2] + l[2] * w[0] + l[11]),
            ],
        );
        if k as i32 == n - 1 && m.i32(this + 0x2778) == 2 {
            fn_004ee980(m, this, 0);
        }
        let len = crate::math::vec_len(m, mo);
        m.set_f32(this + 0x1d1c + 4 * k, len as f32);
        copy_words(m, mo + 0x1708, mo, 3);
        copy_words(m, this + 0x1d1c + 4 * k + 0x1708, this + 0x1d1c + 4 * k, 1);
        let p = v3(m, mo);
        let k4 = 0.4f32 as f64;
        set3(m, mo - 0x478, [r(p[0] - up[0] * k4), r(p[1] - r(up[1] * k4) as f64), r(p[2] - r(up[2] * k4) as f64)]);
    }
    m.set_f32(this + 0x1f74, m.f32(this + 0x1cc0).abs());
    m.set_f32(this + 0x1f78, m.f32(this + 0x1cd8).abs());
    if n == 6 {
        m.set_f32(this + 0x1f78, m.f32(this + 0x1cf0).abs());
    }
    let f2 = r(0.75 * f(m, this + 0x1f9c)) as f64;
    copy_words(m, this + 0x34b0, this + 0x2640, 1);
    copy_words(m, this + 0x34b8, this + 0x1f48, 2);
    m.set_f32(this + 0x2220, r(f2 + (f(m, this + 0x1b70) - f(m, this + 0x1cc4))));
    copy_words(m, this + 0x34c0, this + 0x1f50, 1);
    fn_004ef480(m, this);
    static_loads(m, this, None);
    if m.i32(this + 0x2778) == 1 {
        let extra = m.f32(this + 0x2788);
        fn_004eeaf0(m, this, extra);
    }
}

/// The weight force and the static wheel loads, spring rates and load limits shared by
/// `0x4ee540` and `0x4eeaf0` (`extra`: the semitrailer load spread over the rear wheels).
fn static_loads(m: &mut Mem, this: u32, extra: Option<f32>) {
    let up = v3(m, UP);
    let mass = f(m, this + 0x2640);
    let g = 9.80000019f32 as f64;
    let (ny, nz) = (r(-up[1]) as f64, r(-up[2]) as f64);
    m.set_f32(this + 0xc54, r(-up[0] * mass * g));
    m.set_f32(this + 0xc58, r(ny * mass * g));
    m.set_f32(this + 0xc5c, r(nz * mass * g));
    let n = m.i32(this + 0xdd4);
    for k in 0..n.max(0) as u32 {
        let pf = this + 0x1f7c + 4 * k;
        let mut load = r((f(m, pf + 4 * 0x1b5) * f(m, this + 0xc5c)).abs());
        if let Some(e) = extra {
            if k > 1 {
                load = r(e as f64 / (n - 2) as f32 as f64 + load as f64);
            }
        }
        let fv4 = f(m, pf) - 0.75 * f(m, pf + 0x20);
        m.set_f32(pf + 0x40, r(load as f64 / ((fv4 / f(m, pf) + 1.0) * fv4)));
        m.set_f32(pf - 0x240, load);
        m.set_f32(pf - 0x2e4, r(load as f64 * 1.5));
    }
}

/// `0x4ee980`: the saddle of a semitrailer as an extra wheel (`refresh`: only reset its height).
pub fn fn_004ee980(m: &mut Mem, this: u32, refresh: i32) {
    if m.i32(this + 0x2778) != 2 {
        return;
    }
    let n = m.i32(this + 0xdd4) as u32;
    if refresh != 0 {
        copy_words(m, this + 0x1ea4 + 12 * n, this + 0x1cc4, 1);
        copy_words(m, this + 0x1cb8 + 12 * n, this + 0x1cc4, 1);
        return;
    }
    let tech = crate::gearbox::TECH_TABLE + (m.i32(this + 0x8cc) as u32).wrapping_mul(0x970);
    let s = v3(m, tech + 0x7dc);
    let a = get12(m, this + 0x2790);
    let d = this + (n + 0x28d) * 12;
    set3(
        m,
        d,
        [
            r(s[0] * a[0] + a[3] * s[1] + a[6] * s[2] + a[9]),
            r(a[4] * s[1] + a[1] * s[0] + a[7] * s[2] + a[10]),
            r(a[5] * s[1] + a[2] * s[0] + a[8] * s[2] + a[11]),
        ],
    );
    m.set_f32(this + 0x1ea4 + 12 * n, r(f(m, this + 0x1cc4) + 0.9));
    copy_words(m, this + (n * 3 + 0x72c) * 4, this + (n * 3 + 0x7a7) * 4, 3);
}

/// `0x4eeaf0`: static loads with a semitrailer load `extra` on the rear wheels.
pub fn fn_004eeaf0(m: &mut Mem, this: u32, extra: f32) {
    fn_004ef480(m, this);
    m.set_f32(this + 0x2788, extra);
    static_loads(m, this, Some(extra));
}

/// `0x4ef480`: the share of the weight each wheel carries (`+0x2650`), from the wheel layout.
pub fn fn_004ef480(m: &mut Mem, this: u32) {
    let up = v3(m, UP);
    let (a, b, c) = (-up[0], -up[1], -up[2]);
    let col = |m: &Mem, o: u32| r(a * f(m, this + o) + b * f(m, this + o + 0xc) + c * f(m, this + o + 0x18));
    let dir = [col(m, 0x900), col(m, 0x904), col(m, 0x908)];
    let mo = this + 0x1cbc;
    let out = this + 0x2650;
    let y = |m: &Mem, k: u32| f(m, mo + 12 * k + 4);
    let n = m.i32(this + 0xdd4);
    match n {
        0..=2 => {}
        3 => {
            let (f1, f2) = (y(m, 0), y(m, 2));
            let v = r(f2 / (f2 - f1) * 0.5);
            m.set_f32(out, v);
            m.set_f32(out + 4, v);
            m.set_f32(out + 8, r(f1 / (f1 - f2)));
        }
        4 => {
            let (f1, f2) = (y(m, 0), y(m, 2));
            let v = r(f2 / (f2 - f1) * 0.5);
            m.set_f32(out, v);
            m.set_f32(out + 4, v);
            let w = r(f1 / (f1 - f2) * 0.5);
            m.set_f32(out + 12, w);
            m.set_f32(out + 8, w);
        }
        5 => {
            let f1 = y(m, 4);
            let f2 = r(y(m, 2) + y(m, 0)) as f64;
            let f3 = r(1.0 / (f2 - (f1 + f1))) as f64;
            let v = r(-(f3 * f1) * 0.5);
            for k in 0..4 {
                m.set_f32(out + 4 * k, v);
            }
            m.set_f32(out + 16, r(f2 * f3));
        }
        6 => {
            let f1 = y(m, 0);
            let f2 = r(y(m, 4) + y(m, 2)) as f64;
            let f3 = r(1.0 / (f2 - (f1 + f1))) as f64;
            let v = r(f2 * f3 * 0.5);
            m.set_f32(out, v);
            m.set_f32(out + 4, v);
            let w = r(-(f3 * f1) * 0.5);
            for k in 2..6 {
                m.set_f32(out + 4 * k, w);
            }
        }
        _ => {
            // 0x4ef330: weights from the lever arms of the contact points about `dir`
            let d = [dir[0] as f64, dir[1] as f64, dir[2] as f64];
            let mut len = Vec::new();
            for k in 0..n as u32 {
                let p = v3(m, this + 0x1844 + 12 * k);
                let c = [r(d[1] * p[2] - d[2] * p[1]) as f64, r(d[2] * p[0] - d[0] * p[2]) as f64, r(d[0] * p[1] - d[1] * p[0]) as f64];
                len.push(r((c[2] * c[2] + c[1] * c[1] + c[0] * c[0]).sqrt()) as f64);
            }
            let mut a = Vec::new();
            let mut sum = 0.0f32;
            for i in 0..n as usize {
                let mut x = 1.0f32;
                for (j, l) in len.iter().enumerate() {
                    x = r(if i == j { 1.0 } else { *l } * x as f64);
                }
                sum = r(sum as f64 + x as f64);
                a.push(x);
            }
            let inv = r(1.0 / sum as f64) as f64;
            for (i, x) in a.iter().enumerate() {
                m.set_f32(out + 4 * i as u32, r(inv * *x as f64));
            }
        }
    }
}

/// `0x4f0530`: derived quantities of the velocity (`+0x40`): speed, the velocity along the ground.
pub fn fn_004f0530(m: &mut Mem, this: u32) {
    let g = |m: &Mem, o: u32| f(m, this + o);
    let len = crate::math::vec_len(m, this + 0x40);
    if 5.0 < (g(m, 0xdc0) - len).abs() || len < 1.0 {
        copy_words(m, this + 0xc90, ZERO3, 3);
        copy_words(m, this + 0xcd8, ZERO3, 3);
    }
    m.set_f32(this + 0xdc0, len as f32);
    let f3 = r(g(m, 0x44) * g(m, 0xd8) - g(m, 0x48) * g(m, 0xd4)) as f64;
    let f5 = r(g(m, 0xd0) * g(m, 0x48) - g(m, 0xd8) * g(m, 0x40)) as f64;
    let f4 = r(g(m, 0xd4) * g(m, 0x40) - g(m, 0xd0) * g(m, 0x44)) as f64;
    set3(
        m,
        this + 0x9f0,
        [r(f4 * g(m, 0xd4) - f5 * g(m, 0xd8)), r(f3 * g(m, 0xd8) - f4 * g(m, 0xd0)), r(f5 * g(m, 0xd0) - f3 * g(m, 0xd4))],
    );
    let l7 = crate::math::vec_len(m, this + 0x9f0);
    m.set_f32(this + 0xdc4, l7 as f32);
    let d = r(g(m, 0x9f0) * g(m, 0xa38) + g(m, 0x9f4) * g(m, 0xa3c) + g(m, 0x9f8) * g(m, 0xa40)) as f64;
    m.set_f32(this + 0xdc8, d as f32);
    set3(m, this + 0xa20, [r(d * g(m, 0xa38)), r(d * g(m, 0xa3c)), r(d * g(m, 0xa40))]);
    if 0.001 < l7 {
        set3(m, this + 0x9fc, [r(g(m, 0x9f0) / l7), r(g(m, 0x9f4) / l7), r(g(m, 0x9f8) / l7)]);
    } else {
        copy_words(m, this + 0x9fc, this + 0xa38, 3);
    }
}

/// `0x4f1dd0`: set the velocity.
pub fn fn_004f1dd0(m: &mut Mem, this: u32, v: u32) {
    copy_words(m, this + 0x40, v, 3);
    fn_004f0530(m, this);
    crate::chassis::fn_004f0720(m, this);
}

/// `0x4f1ef0`: set the angular velocity (and the angular momentum from it).
pub fn fn_004f1ef0(m: &mut Mem, this: u32, v: u32) {
    copy_words(m, this + 0x58, v, 3);
    let i = v3(m, this + 0x1f48);
    let w = v3(m, this + 0x58);
    let t = [r(w[0] * i[0]) as f64, r(w[1] * i[1]) as f64, r(w[2] * i[2]) as f64];
    let g = |m: &Mem, o: u32| f(m, this + o);
    let x = r(t[0] * g(m, 0x10) + g(m, 0x28) * t[2] + g(m, 0x1c) * t[1]);
    let y = r(g(m, 0x2c) * t[2] + g(m, 0x14) * t[0] + g(m, 0x20) * t[1]);
    let z = r(g(m, 0x30) * t[2] + g(m, 0x18) * t[0] + g(m, 0x24) * t[1]);
    set3(m, this + 0x4c, [x, y, z]);
    crate::chassis::fn_004f07e0(m, this);
}

/// The angular momentum (`+0x4c`) taken into the body frame and divided by the inertia: the
/// angular velocity (`+0x58`), as `0x4f0800` / `0x4f0df0` compute it.
fn omega_from_momentum(m: &mut Mem, this: u32) {
    let g = |m: &Mem, o: u32| f(m, this + o);
    let l = [
        r(g(m, 0x4c) * g(m, 0x900) + g(m, 0x90c) * g(m, 0x50) + g(m, 0x918) * g(m, 0x54)),
        r(g(m, 0x910) * g(m, 0x50) + g(m, 0x904) * g(m, 0x4c) + g(m, 0x91c) * g(m, 0x54)),
        r(g(m, 0x914) * g(m, 0x50) + g(m, 0x908) * g(m, 0x4c) + g(m, 0x920) * g(m, 0x54)),
    ];
    crate::math::vec_div(m, this + 0x58, l, this + 0x1f48);
}

/// `0x4f0800`: refresh the inverse transform, the angular velocity and the forward / side axes.
pub fn fn_004f0800(m: &mut Mem, this: u32) {
    let inv = SCRATCH + 0x540;
    crate::math::mat_inverse(m, inv, this + 0x10);
    copy_words(m, this + 0x900, inv, 12);
    omega_from_momentum(m, this);
    let g = |m: &Mem, o: u32| f(m, this + o);
    let y = v3(m, 0x702620);
    let a = [
        r(y[0] * g(m, 0x10) + y[1] * g(m, 0x1c) + y[2] * g(m, 0x28)),
        r(y[0] * g(m, 0x14) + y[1] * g(m, 0x20) + y[2] * g(m, 0x2c)),
        r(y[0] * g(m, 0x18) + y[1] * g(m, 0x24) + y[2] * g(m, 0x30)),
    ];
    set3(m, this + 0xa2c, a);
    let a = [a[0] as f64, a[1] as f64, a[2] as f64];
    let n = v3(m, this + 0xd0);
    let f2 = n[1] * a[2] - n[2] * a[1];
    let f4 = a[0] * n[2] - a[2] * n[0];
    let f3 = a[1] * n[0] - a[0] * n[1];
    set3(m, this + 0xa38, [r(f4 * n[2] - f3 * n[1]), r(f3 * n[0] - f2 * n[2]), r(f2 * n[1] - f4 * n[0])]);
    crate::math::vec_normalize(m, this + 0xa38);
}

/// `0x4f0c20`: rebuild the transform from its position and angles (removes scale and shear), then
/// take the recorder snapshot.
pub fn fn_004f0c20(m: &mut Mem, this: u32) {
    let cur: [f32; 12] = core::array::from_fn(|i| m.f32(this + 0x10 + 4 * i as u32));
    let d = SCRATCH + 0x580;
    crate::chassis::fn_005de950(m, cur, d, d + 8, d + 16, d + 24, d + 32, d + 40);
    let dv: Vec<f64> = (0..6).map(|i| m.f64(d + 8 * i)).collect();
    crate::matrix::mat_from_euler(m, SCRATCH + 0x5c0, dv[0], dv[1], dv[2], dv[3], dv[4], dv[5]);
    copy_words(m, this + 0x10, SCRATCH + 0x5c0, 12);
    crate::chassis::fn_004f09f0(m, this, d);
}

/// `0x4f0df0`: place the vehicle at the transform `mat` in `room`, with the wheels hanging free.
pub fn fn_004f0df0(m: &mut Mem, this: u32, room: u32, mat: u32) {
    copy_words(m, this + 0x10, mat, 12);
    let inv = SCRATCH + 0x600;
    crate::math::mat_inverse(m, inv, this + 0x10);
    copy_words(m, this + 0x900, inv, 12);
    omega_from_momentum(m, this);
    m.set_u32(this + 0xf4, room);
    fn_004ef480(m, this);
    let g = get12(m, this + 0x10);
    let mass = f(m, this + 0x2640);
    for k in 0..m.i32(this + 0xdd4).max(0) as u32 {
        let p = v3(m, this + 0x1844 + 12 * k);
        set3(
            m,
            this + 0x64 + 12 * k,
            [
                r(p[1] * g[3] + g[0] * p[0] + g[6] * p[2] + g[9]),
                r(p[1] * g[4] + g[7] * p[2] + g[1] * p[0] + g[10]),
                r(p[2] * g[8] + g[2] * p[0] + p[1] * g[5] + g[11]),
            ],
        );
        copy_words(m, this + 0xf08 + 12 * k, ZERO3, 3);
        m.set_u32(this + 0xee8 + 4 * k, 0);
        m.set_u32(this + 0x2690 + 4 * k, 0x3c23_d70a);
        m.set_f32(this + 0x2670 + 4 * k, 0.5);
        m.set_u32(this + 0xf68 + 4 * k, 0);
        m.set_f32(this + 0xd20 + 4 * k, r(f(m, this + 0x2650 + 4 * k) * mass * 10.0));
    }
}

/// `0x4f1fa0`: put the wheels on the plane through `pt` with normal `n` (`reset`: clear the
/// suspension state first). Wheels whose travel line is parallel to the plane hang free.
pub fn fn_004f1fa0(m: &mut Mem, this: u32, reset: i32, pt: u32, n: u32) {
    copy_words(m, this + 0xc4, n, 3);
    copy_words(m, this + 0xd0, n, 3);
    let up = v3(m, UP);
    let nv = v3(m, n);
    let top = SCRATCH + 0x640;
    let bot = SCRATCH + 0x650;
    let hit = SCRATCH + 0x660;
    for k in 0..m.i32(this + 0xdd4).max(0) as u32 {
        let w = this + 0xf8 * (k + 1);
        let mo = this + 0x1cbc + 12 * k;
        if reset != 0 {
            copy_words(m, this + 0xf08 + 12 * k, ZERO3, 3);
            m.set_u32(this + 0xee8 + 4 * k, 0);
            m.set_u32(this + 0xf68 + 4 * k, 0);
            m.set_u32(this + 0x2690 + 4 * k, 0x3c23_d70a);
            m.set_f32(this + 0x2670 + 4 * k, 0.5);
        }
        let a = [up[0] * 100.0, r(up[1] * 100.0) as f64, r(up[2] * 100.0) as f64];
        let p = v3(m, mo);
        let g = get12(m, this + 0x10);
        let tr = |q: [f64; 3]| {
            [
                r(q[0] * g[0] + q[1] * g[3] + q[2] * g[6] + g[9]),
                r(q[0] * g[1] + q[1] * g[4] + q[2] * g[7] + g[10]),
                r(q[1] * g[5] + q[2] * g[8] + q[0] * g[2] + g[11]),
            ]
        };
        let t = tr([a[0] + p[0], a[1] + p[1], a[2] + p[2]]);
        let b = tr([p[0] - a[0], p[1] - a[1], p[2] - a[2]]);
        set3(m, top, t);
        set3(m, bot, b);
        let d = SCRATCH + 0x670;
        set3(m, d, [r(b[0] as f64 - t[0] as f64), r(b[1] as f64 - t[1] as f64), r(b[2] as f64 - t[2] as f64)]);
        crate::matrix::vec_unit(m, d + 12, d);
        let u = v3(m, d + 12);
        if 0.0010000000474974513 <= (nv[2] * u[2] + nv[0] * u[0] + u[1] * nv[1]).abs() {
            crate::matrix::line_plane(m, hit, pt, n, top, bot);
            copy_words(m, w, hit, 3);
            m.set_i16(w + 0x18, 1);
            copy_words(m, w + 0xc, n, 3);
            m.set_i16(w + 0x1a, 0);
            m.set_u32(w + 0x84, 0);
            m.set_u32(w + 0xf0, 0);
            copy_words(m, this + 0x64 + 12 * k, w, 3);
            let q = v3(m, this + 0x64 + 12 * k);
            let iv = get12(m, this + 0x900);
            let lz = r(iv[2] * q[0] + iv[5] * q[1] + iv[8] * q[2] + iv[11]);
            set3(
                m,
                this + 0x1844 + 12 * k,
                [r(iv[0] * q[0] + iv[3] * q[1] + iv[6] * q[2] + iv[9]), r(iv[1] * q[0] + iv[4] * q[1] + iv[7] * q[2] + iv[10]), lz],
            );
            let c = r(f(m, mo + 8) - f(m, this + 0x1b74 + 4 * k) - lz as f64);
            m.set_f32(this + 0xdd8 + 4 * k, c);
            m.set_i16(this + 0x19a4 + 2 * k, 0);
            if c < 0.0 {
                m.set_u32(this + 0xdd8 + 4 * k, 0);
            }
            if m.f32(this + 0x1f9c + 4 * k) < m.f32(this + 0xdd8 + 4 * k) {
                copy_words(m, this + 0xdd8 + 4 * k, this + 0x1f9c + 4 * k, 1);
                m.set_i16(this + 0x19a4 + 2 * k, 1);
            }
        } else {
            m.set_i16(w + 0x1a, 0);
            m.set_i16(w + 0x18, 0);
            copy_words(m, w, ZERO3, 3);
            copy_words(m, w + 0xc, ZERO3, 3);
            m.set_u32(w + 0x84, 0);
            m.set_u32(w + 0xf0, 0);
            copy_words(m, this + 0x1018 + 0xf8 * k, w, 0x3e);
            m.set_i16(this + 0x19a4 + 2 * k, 1);
            copy_words(m, this + 0xdd8 + 4 * k, this + 0x1f9c + 4 * k, 1);
            copy_words(m, this + 0x64 + 12 * k, w, 3);
            copy_words(m, this + 0x17e4 + 12 * k, w, 3);
        }
    }
}

/// `0x4e6621`: query the room `room` along the segment `a`..`b` (`owner`: the object asking).
pub fn fn_004e6621(m: &mut Mem, world: &mut dyn World, owner: u32, room: u32, a: u32, b: u32, rec: u32) -> bool {
    copy_words(m, rec + 4, crate::globals::MAT_IDENTITY, 12);
    copy_words(m, rec + 0x4c, a, 3);
    copy_words(m, rec + 0x58, b, 3);
    m.set_u32(rec, 0);
    m.set_u32(rec + 0x74, 0);
    m.set_u32(rec + 0x68, 0);
    m.set_u32(rec + 0x78, owner);
    world.room_collide(m, 0, room, 4, rec) & 1 != 0
}

/// `0x4f1240`: put the vehicle at the transform `mat` in `room` and onto the ground below it
/// (`mode` 2: also tilt it to the ground; 1 and 2: then find the wheel contacts).
pub fn fn_004f1240(m: &mut Mem, world: &mut dyn World, this: u32, mode: i32, room: u32, mat: u32) {
    for o in [0xc90, 0xcd8, 0xc9c, 0xce4] {
        copy_words(m, this + o, ZERO3, 3);
    }
    let up = v3(m, UP);
    let (start, end, rec) = (SCRATCH + 0x700, SCRATCH + 0x710, SCRATCH + 0x720);
    let t = v3(m, mat + 0x24);
    let mut hit = false;
    for i in 1..=3i32 {
        let d = (i * i * i + 1) as f32 as f64;
        let s = [r(d * up[0]) as f64, r(d * up[1]) as f64, d * up[2]];
        set3(m, start, [r(s[0] + t[0]), r(s[1] + t[1]), r(s[2] + t[2])]);
        set3(m, end, [r(t[0] - s[0]), r(t[1] - s[1]), r(t[2] - s[2])]);
        rec_init(m, rec);
        let owner = m.u32(this + 0x29d4);
        if fn_004e6621(m, world, owner, room, start, end, rec) {
            hit = true;
            break;
        }
    }
    let h = r(0.75 * f(m, this + 0x1f9c) + f(m, this + 0x1b70) - f(m, this + 0x1cc4)) as f64;
    if hit {
        let (hp, nr) = (SCRATCH + 0x800, SCRATCH + 0x810);
        copy_words(m, hp, rec + 0x34, 3);
        copy_words(m, nr, rec + 0x40, 3);
        let place = SCRATCH + 0x820;
        copy_words(m, place, mat, 12);
        let inv = SCRATCH + 0x860;
        crate::math::mat_inverse(m, inv, mat);
        if mode == 2 {
            let h = r(f(m, this + 0x1b70) - f(m, this + 0x1cc4) + 0.75 * f(m, this + 0x1f9c)) as f64;
            let n = v3(m, nr);
            let hpv = v3(m, hp);
            let pos = [r(r(n[0] * h) as f64 + hpv[0]), r(r(n[1] * h) as f64 + hpv[1]), r(n[2] * h + hpv[2])];
            let l = get12(m, inv);
            let nx = l[3] * n[1] + l[6] * n[2] + l[0] * n[0];
            let ny = r(l[4] * n[1] + l[7] * n[2] + l[1] * n[0]) as f64;
            let nz = r(l[5] * n[1] + l[8] * n[2] + l[2] * n[0]) as f64;
            let axis = SCRATCH + 0x8a0;
            set3(m, axis, [r(nz * up[1] - ny * up[2]), r(nx * up[2] - nz * up[0]), r(ny * up[0] - nx * up[1])]);
            let len = crate::math::vec_len(m, axis) as f32;
            let look = SCRATCH + 0x8b0;
            let ax = [m.f32(axis), m.f32(axis + 4), m.f32(axis + 8)];
            let z = [m.f32(ZERO3), m.f32(ZERO3 + 4), m.f32(ZERO3 + 8)];
            crate::matrix::mat_look(m, look, z, ax, len);
            crate::matrix::mat_mul(m, place, look, mat);
            set3(m, place + 0x24, pos);
        }
        fn_004f0df0(m, this, room, place);
        copy_words(m, this + 0x8d0, this + 0x10, 12);
        copy_words(m, this + 0x930, this + 0x900, 12);
        copy_words(m, this + 0x1a10, this + 0xf4, 1);
        fn_004f1fa0(m, this, 1, hp, nr);
        if mode == 1 || mode == 2 {
            fn_00504d30(m, world, this);
        }
    } else {
        fn_004f0df0(m, this, room, mat);
        copy_words(m, this + 0x8d0, this + 0x10, 12);
        copy_words(m, this + 0x1a10, this + 0xf4, 1);
        copy_words(m, this + 0x930, this + 0x900, 12);
        let pos = v3(m, this + 0x34);
        let pt = SCRATCH + 0x8f0;
        set3(m, pt, [r(pos[0] - r(h * up[0]) as f64), r(pos[1] - r(h * up[1]) as f64), r(pos[2] - r(h * up[2]) as f64)]);
        fn_004f1fa0(m, this, 1, pt, UP);
    }
    fn_004f0800(m, this);
    fn_004f0c20(m, this);
}

/// `0x504d30`: find the ground under every wheel (`0x503e30`) and set the suspension from it;
/// returns the number of wheels on the ground.
pub fn fn_00504d30(m: &mut Mem, world: &mut dyn World, this: u32) -> i32 {
    copy_words(m, this + 0xc4, ZERO3, 3);
    m.set_u32(this + 0x26b0, 0);
    let up = v3(m, UP);
    let mut count = 0i32;
    let centres = SCRATCH + 0x900;
    for k in 0..m.i32(this + 0xdd4).max(0) as u32 {
        copy_words(m, this + 0xf08 + 12 * k, ZERO3, 3);
        let d = f(m, this + 0x1b74 + 4 * k);
        let mo = v3(m, this + 0x1cbc + 12 * k);
        let p = [r(mo[0] - r(up[0] * d) as f64) as f64, r(mo[1] - r(up[1] * d) as f64) as f64, mo[2] - up[2] * d];
        let g = get12(m, this + 0x10);
        let c = centres + 12 * k;
        set3(
            m,
            c,
            [
                r(p[2] * g[6] + p[1] * g[3] + p[0] * g[0] + g[9]),
                r(p[2] * g[7] + p[1] * g[4] + p[0] * g[1] + g[10]),
                r(p[2] * g[8] + p[1] * g[5] + p[0] * g[2] + g[11]),
            ],
        );
        let res = crate::contact::fn_00503e30(m, world, this, 0, c, 0, k);
        if m.u32(this + 0x3c54) != 0 {
            return 0;
        }
        let flag = this + 0x19a4 + 2 * k;
        let w = this + 0xf8 * (k + 1);
        if res == 1 {
            let s = f(m, this + 0xe3c + 4 * k);
            let wp = v3(m, w);
            let wn = v3(m, w + 0xc);
            set3(m, this + 0x64 + 12 * k, [r(s * wn[0] + wp[0]), r(s * wn[1] + wp[1]), r(s * wn[2] + wp[2])]);
            crate::matrix::vec_add_in_place(m, this + 0xc4, SCRATCH + 0x9a0, w + 0xc);
            count += 1;
            m.set_f32(this + 0x26b0, r(f(m, this + 0x26b0) + f(m, this + 0x2670 + 4 * k)));
            let q = v3(m, this + 0x64 + 12 * k);
            let iv = get12(m, this + 0x900);
            let lz = r(iv[2] * q[0] + iv[5] * q[1] + iv[8] * q[2] + iv[11]);
            set3(
                m,
                this + 0x1844 + 12 * k,
                [r(iv[0] * q[0] + iv[3] * q[1] + iv[6] * q[2] + iv[9]), r(iv[1] * q[0] + iv[4] * q[1] + iv[7] * q[2] + iv[10]), lz],
            );
            let v = r(mo[2] - d - lz as f64);
            m.set_f32(this + 0xe18 + 4 * k, v);
            m.set_f32(this + 0xdd8 + 4 * k, v);
            if m.f32(this + 0x1f9c + 4 * k) < v {
                copy_words(m, this + 0xdd8 + 4 * k, this + 0x1f9c + 4 * k, 1);
                m.set_i16(flag, 1);
            } else {
                if v < 0.0 {
                    m.set_u32(this + 0xdd8 + 4 * k, 0);
                }
                if m.f32(this + 0xd20 + 4 * k) < 0.001f64 as f32 && m.i16(flag) == 0 {
                    m.set_f32(this + 0xd20 + 4 * k, r((f(m, w + 0x14)).abs() * f(m, this + 0x2640) * 2.45));
                }
                m.set_i16(flag, 0);
            }
        } else {
            let b = [up[0] * 1000.0, up[1] * 1000.0, up[2] * 1000.0];
            let cv = v3(m, c);
            let rx = r(b[2] * g[6] + b[1] * g[3] + b[0] * g[0]) as f64;
            let ry = r(b[1] * g[4] + b[2] * g[7] + b[0] * g[1]) as f64;
            let rz = b[1] * g[5] + b[2] * g[8] + b[0] * g[2];
            set3(m, this + 0x64 + 12 * k, [r(cv[0] - rx), r(cv[1] - ry), r(cv[2] - rz)]);
            let (b1, b2) = (r(b[1]) as f64, r(b[2]) as f64);
            set3(m, this + 0x1844 + 12 * k, [r(mo[0] - b[0]), r(mo[1] - b1), r(mo[2] - b2)]);
            m.set_f32(this + 0xe18 + 4 * k, 1000.0);
            copy_words(m, this + 0xdd8 + 4 * k, this + 0x1f9c + 4 * k, 1);
            m.set_i16(flag, 1);
            m.set_u32(this + 0xd20 + 4 * k, 0);
        }
    }
    let e = |m: &Mem, k: u32| m.f32(this + 0xe18 + 4 * k);
    let a = if e(m, 2) < e(m, 3) { e(m, 3) } else { e(m, 2) };
    let b = if e(m, 0) < e(m, 1) { e(m, 1) } else { e(m, 0) };
    m.set_f32(this + 0xe38, if b < a { a } else { b });
    if count == 0 {
        copy_words(m, this + 0xc4, UP, 3);
    } else {
        crate::math::vec_normalize(m, this + 0xc4);
        m.set_f32(this + 0x26b0, r(f(m, this + 0x26b0) / count as f32 as f64));
    }
    fn_00504930(m, this);
    // 0x506a60: which wheel pairs have a common ground
    let mut all = true;
    for (i, o) in [0x8bcu32, 0x8c0, 0x8c4, 0x8c8].iter().enumerate() {
        let v = crate::contact::fn_00506750(m, this, i as u32);
        m.set_u32(this + o, v);
        all &= v != 0;
    }
    m.set_u32(this + 0x8b8, all as u32);
    let s = |m: &Mem, k: u32| m.i16(this + 0x19a4 + 2 * k) != 0;
    let n = m.i32(this + 0xdd4);
    let b7;
    let b8;
    if n == 6 {
        b7 = s(m, 2) && s(m, 3) && s(m, 4) && s(m, 5);
        m.wr8(this + 0x19b7, b7 as u8);
        m.wr8(this + 0x19b5, (s(m, 0) && s(m, 1)) as u8);
        m.wr8(this + 0x19b6, (s(m, 0) && s(m, 2) && s(m, 4)) as u8);
        b8 = s(m, 1) && s(m, 3) && s(m, 5);
    } else {
        b7 = s(m, 2) && s(m, 3);
        m.wr8(this + 0x19b7, b7 as u8);
        m.wr8(this + 0x19b5, (s(m, 0) && s(m, 1)) as u8);
        m.wr8(this + 0x19b6, (s(m, 0) && s(m, 2)) as u8);
        b8 = s(m, 1) && s(m, 3);
    }
    m.wr8(this + 0x19b8, b8 as u8);
    m.wr8(this + 0x19b4, 1);
    let any = m.rd8(this + 0x19b5) != 0 || m.rd8(this + 0x19b6) != 0 || m.rd8(this + 0x19b7) != 0 || m.rd8(this + 0x19b8) != 0;
    m.wr8(this + 0x19b9, any as u8);
    m.wr8(this + 0x19ba, 0);
    for k in 0..n.max(0) as u32 {
        let v = m.rd8(this + 0x19ba) != 0 || s(m, k);
        m.wr8(this + 0x19ba, v as u8);
        let v = m.rd8(this + 0x19b4) != 0 && s(m, k);
        m.wr8(this + 0x19b4, v as u8);
    }
    crate::contact::fn_00506210(m, this, 1);
    count
}

/// `0x504930`: the hitch / ground load of the body: weight force (`+0xd40`) and damping of the
/// velocities when the body is pushed into soft ground (surface kinds 4 and 7).
pub fn fn_00504930(m: &mut Mem, this: u32) {
    let n = m.i32(this + 0xdd4);
    let mut k = 0;
    while k < n {
        let s = m.u32(this + 0x17c + 0xf8 * k as u32);
        if s != 0 && m.i32(s) == 1 && (m.i32(s + 4) == 7 || m.i32(s + 4) == 4) {
            break;
        }
        k += 1;
    }
    let up = v3(m, UP);
    let h = 0.75 * f(m, this + 0x1f9c) + f(m, this + 0x1b70);
    let (hx, hy, hz) = (r(up[0] * h) as f64, r(up[1] * h) as f64, up[2] * h);
    let g = |m: &Mem, o: u32| f(m, this + o);
    let xs = g(m, 0x1cd4) + g(m, 0x1cbc);
    let ys = r(g(m, 0x1cc0) + g(m, 0x1cd8)) as f64;
    let zs = r(g(m, 0x1cc4) + g(m, 0x1cdc)) as f64;
    let cx = r(xs * 0.5 - hx) as f64;
    let cy = r(r(ys * 0.5) as f64 - hy) as f64;
    let cz = r(zs * 0.5) as f64 - hz;
    let w = r(cz * g(m, 0x30) + cy * g(m, 0x24) + cx * g(m, 0x18) + g(m, 0x3c));
    let nu = [r(-up[0]) as f64, r(-up[1]) as f64, r(-up[2]) as f64];
    let gv = [r(nu[0] * 9.80000019f32 as f64) as f64, r(nu[1] * 9.80000019f32 as f64) as f64, r(nu[2] * 9.80000019f32 as f64) as f64];
    if k != n && w < 0.0 {
        let a = w.abs();
        let mut s = r(a as f64 * 0.5);
        if s < 0.0 {
            s = 0.0;
        } else if s > 1.0 {
            s = 1.0;
        }
        let t = 1.0 - s as f64 * 0.8;
        let v = [r(gv[0] * t), r(gv[1] * t), r(gv[2] * t)];
        set3(m, this + 0xd40, v);
        m.set_u32(this + 0x3c4c, 1);
        set3(m, this + 0x3310, v);
        m.set_f32(this + 0x3c48, a);
        if m.i16(this + 0x2740) == 1 && 3.0 < a as f64 {
            m.set_u32(this + 0x3344, 1);
        }
        let k1 = r(-(g(m, 0xdc0) * g(m, 0xcf8) * s as f64)) as f64;
        let vel = v3(m, this + 0x40);
        set3(m, this + 0xc90, [r(k1 * vel[0]), r(k1 * vel[1]), r(k1 * vel[2])]);
        let k2 = r(-(s as f64 * g(m, 0xcfc))) as f64;
        let om = v3(m, this + 0x58);
        set3(m, this + 0xcd8, [r(k2 * om[0]), r(k2 * om[1]), r(k2 * om[2])]);
    } else {
        let v = [r(gv[0]), r(gv[1]), r(gv[2])];
        m.set_u32(this + 0x3c48, 0);
        set3(m, this + 0xd40, v);
        m.set_u32(this + 0x3c4c, 0);
        set3(m, this + 0x3310, v);
        let vel = v3(m, this + 0x40);
        set3(m, this + 0xc90, [r(0.0 * vel[0]), r(0.0 * vel[1]), r(0.0 * vel[2])]);
        let om = v3(m, this + 0x58);
        set3(m, this + 0xcd8, [r(0.0 * om[0]), r(0.0 * om[1]), r(0.0 * om[2])]);
    }
}

/// `0x4f7380`: set up the body integrator (`+0x2244..`): the frame about the centre of mass, the
/// number of substeps for the frame time, the ground and air terms.
pub fn fn_004f7380(m: &mut Mem, this: u32, full: i32) {
    let p = |o: u32| this + o;
    let c = v3(m, p(0x17e4));
    crate::matrix::mat_translation(m, p(0x2244), c[0] as f32, c[1] as f32, c[2] as f32);
    crate::math::mat_inverse(m, SCRATCH + 0xa00, p(0x2244));
    copy_words(m, p(0x2274), SCRATCH + 0xa00, 12);
    crate::matrix::mat_mul(m, SCRATCH + 0xa00, p(0x8d0), p(0x2274));
    copy_words(m, p(0x22a4), SCRATCH + 0xa00, 12);
    crate::matrix::mat_mul(m, SCRATCH + 0xa00, p(0x8d0), p(0x2274));
    copy_words(m, p(0x22d4), SCRATCH + 0xa00, 12);
    crate::math::mat_inverse(m, SCRATCH + 0xa00, p(0x22a4));
    copy_words(m, p(0x2304), SCRATCH + 0xa00, 12);
    copy_words(m, p(0x2334), p(0xa08), 3);
    copy_words(m, p(0x2340), p(0xc3c), 3);
    let g = |m: &Mem, o: u32| f(m, this + o);
    let l = v3(m, p(0xc3c));
    let lb = [
        r(l[0] * g(m, 0x2304) + g(m, 0x2310) * l[1] + g(m, 0x231c) * l[2]),
        r(g(m, 0x2308) * l[0] + g(m, 0x2314) * l[1] + g(m, 0x2320) * l[2]),
        r(g(m, 0x230c) * l[0] + g(m, 0x2318) * l[1] + g(m, 0x2324) * l[2]),
    ];
    set3(m, p(0x234c), lb);
    crate::math::vec_div(m, p(0x2358), lb, p(0x1f48));
    for o in [0x2388, 0x23a0, 0x23ac] {
        copy_words(m, p(o), ZERO3, 3);
    }
    copy_words(m, p(0x2394), p(0xc6c), 3);
    copy_words(m, p(0x23d0), p(0xcc0), 3);
    copy_words(m, p(0x23dc), ZERO3, 3);
    copy_words(m, p(0x23e8), ZERO3, 3);
    let n = m.i32(p(0xdd4)).max(0) as u32;
    for k in 0..n {
        copy_words(m, p(0x241c + 4 * k), p(0xdf8 + 4 * k), 1);
    }
    let t = get12(m, p(0x2274));
    let tr = |q: [f64; 3]| {
        [
            r(t[0] * q[0] + t[3] * q[1] + t[6] * q[2] + t[9]),
            r(t[1] * q[0] + t[4] * q[1] + t[7] * q[2] + t[10]),
            r(t[2] * q[0] + t[5] * q[1] + t[8] * q[2] + t[11]),
        ]
    };
    for k in 0..n {
        let a = tr(v3(m, p(0x64 + 12 * k)));
        set3(m, p(0x24bc + 12 * k), a);
        let b = tr(v3(m, p(0x17e4 + 12 * k)));
        set3(m, p(0x245c + 12 * k), b);
    }
    let dt = crate::globals::dt_hi(m);
    m.set_f32(p(0x251c), dt as f32);
    let mut steps = ((dt * 33.333333333333336).floor() + 1.0) as i32;
    m.set_i32(p(0x2530), steps);
    if (m.i16(p(0x2740)) != 0 && m.f32(p(0x2640)) < 2500.0) || m.i32(p(0x276c)) != 0 {
        steps = ((g(m, 0x251c) * 100.0).floor() + 1.0) as i32;
        m.set_i32(p(0x2530), steps);
    }
    m.set_f32(p(0x252c), steps as f32);
    let sub = r(g(m, 0x251c) / steps as f32 as f64);
    m.set_f32(p(0x2524), sub);
    m.set_f32(p(0x2528), sub);
    let s = |m: &Mem, o: u32| m.i16(this + o) != 0;
    if full == 0 {
        let whole = m.f32(p(0x251c));
        m.set_i32(p(0x2530), 1);
        m.set_f32(p(0x2528), whole);
        let wn = m.i32(p(0xdd4));
        if wn == 4 && ((s(m, 0x19a4) && s(m, 0x19a8)) || (s(m, 0x19a6) && s(m, 0x19aa))) {
            m.set_f32(p(0x2524), whole);
        }
        if wn == 2 && (s(m, 0x19a4) || s(m, 0x19a6)) {
            m.set_f32(p(0x2524), whole);
        }
    }
    m.set_u32(p(0x2520), 0);
    if g(m, 0xdc4) < 0.1 && m.i32(p(0x1abc)) != 0 {
        let e = v3(m, p(0x2334));
        let nn = v3(m, p(0xd0));
        let f2 = r(nn[1] * e[2] - e[1] * nn[2]) as f64;
        let f4 = r(e[0] * nn[2] - nn[0] * e[2]) as f64;
        let f3 = r(e[1] * nn[0] - nn[1] * e[0]) as f64;
        let v = SCRATCH + 0xa40;
        set3(m, v, [r(f4 * nn[2] - f3 * nn[1]), r(f3 * nn[0] - f2 * nn[2]), r(f2 * nn[1] - f4 * nn[0])]);
        let a = crate::math::vec_len(m, v);
        m.set_f32(p(0xdc4), a as f32);
        let b = crate::math::vec_len(m, p(0x2334));
        m.set_f32(p(0x2418), b as f32);
    }
    let nn = v3(m, p(0xd0));
    set3(
        m,
        p(0x2598),
        [
            r(nn[0] * g(m, 0x2304) + g(m, 0x2310) * nn[1] + g(m, 0x231c) * nn[2]),
            r(g(m, 0x2308) * nn[0] + g(m, 0x2314) * nn[1] + g(m, 0x2320) * nn[2]),
            r(g(m, 0x230c) * nn[0] + g(m, 0x2318) * nn[1] + g(m, 0x2324) * nn[2]),
        ],
    );
    let up = v3(m, UP);
    let q = v3(m, p(0xc4));
    let f2 = up[2] * q[1] - up[1] * q[2];
    let f3 = up[0] * q[2] - up[2] * q[0];
    let f4 = up[1] * q[0] - up[0] * q[1];
    set3(m, p(0x25a4), [r(f3 * q[2] - f4 * q[1]), r(f4 * q[0] - f2 * q[2]), r(f2 * q[1] - f3 * q[0])]);
    copy_words(m, p(0x25b0), ZERO3, 3);
    copy_words(m, p(0x25d4), p(0xc54), 3);
    let l = crate::math::vec_len(m, p(0x25a4));
    m.set_f32(p(0x25e0), l as f32);
    if 0.0001 < l {
        let a = v3(m, p(0x25a4));
        set3(m, p(0x25b0), [r(a[0] / l), r(a[1] / l), r(a[2] / l)]);
    }
    identity(m, p(0x25e4));
    let wn = m.i32(p(0xdd4));
    let mut free = false;
    for k in 0..wn.max(0) as u32 {
        if free || m.i16(p(0x19a4 + 2 * k)) == 0 {
            free = true;
        }
    }
    let kk = if wn > 0 && free { g(m, 0x2644) * g(m, 0xdc4) * 0.033 } else { 0.0 };
    let kf = if kk > 1.0 { 1.0f32 } else { kk as f32 };
    let dv = SCRATCH + 0xa60;
    let vel = v3(m, p(0x40));
    let tgt = v3(m, p(0x28f4));
    let d = [r(tgt[0] - vel[0]), r(tgt[1] - vel[1]), r(tgt[2] - vel[2])];
    set3(m, dv, d);
    set3(m, dv + 12, d);
    let len = r(crate::math::vec_len(m, dv));
    if len != 0.0 {
        crate::matrix::vec_div_in_place(m, dv + 12, SCRATCH + 0xa80, len);
    }
    let dn = v3(m, dv + 12);
    let proj = r((g(m, 0xa38) * dn[0] + g(m, 0xa3c) * dn[1] + g(m, 0xa40) * dn[2]).abs()) as f64;
    let nu = [-up[0], -up[1], -up[2]];
    let ax = r(nu[2] * g(m, 0x22bc) + nu[1] * g(m, 0x22b0) + nu[0] * g(m, 0x22a4)) as f64;
    let ay = r(nu[2] * g(m, 0x22c0) + nu[1] * g(m, 0x22b4) + nu[0] * g(m, 0x22a8)) as f64;
    let az = nu[2] * g(m, 0x22c4) + nu[1] * g(m, 0x22b8) + nu[0] * g(m, 0x22ac);
    let cc = kf as f64 * g(m, 0x2640) * 10.0;
    let b = [r(ax * cc) as f64, r(ay * cc) as f64, r(az * cc) as f64];
    let lf = len as f64;
    let v1 = [r(d[0] as f64 * lf) as f64, r(d[1] as f64 * lf) as f64, r(d[2] as f64 * lf) as f64];
    let sc = proj * g(m, 0x2648) + (1.0 - proj) * g(m, 0x264c);
    let fv = [r(v1[0] * sc), r(v1[1] * sc), r(v1[2] * sc)];
    set3(m, p(0x2614), [r(fv[0] as f64 + b[0]), r(fv[1] as f64 + b[1]), r(fv[2] as f64 + b[2])]);
    let (fa, rb) = (SCRATCH + 0xa90, SCRATCH + 0xaa0);
    set3(m, fa, fv);
    crate::matrix::rotate(m, rb, fa, p(0x2304));
    crate::matrix::vec_cross(m, p(0x2620), p(0x26b4), rb);
    m.set_u32(p(0x2534), 1);
}

/// The member constructors of `_Car_V` (the start of `0x4ebd00`): the fields they clear and the
/// identity transforms, wheel records, filters, sub-objects and technical systems.
fn members(m: &mut Mem, this: u32) {
    let p = |i: u32| this + 4 * i;
    let span = |m: &mut Mem, a: u32, b: u32| zero(m, p(a), b - a);
    identity(m, p(4));
    span(m, 0x10, 0x19);
    span(m, 0x19, 0x3d);
    for k in 0..8u32 {
        wheel_init(m, this + 0xf8 * (k + 1));
    }
    for i in [0x234, 0x240, 0x24c, 600, 0x264, 0x270] {
        identity(m, p(i));
    }
    span(m, 0x27c, 0x309);
    span(m, 0x309, 0x33c);
    span(m, 0x350, 0x35f);
    span(m, 0x360, 0x36f);
    span(m, 0x3c2, 0x3da);
    span(m, 0x402, 0x405);
    for k in 0..8u32 {
        wheel_init(m, this + 0x1018 + 0xf8 * k);
    }
    span(m, 0x5f9, 0x659);
    span(m, 0x67f, 0x682);
    span(m, 0x6b0, 0x6b9);
    identity(m, p(0x6ba));
    crate::filter::new(m, this + 0x1b30, crate::filter::WEIGHT_QUADRATIC, 0.5, 0, 0.0);
    crate::filter::new(m, this + 0x1b50, crate::filter::WEIGHT_COSINE, f32::from_bits(0x3f33_3333), 0, 0.0);
    span(m, 0x72f, 0x747);
    span(m, 0x7aa, 0x7c2);
    span(m, 0x7d2, 0x7d8);
    zero(m, this + 0x1ffc, 2);
    identity(m, p(0x801));
    span(m, 0x80d, 0x888);
    fn_004f6cf0(m, this + 0x2224);
    span(m, 0x9ad, 0x9b0);
    fn_004eb770(m, this + 0x2700);
    identity(m, this + 0x29f0);
    identity(m, this + 0x2a20);
    zero(m, this + 0x2a50, 6);
    zero(m, this + 0x2a68, 6);
    zero(m, this + 0x2a80, 48);
    zero(m, this + 0x2b40, 3);
    for k in 0..8u32 {
        wheel_init(m, this + 0x2b50 + 0xf8 * k);
    }
    zero(m, this + 0x3310, 3);
    zero(m, this + 0x33ac, 6);
    zero(m, this + 0x33c4, 24);
    zero(m, this + 0x34b8, 3);
    for k in 0..8u32 {
        rec_init(m, this + 0x3510 + 0xd0 * k);
    }
    zero(m, this + 0x3c1c, 6);
    zero(m, this + 0x3c5c, 6);
    for k in 0..7u32 {
        ts::reset(m, this + 0x3ebc + 0x34 * k);
    }
}

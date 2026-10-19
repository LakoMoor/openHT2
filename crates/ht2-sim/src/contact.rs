//! Wheel and body contact with the world (`car*.cpp`, `0x5039a0..0x506750`).
//!
//! The original asks the scene graph for collisions through a virtual method (`+0x34` of every
//! node class) that takes a mode and a 0xd0-byte query record. That call is the boundary to the
//! world: [`World::collide`] stands for it, so the vehicle code here stays the original while the
//! world can be the ported collision engine or a stand-in.
//!
//! Query record (52 dwords):
//! ```text
//! [0]        flags             [1..13]   transform into the node space (identity at the top)
//! [0xd..0x10] hit point        [0x10..0x13] hit normal
//! [0x13..0x16] segment start   [0x16..0x19] segment end
//! [0x19]     surface record    [0x1a] room / object hit   [0x1b] collision volume
//! [0x1c]     hit kind          [0x1d..]  more results     [0x30..0x34] statistics
//! ```

use crate::mem::Mem;

/// Scratch memory standing in for the stack frames of the functions here.
const SCRATCH: u32 = 0x2ff9_0000;

/// The scene-graph collision query of the original (`(*obj->vtbl[0x34/4])(mode, rec)`).
pub trait World {
    /// Collide the query `rec` against the node `obj`; returns the original's result bits
    /// (1 = hit, 2 = crossed into another room / object).
    fn collide(&mut self, m: &mut Mem, obj: u32, mode: u32, rec: u32) -> u32;

    /// The room-based queries used when `+0x2708` is set: `which` 0 is `0x4e6059` (room
    /// collision node, then its neighbours), 1 is `0x4e623e` (objects of the room).
    fn room_collide(&mut self, m: &mut Mem, which: u32, room: u32, mode: u32, rec: u32) -> u32 {
        let _ = which;
        self.collide(m, room, mode, rec)
    }
}

/// A world with nothing in it.
pub struct EmptyWorld;

impl World for EmptyWorld {
    fn collide(&mut self, _m: &mut Mem, _obj: u32, _mode: u32, _rec: u32) -> u32 {
        0
    }
}

fn copy_words(m: &mut Mem, dst: u32, src: u32, n: u32) {
    for i in 0..n {
        let w = m.u32(src + 4 * i);
        m.set_u32(dst + 4 * i, w);
    }
}

/// `0x5ef5c0`: initialise a query record.
pub fn rec_init(m: &mut Mem, rec: u32) {
    for i in 1..13 {
        m.set_u32(rec + 4 * i, 0);
    }
    copy_words(m, rec + 4, crate::globals::MAT_IDENTITY, 12);
    for i in 0xd..0x19 {
        m.set_u32(rec + 4 * i, 0);
    }
    for i in [0x30, 0, 0x19, 0x1a, 0x1b, 0x1e, 0x1d, 0x31, 0x33, 0x32] {
        m.set_u32(rec + 4 * i, 0);
    }
}

/// `0x5039a0`: move the point `pt` from the previous position (`+0x8f4`) through the world and
/// find the room it ends up in (written to `room_out`). Returns 0 when the way is blocked (the
/// point is moved back to the hit), 1 otherwise.
pub fn fn_005039a0(m: &mut Mem, world: &mut dyn World, this: u32, pt: u32, room_out: u32) -> u32 {
    let rec = SCRATCH;
    rec_init(m, rec);
    copy_words(m, rec + 4, crate::globals::MAT_IDENTITY, 12);
    copy_words(m, rec + 0x4c, this + 0x8f4, 3);
    copy_words(m, rec + 0x58, pt, 3);
    m.set_u32(rec + 0x74, 0);
    m.set_u32(rec, 0);
    let obj = m.u32(this + 0x1a10);
    if obj != 0 {
        let flags = world.collide(m, obj, 0, rec);
        let crossed = flags & 2;
        if crossed != 0 && m.u32(rec + 0x68) == 0 {
            m.set_u32(this + 0x3c54, 1);
            m.set_u32(rec + 0x68, obj);
        }
        if flags & 1 == 1 {
            let o = if crossed == 2 { m.u32(rec + 0x68) } else { obj };
            m.set_u32(room_out, o);
            copy_words(m, pt, rec + 0x34, 3);
            return 0;
        }
        if crossed == 2 {
            let o = m.u32(rec + 0x68);
            m.set_u32(room_out, o);
            return 1;
        }
    }
    m.set_u32(room_out, obj);
    1
}

/// `0x503ac0`: contact for a wheel without its own collision: the average contact point and
/// normal of the wheels that have one, with the segment of `rec` cut there.
pub fn fn_00503ac0(m: &mut Mem, this: u32, rec: u32) -> u32 {
    let count = m.i32(this + 0xdd4).max(0) as u32;
    let mut idx = Vec::new();
    for i in 0..count {
        let w = this + 0xf8 * (i + 1);
        if m.u32(this + 0x1bf4 + 4 * i) != 0 && m.i16(w + 0x18) != 0 {
            idx.push(i);
        }
    }
    if idx.is_empty() {
        return 0;
    }
    let (p, n) = (SCRATCH + 0x100, SCRATCH + 0x110);
    let get = |m: &Mem, a: u32| [m.f32(a), m.f32(a + 4), m.f32(a + 8)];
    let mut sp = [m.f32(0x702650), m.f32(0x702654), m.f32(0x702658)];
    let mut sn = sp;
    if idx.len() == 1 {
        let w = this + 0xf8 * (idx[0] + 1);
        sp = get(m, w);
        sn = get(m, w + 0xc);
    } else {
        for &i in &idx {
            let w = this + 0xf8 * (i + 1);
            let (a, b) = (get(m, w), get(m, w + 0xc));
            for k in 0..3 {
                sp[k] = (sp[k] as f64 + a[k] as f64) as f32;
                sn[k] = (sn[k] as f64 + b[k] as f64) as f32;
            }
        }
    }
    for k in 0..3u32 {
        m.set_f32(p + 4 * k, sp[k as usize]);
        m.set_f32(n + 4 * k, sn[k as usize]);
    }
    if idx.len() > 1 {
        let c = idx.len() as f32;
        let tmp = SCRATCH + 0x120;
        crate::matrix::vec_div_in_place(m, p, tmp, c);
        crate::matrix::vec_div_in_place(m, n, tmp, c);
    }
    let w0 = this + 0xf8 * (idx[0] + 1);
    if m.u32(w0 + 0x84) != 0 {
        copy_words(m, rec, w0 + 0x20, 0x34);
    } else {
        m.set_u32(rec + 0x64, 0);
        m.set_u32(rec + 0x6c, 0);
        m.set_u32(rec, 0);
        copy_words(m, rec + 4, crate::globals::MAT_IDENTITY, 12);
        m.set_u32(rec + 0x70, 0);
        m.set_u32(rec + 0x74, 0);
        m.set_u32(rec + 0x78, 0);
        m.wr8(rec + 0x7c, 0);
    }
    let hit = SCRATCH + 0x130;
    crate::matrix::line_plane(m, hit, p, n, rec + 0x4c, rec + 0x58);
    copy_words(m, rec + 0x34, hit, 3);
    copy_words(m, rec + 0x40, n, 3);
    1
}

/// The world used by the golden tests (mirrors `tools/emu/world_stub.py`): a flat ground at the
/// height stored in the object. Object layout: `+8` f32 height, `+0xc` surface record, `+0x10`
/// room, `+0x14` extra result flags (bit 2: crossed into the room at `+0x10`).
pub struct StubWorld;

impl World for StubWorld {
    fn collide(&mut self, m: &mut Mem, obj: u32, _mode: u32, rec: u32) -> u32 {
        let f = |x: f64| x as f32 as f64;
        let z0 = m.f32(obj + 8) as f64;
        let s: Vec<f64> = (0..3).map(|i| m.f32(rec + 0x4c + 4 * i) as f64).collect();
        let t: Vec<f64> = (0..3).map(|i| m.f32(rec + 0x58 + 4 * i) as f64).collect();
        let extra = m.u32(obj + 0x14) & 2;
        if extra != 0 {
            let room = m.u32(obj + 0x10);
            m.set_u32(rec + 0x68, room);
        }
        if t[2] < z0 && z0 <= s[2] {
            let k = f((s[2] - z0) / (s[2] - t[2]));
            let mut hit: Vec<f64> = (0..3).map(|i| f(s[i] + f(f(t[i] - s[i]) * k))).collect();
            hit[2] = z0;
            for i in 0..3 {
                m.set_f32(rec + 0x34 + 4 * i as u32, hit[i] as f32);
            }
            m.set_f32(rec + 0x40, 0.0);
            m.set_f32(rec + 0x44, 0.0);
            m.set_f32(rec + 0x48, 1.0);
            let surf = m.u32(obj + 0xc);
            m.set_u32(rec + 0x64, surf);
            return 1 | extra;
        }
        extra
    }
}

/// The default surface record the original uses when a room query hits nothing specific.
pub const DEFAULT_SURFACE: u32 = 0x697678;

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

/// `0x503e30`: find the ground under wheel `wheel` whose centre is at `centre`, by casting
/// segments along the body's up axis (longer on each of three tries) into the room `*room`
/// (looked up first when 0). Fills the wheel's contact (`W = this + 0xf8 * (wheel + 1)`:
/// point `W+0`, normal `W+0xc`, flags `W+0x18`/`W+0x1a`, record copy `W+0x20`, room `W+0xf0`),
/// and unless `no_surface` the surface response ([`fn_00502780`], [`fn_00503500`]).
/// Returns 1 on contact, 0 without, 2 when the wheel is inside a wall.
pub fn fn_00503e30(m: &mut Mem, world: &mut dyn World, this: u32, no_surface: u32, centre: u32, room: u32, wheel: u32) -> u32 {
    let rec = SCRATCH + 0x200;
    let p = v3(m, centre);
    rec_init(m, rec);
    let w = this + 0xf8 * (wheel + 1);
    let c = |m: &Mem, k: u32| f(m, 0x702630 + 4 * k);
    let col = |m: &Mem, o: u32| r(c(m, 2) * f(m, this + o + 0x18) + c(m, 1) * f(m, this + o + 0xc) + c(m, 0) * f(m, this + o));
    let u = [col(m, 0x10) as f64, col(m, 0x14) as f64, col(m, 0x18) as f64];
    let q = SCRATCH + 0x2e0;
    set3(m, q, [r(p[0] + c(m, 0)), r(p[1] + c(m, 1)), r(p[2] + c(m, 2))]);
    let room_cell = SCRATCH + 0x2f0;
    m.set_u32(room_cell, room);
    if room == 0 && fn_005039a0(m, world, this, q, room_cell) == 0 {
        m.set_u32(w + 0xf0, 0);
        m.set_i16(w + 0x18, 0);
        set3(m, w, [p[0] as f32, p[1] as f32, p[2] as f32]);
        m.set_u32(w + 0x84, 0);
        m.set_i16(w + 0x1a, 0);
        set3(m, w + 0xc, [p[0] as f32, p[1] as f32, p[2] as f32]);
        return 2;
    }
    if m.u32(this + 0x3c54) != 0 {
        return 0;
    }
    let room = m.u32(room_cell);
    let mut tries = 1i32;
    loop {
        copy_words(m, rec + 4, crate::globals::MAT_IDENTITY, 12);
        let dt = crate::globals::dt_hi(m);
        let len = r((dt * f(m, this + 0xdc0).sqrt() + f(m, this + 0x1b74 + 4 * wheel)) * (tries * tries) as f64 + 1.0) as f64;
        m.set_u32(rec + 0x74, 0);
        m.set_u32(rec, 0);
        let a = u[0] * len;
        let b = r(u[1] * len) as f64;
        let cz = r(u[2] * len) as f64;
        set3(m, rec + 0x4c, [r(a + p[0]), r(b + p[1]), r(cz + p[2])]);
        set3(m, rec + 0x58, [r(p[0] - a), r(p[1] - b), r(p[2] - cz)]);
        let k = m.u32(this + 0x29d4);
        m.set_u32(rec + 0x78, k);
        m.set_u32(rec + 0x68, if m.u32(this + 0x275c) != 0 { room } else { 0 });
        let res = if m.u32(this + 0x1bf4 + 4 * wheel) == 0 {
            fn_00503ac0(m, this, rec)
        } else if m.i16(this + 0x2708) == 0 {
            world.collide(m, room, 4, rec)
        } else {
            let res = world.room_collide(m, 0, room, 4, rec);
            if res & 1 != 0 {
                res
            } else {
                let res = world.room_collide(m, 1, room, 4, rec);
                if res & 1 != 0 && m.u32(rec + 0x64) == 0 {
                    m.set_u32(rec + 0x64, DEFAULT_SURFACE);
                }
                res
            }
        };
        if res & 1 == 1 {
            break;
        }
        tries += 1;
        if tries > 3 {
            if m.i16(w + 0x18) == 0 || m.i16(w + 0x1a) != 0 {
                m.set_u32(w + 0xf0, 0);
                set3(m, w, [0.0; 3]);
                copy_words(m, w + 0xc, 0x702650, 3);
            } else {
                // keep the contact plane of the last step if the wheel is still close to it
                let (s2, e2, h) = (SCRATCH + 0x300, SCRATCH + 0x310, SCRATCH + 0x320);
                set3(m, s2, [r(u[0] + p[0]), r(p[1] + u[1]), r(p[2] + u[2])]);
                set3(m, e2, [r(p[0] - u[0]), r(p[1] - u[1]), r(p[2] - u[2])]);
                crate::matrix::line_plane(m, h, w, w + 0xc, s2, e2);
                let hv = v3(m, h);
                let d = [r(p[0] - hv[0]) as f64, r(p[1] - hv[1]) as f64, r(p[2] - hv[2]) as f64];
                let dist = (d[2] * d[2] + d[1] * d[1] + d[0] * d[0]).sqrt();
                let nv = v3(m, w + 0xc);
                let along = nv[2] * d[2] + nv[1] * d[1] + nv[0] * d[0];
                let up = nv[2] * u[2] + nv[1] * u[1] + nv[0] * u[0];
                let reach = r(f(m, this + 0x1f9c + 4 * wheel) + f(m, this + 0x1b74 + 4 * wheel)) as f64;
                if along > 0.0 && up > 0.0 && dist < reach {
                    m.set_u32(w + 0xf0, room);
                    copy_words(m, w, h, 3);
                    m.set_i16(w + 0x18, 1);
                    m.set_i16(w + 0x1a, 1);
                    return 1;
                }
                m.set_u32(w + 0xf0, 0);
            }
            m.set_i16(w + 0x18, 0);
            m.set_i16(w + 0x1a, 0);
            m.set_u32(w + 0x84, 0);
            return 0;
        }
    }
    if m.i16(w + 0x18) == 0 && m.i16(w + 0x1a) == 0 {
        m.set_i16(w + 0x1a, 1);
        return 0;
    }
    m.set_u32(w + 0xf0, room);
    copy_words(m, w + 0x20, rec, 0x34);
    copy_words(m, w, rec + 0x34, 3);
    m.set_i16(w + 0x18, 1);
    m.set_i16(w + 0x1a, 0);
    copy_words(m, w + 0xc, rec + 0x40, 3);
    if no_surface == 0 {
        if wheel <= 5 {
            let tech = crate::gearbox::TECH_TABLE + (m.i32(this + 0x8cc) as u32).wrapping_mul(0x970);
            let off = if wheel < 2 { 0xb8 } else { 0xbc };
            let half = r(f(m, tech + off) * 0.5) as f64;
            let k = |m: &Mem, i: u32| f(m, 0x702640 + 4 * i);
            let (vx, vy) = (r(half * k(m, 0)) as f64, r(half * k(m, 1)) as f64);
            let vz = half * k(m, 2);
            let g = |o: u32| f(m, this + o);
            let v = [
                r(vz * g(0x28) + vy * g(0x1c) + vx * g(0x10)),
                r(vx * g(0x14) + vz * g(0x2c) + vy * g(0x20)),
                r(vx * g(0x18) + vz * g(0x30) + vy * g(0x24)),
            ];
            let vv = SCRATCH + 0x330;
            set3(m, vv, v);
            if wheel % 2 == 0 {
                crate::matrix::vec_add_in_place(m, rec + 0x34, SCRATCH + 0x340, vv);
            } else {
                crate::matrix::vec_sub_in_place(m, rec + 0x34, SCRATCH + 0x340, vv);
            }
        }
        let kind = fn_00502780(m, this, rec, wheel);
        fn_00503500(m, this, rec, wheel, kind);
    }
    copy_words(m, this + 0x3510 + 0xd0 * wheel, rec, 0x34);
    1
}

/// `0x503500`: rolling resistance of wheel `wheel` on the surface of `rec` (kind `kind`, or the
/// surface record kind when 0): the force `+0xee8` along the wheel direction (`+0x1904`) and its
/// vector `+0xf08`, plus the drag of a damaged tyre.
pub fn fn_00503500(m: &mut Mem, this: u32, rec: u32, wheel: u32, kind: i32) {
    let (f4, v3o) = (4 * wheel, 12 * wheel);
    let force = this + 0xee8 + f4;
    let vec = this + 0xf08 + v3o;
    m.set_u32(force, 0);
    copy_words(m, vec, 0x702650, 3);
    let surf = m.u32(rec + 0x64);
    if surf == 0 {
        return;
    }
    let mut sp = m.f32(this + 0x1964 + f4);
    if sp > 10.0 {
        sp = 10.0;
    }
    let sp = sp as f64;
    let kind = if kind == 0 { m.i32(surf + 4) } else { kind };
    let locked = m.i16(this + 0x19a4 + 2 * wheel) != 0;
    let a = f(m, this + 0x1984 + f4);
    let mu = f(m, this + 0x2690 + f4);
    let load = f(m, this + 0xd20 + f4);
    let mass = f(m, this + 0x2640);
    let dt = crate::globals::dt_hi(m);
    // the force grows with speed up to `p` and falls off quadratically below the speed `x`
    let soft = |p: f64, x: f64| if sp <= x { (sp / x) * (sp / x) * p } else { p };
    let value: Option<f64> = match kind {
        1 | 8 if !locked => Some(crate::curves::fn_00507330(a as f32, mu as f32) as f64 * load),
        2 if !locked => {
            let p = r(a / (a + 10.0) * mu * load) as f64;
            Some(soft(p, dt * p * 32.0 / mass + 0.01))
        }
        3 if !locked => {
            let p = r((sp + 5.0) * mu * load * 0.2f32 as f64) as f64;
            Some(soft(p, dt * p * 32.0 / mass))
        }
        6 | 10 | 11 if !locked => {
            let p = r(a / (a + 20.0) * mu * load) as f64;
            Some(soft(p, dt * p * 32.0 / mass + 0.01))
        }
        12 if !locked => Some(mu * load * sp.sqrt()),
        13 if !locked => {
            let st = this + 0x3cdc + 20 * wheel;
            if m.u32(this + 0x26c0 + f4) == 0 && m.i32(st) != m.i32(st + 4) {
                m.set_u32(this + 0x3c84, 1);
                m.set_u32(this + 0x26c0 + f4, 1);
                let t = crate::globals::time_hi(m);
                m.set_f32(this + 0x26e0 + f4, r(t - 2.0 * f(m, st + 0xc)));
            }
            Some(mu * load * sp.sqrt())
        }
        4 | 7 | 9 => {
            let mut v = 0.0;
            if !locked {
                let mut p = r(a / (a + 20.0) * mu) as f64;
                let lim = f(m, this + 0x2670 + f4);
                p = if p <= lim { r(p * load) as f64 } else { r(load * lim) as f64 };
                v = soft(p, dt * p * 32.0 / mass + 0.01);
            }
            let mut k = 0.5f32 as f64;
            if (m.u32(surf + 8) << 2) > 8 {
                let params = m.u32(surf + 0xc);
                if params != 0 {
                    k = (f(m, params + 8) - f(m, rec + 0x3c)) * 0.5;
                }
            }
            m.set_f32(force, r(-(a * a / (a + 20.0) * f(m, this + 0x34b4) * k * 15000.0 + v)));
            None
        }
        _ => None,
    };
    if let Some(v) = value {
        m.set_f32(force, r(-v));
    }
    let g = v3(m, this + 0x1904 + v3o);
    let fv = f(m, force);
    let t = SCRATCH + 0x400;
    set3(m, t, [r(fv * g[0]), r(fv * g[1]), r(fv * g[2])]);
    crate::matrix::vec_add_in_place(m, vec, SCRATCH + 0x410, t);
    let dmg = m.i32(this + 0x3cdc + 20 * wheel);
    if dmg != 0 && m.f32(this + 0x1b74 + f4) > 0.0 {
        let q = load * a * 5.0 / (a + 5.0) * (dmg as f64 / m.i32(this + 0x3ce0 + 20 * wheel) as f64);
        set3(m, t, [r(q * g[0]), r(q * g[1]), r(q * g[2])]);
        crate::matrix::vec_sub_in_place(m, vec, SCRATCH + 0x410, t);
    }
}

/// Surface tables filled by static initialisers of the original (`0x5021c0`, `0x501fb0`):
/// rolling resistance per surface kind for the cosine and the sine of the slip angle, and grip
/// per kind for the four combinations of season (`0x69784c`) and tyre type (`+0x3f4c`).
pub const SURF_ROLL_COS: u32 = 0x697688;
pub const SURF_ROLL_SIN: u32 = 0x6976c0;
pub const SURF_GRIP: u32 = 0x6976f8;
/// `0x69784c`: 1 in winter (slippery roads).
pub const WINTER: u32 = 0x69784c;
/// `0x696bdc`: global grip factor (1.4).
pub const GRIP_SCALE: u32 = 0x696bdc;
/// `0x6ef1fc`: the object whose position makes surfaces of kind 8 kick the wheels.
pub const KICK_OBJECT: u32 = 0x6ef1fc;

const ROLL_COS: [f32; 14] = [0.0, 0.1, 0.2, 0.3, 0.3, 0.1, 0.2, 0.2, 0.3, 0.4, 0.1, 0.2, 5.0, 5.0];
const ROLL_SIN: [f32; 14] = [0.0, 0.05, 1.5, 3.0, 0.6, 0.05, 1.0, 0.6, 0.6, 1.0, 0.05, 1.5, 5.0, 5.0];
const GRIP: [f32; 56] = [
    0.0, 0.6, 0.55, 0.3, 0.3, 0.45, 0.45, 0.3, 0.45, 0.2, 0.6, 0.45, 0.0, 0.0, //
    0.0, 0.5, 0.35, 0.3, 0.3, 0.4, 0.35, 0.3, 0.35, 0.2, 0.5, 0.35, 0.0, 0.0, //
    0.0, 0.8, 0.6, 0.2, 0.3, 0.8, 0.45, 0.3, 0.45, 0.2, 0.7, 0.7, 0.0, 0.0, //
    0.0, 0.7, 0.35, 0.2, 0.3, 0.7, 0.35, 0.3, 0.35, 0.2, 0.5, 0.25, 0.0, 0.0,
];

/// Install the surface tables and the global factors the original sets up at start-up.
pub fn install_surface_tables(m: &mut Mem) {
    for (i, v) in ROLL_COS.iter().enumerate() {
        m.set_f32(SURF_ROLL_COS + 4 * i as u32, *v);
    }
    for (i, v) in ROLL_SIN.iter().enumerate() {
        m.set_f32(SURF_ROLL_SIN + 4 * i as u32, *v);
    }
    for (i, v) in GRIP.iter().enumerate() {
        m.set_f32(SURF_GRIP + 4 * i as u32, *v);
    }
    m.set_u32(DEFAULT_SURFACE, 1);
    m.set_u32(DEFAULT_SURFACE + 4, 12);
    m.set_u32(DEFAULT_SURFACE + 8, 0);
    m.set_u32(DEFAULT_SURFACE + 12, 0);
    m.set_f32(GRIP_SCALE, 1.4);
    m.set_f32(SAME_PLANE, 0.044680428);
}

/// `0x6749f0`: tolerance factor for wheels standing on one plane (`0x506750`).
pub const SAME_PLANE: u32 = 0x6749f0;

/// Random bumps of the road under a wheel: the vertical speed `+0xe5c` and offset `+0xe3c`.
/// `lim` bounds the speed, `amp` the shape of the noise, `z` its strength and `clamp` the offset.
fn bump(m: &mut Mem, this: u32, wheel: u32, h: f64, ratio: f64, x_rnd: f64, z: f64, lim: f64, clamp: f64, sqrt: bool) {
    let f4 = 4 * wheel;
    let (vs, off) = (this + 0xe5c + f4, this + 0xe3c + f4);
    let _ = h;
    let shape = if sqrt { ratio.sqrt() * 0.9 + 0.1 } else { ratio * 0.9 + 0.1 };
    let v = r(x_rnd * shape * z + f(m, vs));
    m.set_f32(vs, v);
    if (v as f64) > lim {
        m.set_f32(vs, lim as f32);
    }
    if -lim > f(m, vs) {
        m.set_f32(vs, (-lim) as f32);
    }
    let dt = crate::globals::dt(m) as f64;
    let o = r(dt * f(m, vs) + f(m, off));
    m.set_f32(off, o);
    if (o as f64) > clamp {
        m.set_f32(off, clamp as f32);
    }
    if f(m, off) < -clamp {
        m.set_f32(off, (-clamp) as f32);
    }
}

/// `0x502780`: grip and rolling resistance of the surface under wheel `wheel` (`rec` is the
/// contact query), road bumps, the kick of surfaces of kind 8 and the sliding marks (`+0x34d0`).
/// Returns the surface kind (13 inside a special zone).
pub fn fn_00502780(m: &mut Mem, this: u32, rec: u32, wheel: u32) -> i32 {
    use crate::tech_system as ts;
    let (f4, v3o) = (4 * wheel, 12 * wheel);
    let h = m.f32(this + 0xe3c + f4) as f64;
    copy_words(m, this + 0xf08 + v3o, 0x702650, 3);
    m.set_u32(this + 0xee8 + f4, 0);
    m.set_u32(this + 0xf68 + f4, 0);
    m.set_u32(this + 0x34d0 + f4, 0);
    let surf = m.u32(rec + 0x64);
    if surf == 0 {
        return 0;
    }
    let mut sp = m.f32(this + 0x1964 + f4);
    if sp > 10.0 {
        sp = 10.0;
    }
    let sp = sp as f64;
    let dt = crate::globals::dt(m) as f64;
    let cos_a = r(f(m, this + 0x1a7c + f4).abs()) as f64;
    let sin_a = r(f(m, this + 0x1a5c + f4).abs()) as f64;
    let (roll, grip) = (this + 0x2690 + f4, this + 0x2670 + f4);
    m.set_f32(roll, 0.01);
    m.set_f32(grip, if m.i32(WINTER) == 1 { 0.3 } else { 0.5 });
    let mut kind = m.i32(surf + 4);
    let wb = this + 0xf8 * wheel;
    if m.i16(wb + 0x1030) != 0 {
        let pt = SCRATCH + 0x500;
        copy_words(m, pt, wb + 0x1018, 3);
        let hit = SCRATCH + 0x510;
        copy_words(m, hit, rec + 0x34, 3);
        for i in 0..m.i32(this + 0x29d0).max(0) as u32 {
            if fn_00502270(m, this + 0x2900 + 0x30 * i, pt, hit) != 0 {
                kind = 13;
            }
        }
    }
    let locked = m.i16(this + 0x19a4 + 2 * wheel) != 0;
    let skid = m.rd8(this + 0x19c4 + wheel) != 0;
    let slipping = m.i32(this + 0x19d4 + f4) != 0;
    let marks = m.i16(this + 0x2740) != 0;
    let mode = m.i32(this + 0x276c);
    let rnd = || 0.0f64;
    let _ = rnd;
    if m.u32(surf) == 1 {
        let sk = m.i32(surf + 4);
        if (1..14).contains(&kind) {
            let k = sk as u32;
            m.set_f32(roll, r(sin_a * f(m, SURF_ROLL_SIN + 4 * k) + cos_a * f(m, SURF_ROLL_COS + 4 * k)));
            let idx = (sk + (m.i32(WINTER) + m.i32(this + 0x3f4c) * 2) * 14) as u32;
            let g = m.u32(SURF_GRIP + 4 * idx);
            m.set_u32(grip, g);
        }
        let params = |m: &Mem| -> Option<u32> {
            if (m.u32(surf + 8) << 2) == 8 {
                Some(m.u32(surf + 0xc))
            } else {
                None
            }
        };
        let set_params = |m: &mut Mem, p: u32| {
            let a = m.u32(p);
            let b = m.u32(p + 4);
            m.set_u32(grip, a);
            m.set_u32(roll, b);
        };
        let slip_angle = f(m, this + 0x1a30 + f4).abs();
        let mut common = false;
        match sk {
            1 | 5 => common = true,
            8 => {
                if !locked {
                    let mut kick = true;
                    if !marks {
                        let obj = m.u32(KICK_OBJECT);
                        let d = [
                            r(m.f64(obj + 0xb58) as f32 as f64 - f(m, this + 0x34)) as f64,
                            r(m.f64(obj + 0xb60) as f32 as f64 - f(m, this + 0x38)) as f64,
                            r(m.f64(obj + 0xb68) as f32 as f64 - f(m, this + 0x3c)) as f64,
                        ];
                        let dist = (d[2] * d[2] + d[1] * d[1] + d[0] * d[0]).sqrt();
                        let chance = 15.0 / (dist + 0.1);
                        let rv = crate::globals::rand(m) as f64 * 3.05185094e-05f32 as f64;
                        if rv <= chance {
                            kick = false;
                        }
                    }
                    if kick {
                        let n = v3(m, rec + 0x40);
                        m.set_u32(this + 0x3c58, 1);
                        let fv = this + 0x3c5c;
                        set3(m, fv, [r(n[0] * 4000.0), r(n[1] * 4000.0), r(n[2] * 4000.0)]);
                        copy_words(m, this + 0x3c68, rec + 0x34, 3);
                        let cnt = m.u32(surf + 8) << 2;
                        if cnt >= 4 {
                            let p = m.u32(surf + 0xc);
                            let w = m.u32(p);
                            m.set_u32(this + 0x3c58, w);
                        }
                        if cnt >= 8 {
                            let p = m.u32(surf + 0xc);
                            let s = f(m, p + 4);
                            set3(m, fv, [r(s * n[0]), r(s * n[1]), r(s * n[2])]);
                        }
                        crate::matrix::vec_scale_in_place(m, fv, SCRATCH + 0x520, r(f(m, this + 0x2640) * 0.00039999999f32 as f64));
                        let fvv = v3(m, fv);
                        let acc = SCRATCH + 0x530;
                        set3(m, acc, [r(fvv[0] / dt), r(fvv[1] / dt), r(fvv[2] / dt)]);
                        let pt = SCRATCH + 0x540;
                        crate::matrix::transform(m, pt, this + 0x3c68, this + 0x900);
                        fn_004ebae0(m, this + 0x2700, pt, acc, 0x702650);
                    }
                }
                common = true;
            }
            2 => {
                if ts::value(m, this + 0x3f24) == 2.0 {
                    m.set_f32(roll, r(f(m, roll) * 0.666666667));
                }
                if let Some(p) = params(m) {
                    set_params(m, p);
                }
                if !locked {
                    let a = (skid && slip_angle > 0.174533) || slipping;
                    if marks && a {
                        m.set_u32(this + 0x34d0 + f4, 1);
                    }
                    if marks && a {
                        m.set_u32(this + 0x34d0 + f4, 2);
                    }
                    let (lim, amp) = match mode {
                        2 => (r(sp * 0.05) as f64, r(f(m, this + 0x1f9c + f4) * 0.05) as f64),
                        1 => (r(sp * 0.1) as f64, r(f(m, this + 0x1f9c + f4) * 0.2) as f64),
                        _ => (r(sp * 0.5) as f64, r(f(m, this + 0x1f9c + f4) * 0.5) as f64),
                    };
                    let ratio = r(h.abs() / amp) as f64;
                    let vs = this + 0xe5c + f4;
                    m.set_f32(vs, r((1.0 - ratio) * f(m, vs)));
                    let rnd = crate::globals::rand(m) as f64 * 3.05185094e-05f32 as f64;
                    let x = rnd - (h + amp) / (amp + amp);
                    let z = 0.1 / (dt + 0.01) * sp * 2.0 / (sp + 10.0);
                    bump(m, this, wheel, h, ratio, x, z, lim, 0.1f32 as f64, true);
                }
            }
            3 => {
                if let Some(p) = params(m) {
                    set_params(m, p);
                }
                if !locked {
                    let sa = f(m, this + 0x1a5c + f4).abs();
                    let a = (skid && sa > 0.2) || slipping;
                    if marks && a {
                        m.set_u32(this + 0x34d0 + f4, 1);
                    }
                    if marks && a {
                        m.set_u32(this + 0x34d0 + f4, 2);
                    }
                }
            }
            4 | 7 | 9 => {
                if !locked {
                    if let Some(p) = params(m) {
                        set_params(m, p);
                    }
                    let (amp, lim) = if sk == 9 { (0.05f32 as f64, 2.0) } else { (0.3f32 as f64, 10.0) };
                    let ratio = r(h.abs() / amp) as f64;
                    let vs = this + 0xe5c + f4;
                    m.set_f32(vs, r((1.0 - ratio) * f(m, vs)));
                    let rnd = crate::globals::rand(m) as f64 * 3.05185094e-05f32 as f64;
                    let x = rnd - (h + amp) / (amp + amp);
                    let z = sp * 5.0 / (sp + 10.0);
                    bump(m, this, wheel, h, ratio, x, z, lim, amp, false);
                }
            }
            6 | 10 | 11 => {
                if let Some(p) = params(m) {
                    set_params(m, p);
                }
                if !locked {
                    let ratio = r(h.abs() * 4.99999993) as f64;
                    let vs = this + 0xe5c + f4;
                    m.set_f32(vs, r((1.0 - ratio) * f(m, vs)));
                    let rnd = crate::globals::rand(m) as f64 * 3.05185094e-05f32 as f64;
                    let x = rnd - (h + 0.2f32 as f64) * 2.5;
                    let z = sp / (sp + 10.0);
                    bump(m, this, wheel, h, ratio, x, z, 10.0, 0.3f32 as f64, true);
                }
            }
            _ => {}
        }
        if common {
            let cnt = m.u32(surf + 8) << 2;
            if cnt >= 8 {
                let p = if m.i32(surf + 4) == 8 {
                    if cnt == 0x10 {
                        m.u32(surf + 0xc) + 8
                    } else {
                        0
                    }
                } else {
                    m.u32(surf + 0xc)
                };
                if p != 0 {
                    set_params(m, p);
                    if m.i32(WINTER) == 1 {
                        m.set_f32(grip, r(f(m, grip) * 0.6));
                    }
                }
            }
            if ((skid && slip_angle > 0.174533) || slipping) && !locked && marks {
                m.set_u32(this + 0x34d0 + f4, 2);
            }
            let (lim, amp) = match mode {
                2 => (r(sp * 0.003) as f64, r(f(m, this + 0x1f9c + f4) * 0.05) as f64),
                1 => (r(sp * 0.03) as f64, r(f(m, this + 0x1f9c + f4) * 0.1) as f64),
                _ => (sp, r(f(m, this + 0x1f9c + f4) * 0.2) as f64),
            };
            let ratio = r(h.abs() / amp) as f64;
            let vs = this + 0xe5c + f4;
            m.set_f32(vs, r((1.0 - ratio) * f(m, vs)));
            let rnd = crate::globals::rand(m) as f64 * 3.05185094e-05f32 as f64;
            let x = rnd - (h + amp) / (amp + amp);
            let z = 0.1 / (dt + 0.01) * sp / (sp + 10.0);
            bump(m, this, wheel, h, ratio, x, z, lim, 0.0199999996f32 as f64, true);
        }
    }
    let g = ts::value(m, this + 0x3f24) as f64 * f(m, grip) * f(m, GRIP_SCALE);
    m.set_f32(grip, r(g));
    let sk = m.i32(surf + 4);
    if mode == 2 && (sk == 1 || sk == 5) {
        m.set_f32(grip, r(g * 1.5 + 0.3));
    } else if mode != 0 {
        m.set_f32(grip, r(g * 1.1 + 0.1));
    }
    copy_words(m, this + 0x3b90 + f4, this + 0xe3c + f4, 1);
    kind
}

/// `0x4ebae0` (`this` = vehicle + `0x2700`): queue an external push (point `p2`, acceleration
/// `p3`, `p4`) for the step; at most five per step.
pub fn fn_004ebae0(m: &mut Mem, this: u32, p2: u32, p3: u32, p4: u32) -> i32 {
    let n = m.i32(this + 0xc0);
    if n > 4 {
        return 0;
    }
    let k = n as u32 * 12;
    copy_words(m, this + 0x104 + k, p3, 3);
    copy_words(m, this + 0x140 + k, p4, 3);
    copy_words(m, this + 200 + k, p2, 3);
    m.set_i32(this + 0xc0, n + 1);
    n
}

/// `0x502270`: does the segment `p2`..`p3` touch the zone `zone` (a quad of four corner points,
/// 48 bytes)? True when either end lies inside or the segment crosses one of the four sides.
pub fn fn_00502270(m: &mut Mem, zone: u32, p2: u32, p3: u32) -> u32 {
    use crate::matrix as mx;
    let rz = SCRATCH + 0x600;
    mx::mat_rot_z(m, rz, 1.5707964);
    let corner = |m: &Mem, i: u32| v3(m, zone + 12 * i);
    let mut e = [[0.0f64; 3]; 4];
    let mut len = [0.0f64; 4];
    for i in 0..4usize {
        if i < 2 {
            let (a, b) = (corner(m, i as u32), corner(m, i as u32 + 1));
            let d = [r(b[0] - a[0]), r(b[1] - a[1]), r(b[2] - a[2])];
            let l = ((d[2] as f64).powi(2) + (d[1] as f64).powi(2) + (d[0] as f64).powi(2)).sqrt();
            len[i] = r(l) as f64;
            let mut dd = [d[0] as f64, d[1] as f64, d[2] as f64];
            if l != 0.0 {
                let s = r(1.0 / l) as f64;
                dd = [r(s * dd[0]) as f64, r(s * dd[1]) as f64, r(s * dd[2]) as f64];
            }
            e[i] = dd;
        } else {
            len[i] = len[i - 2];
            e[i] = [-e[i - 2][0], -e[i - 2][1], -e[i - 2][2]];
        }
    }
    let q: [f64; 12] = core::array::from_fn(|k| f(m, rz + 4 * k as u32));
    let mut n = [[0.0f64; 3]; 4];
    for i in 0..4usize {
        if i < 2 {
            let v = e[i];
            n[i] = [
                r(q[3] * v[1] + q[0] * v[0] + q[6] * v[2] + q[9]) as f64,
                r(q[4] * v[1] + q[1] * v[0] + q[7] * v[2] + q[10]) as f64,
                r(q[5] * v[1] + q[2] * v[0] + q[8] * v[2] + q[11]) as f64,
            ];
        } else {
            n[i] = [-n[i - 2][0], -n[i - 2][1], -n[i - 2][2]];
        }
    }
    let inside = |m: &Mem, p: [f64; 3]| {
        (0..4u32).all(|i| {
            let c = corner(m, i);
            let ni = n[i as usize];
            r(p[0] - c[0]) as f64 * ni[0] + r(p[1] - c[1]) as f64 * ni[1] + r(p[2] - c[2]) as f64 * ni[2] < 0.0
        })
    };
    let (a, b) = (v3(m, p2), v3(m, p3));
    if inside(m, a) || inside(m, b) {
        return 1;
    }
    let d = [r(b[0] - a[0]) as f64, r(b[1] - a[1]) as f64, r(b[2] - a[2]) as f64];
    let l = (d[2] * d[2] + d[1] * d[1] + d[0] * d[0]).sqrt();
    if l == 0.0 {
        return 0;
    }
    let s = r(1.0 / l) as f64;
    let d = [r(s * d[0]) as f64, r(s * d[1]) as f64, r(s * d[2]) as f64];
    let mut any = false;
    for i in 0..4usize {
        let c = corner(m, i as u32);
        let ni = n[i];
        let w = [r(c[0] - a[0]) as f64, r(c[1] - a[1]) as f64, r(c[2] - a[2]) as f64];
        let num = ni[0] * w[0] + ni[1] * w[1] + ni[2] * w[2];
        let den = ni[0] * d[0] + ni[1] * d[1] + ni[2] * d[2];
        let t = num / den;
        let tf = t as f32 as f64;
        let hit = [r(a[0] + r(d[0] * tf) as f64) as f64, r(a[1] + r(d[1] * tf) as f64) as f64, r(a[2] + r(d[2] * tf) as f64) as f64];
        let mut ok = !(t < 0.0 || l < t);
        if ok {
            let ei = e[i];
            let s = r(hit[1] - c[1]) as f64 * ei[1] + r(hit[0] - c[0]) as f64 * ei[0] + r(hit[2] - c[2]) as f64 * ei[2];
            if s < 0.0 || len[i] < s {
                ok = false;
            }
        }
        any |= ok;
    }
    any as u32
}

/// `0x506750`: do the two wheels of side / axle `pair` stand on the same plane (normals within
/// 0.999 and contact points within `0.0447 * +0x1f64[pair]` of each other's plane)? Always true
/// for vehicles with fewer than four wheels.
pub fn fn_00506750(m: &Mem, this: u32, pair: u32) -> u32 {
    if m.i32(this + 0xdd4) < 4 {
        return 1;
    }
    let w = |i: u32| this + 0xf8 * (i + 1);
    let (a, b, n) = match pair {
        0 => (0, 2, 2),
        1 => (0, 1, 0),
        2 => (1, 3, 1),
        3 => (2, 3, 2),
        _ => return 0,
    };
    if m.i16(w(a) + 0x18) == 0 || m.i16(w(b) + 0x18) == 0 {
        return 0;
    }
    let (na, nb) = (v3(m, w(a) + 0xc), v3(m, w(b) + 0xc));
    if na[2] * nb[2] + na[1] * nb[1] + na[0] * nb[0] <= 0.999 {
        return 0;
    }
    let (pa, pb) = (v3(m, w(a)), v3(m, w(b)));
    let nn = v3(m, w(n) + 0xc);
    let d = [r(pa[0] - pb[0]) as f64, r(pa[1] - pb[1]) as f64, r(pa[2] - pb[2]) as f64];
    let dist = (d[0] * nn[0] + d[1] * nn[1] + d[2] * nn[2]).abs();
    let limit = f(m, SAME_PLANE) * f(m, this + 0x1f64 + 4 * pair);
    (dist < limit) as u32
}

/// `0x506210`: the ground normal `+0xd0` from the planes through each three of the four wheel
/// contacts (or the default normal `+0xc4`).
pub fn fn_00506210(m: &mut Mem, this: u32, flag: u32) {
    let fallback = |m: &mut Mem| copy_words(m, this + 0xd0, this + 0xc4, 3);
    if m.i32(this + 0xdd4) != 4 {
        fallback(m);
        return;
    }
    let use_wheels = (m.rd8(this + 0x19b9) == 0 || (flag != 0 && m.f32(this + 0xe38) < 1.0)) && m.u32(this + 0x8b8) == 0;
    if !use_wheels {
        fallback(m);
        return;
    }
    let p = |m: &Mem, i: u32| v3(m, this + 0xf8 * (i + 1));
    let on = |m: &Mem, i: u32| m.i16(this + 0xf8 * (i + 1) + 0x18) != 0;
    let sub = |x: [f64; 3], y: [f64; 3]| [r(x[0] - y[0]) as f64, r(x[1] - y[1]) as f64, r(x[2] - y[2]) as f64];
    // normal of the plane through `o`, `o + a`, `o + b` the way the original writes it
    let tri = |a: [f64; 3], b: [f64; 3]| [r(a[2] * b[1] - a[1] * b[2]), r(b[2] * a[0] - a[2] * b[0]), r(a[1] * b[0] - b[1] * a[0])];
    let mut normals: [Option<[f32; 3]>; 4] = [None; 4];
    if on(m, 0) && on(m, 1) && on(m, 2) {
        normals[0] = Some(tri(sub(p(m, 1), p(m, 0)), sub(p(m, 2), p(m, 0))));
    }
    if on(m, 0) && on(m, 1) && on(m, 3) {
        normals[1] = Some(tri(sub(p(m, 3), p(m, 1)), sub(p(m, 0), p(m, 1))));
    }
    if on(m, 0) && on(m, 2) && on(m, 3) {
        let a = sub(p(m, 0), p(m, 2));
        let b = sub(p(m, 3), p(m, 2));
        normals[2] = Some([r(b[1] * a[2] - b[2] * a[1]), r(b[2] * a[0] - b[0] * a[2]), r(b[0] * a[1] - b[1] * a[0])]);
    }
    if on(m, 1) && on(m, 2) && on(m, 3) {
        let b = sub(p(m, 2), p(m, 3));
        let a = sub(p(m, 1), p(m, 3));
        normals[3] = Some([r(b[2] * a[1] - a[2] * b[1]), r(a[2] * b[0] - b[2] * a[0]), r(b[1] * a[0] - b[0] * a[1])]);
    }
    let (sum, tmp, u) = (SCRATCH + 0x700, SCRATCH + 0x710, SCRATCH + 0x720);
    copy_words(m, sum, 0x702650, 3);
    let mut count = 0;
    for n in normals.iter().flatten() {
        set3(m, tmp, *n);
        crate::matrix::vec_unit(m, u, tmp);
        crate::matrix::vec_add_in_place(m, sum, SCRATCH + 0x730, u);
        count += 1;
    }
    if flag != 0 && count == 0 {
        fallback(m);
        return;
    }
    crate::matrix::vec_unit(m, this + 0xd0, sum);
}

/// `0x4e6490`: where the line through `p4` along `p5` meets the plane through `p2` with normal `p3`.
pub fn line_plane_dir(m: &mut Mem, dst: u32, p2: u32, p3: u32, p4: u32, p5: u32) {
    let s = SCRATCH + 0x800;
    crate::matrix::vec_sub(m, s, p2, p4);
    let num = crate::matrix::vec_dot(m, p3, s) as f64;
    let den = crate::matrix::vec_dot(m, p3, p5);
    let t = (num / den) as f32;
    crate::matrix::vec_scale(m, s + 0x10, p5, t);
    crate::matrix::vec_add(m, dst, p4, s + 0x10);
}

/// `0x505530`: suspension contact of all wheels for the body transform `mat`. The two wheels at
/// the leading end (by the sign of the speed) query the world ([`fn_00503e30`]); the others
/// borrow the contact plane of the nearest of them and query only when it does not fit. Sets the
/// spring compressions `+0xe18`, wheel loads `+0xd20`, airborne flags `+0x19a4..` and their
/// combinations `+0x19b4..+0x19ba`, the average ground normal `+0xc4` and contact `+0x2214`, and
/// the ground plane `+0xd0` ([`fn_00506210`]). Returns the number of wheels on the ground.
pub fn fn_00505530(m: &mut Mem, world: &mut dyn World, this: u32, mat: u32) -> u32 {
    use crate::matrix as mx;
    let inv = SCRATCH + 0x900;
    crate::math::mat_inverse(m, inv, mat);
    copy_words(m, this + 0xc4, 0x702650, 3);
    copy_words(m, this + 0x2214, 0x702650, 3);
    m.set_u32(this + 0x26b0, 0);
    let count = m.i32(this + 0xdd4).max(0) as u32;
    let pos = SCRATCH + 0xa00; // world positions of the wheel mounts, 12 bytes each
    let mt: [f64; 12] = core::array::from_fn(|k| f(m, mat + 4 * k as u32));
    for i in 0..count {
        let (f4, v3o) = (4 * i, 12 * i);
        copy_words(m, this + 0xf08 + v3o, 0x702650, 3);
        m.set_u32(this + 0xee8 + f4, 0);
        m.set_f32(this + 0x2690 + f4, 0.01);
        m.set_f32(this + 0x2670 + f4, 0.5);
        let c = v3(m, this + 0x1cbc + v3o);
        set3(
            m,
            pos + v3o,
            [
                r(c[0] * mt[0] + c[1] * mt[3] + c[2] * mt[6] + mt[9]),
                r(mt[1] * c[0] + c[2] * mt[7] + c[1] * mt[4] + mt[10]),
                r(mt[8] * c[2] + mt[5] * c[1] + c[0] * mt[2] + mt[11]),
            ],
        );
    }
    // the two wheels with the smallest and the two with the largest mount y
    let (mut lo1, mut lo2, mut hi1) = (100.0f32, 100.0f32, -100.0f32);
    let mut hi2 = -100.0f32;
    let (mut lo1_i, mut lo2_i, mut hi1_i, mut hi2_i) = (0u32, 0u32, 0u32, 0u32);
    for i in 0..count {
        let y = m.f32(this + 0x1cc0 + 12 * i);
        if y <= lo2 {
            if y <= lo1 {
                lo2 = lo1;
                lo2_i = lo1_i;
                lo1 = y;
                lo1_i = i;
            } else {
                lo2 = y;
                lo2_i = i;
            }
        }
        if hi2 <= y {
            if y < hi1 {
                hi2 = y;
                hi2_i = i;
            } else {
                hi2 = hi1;
                hi2_i = hi1_i;
                hi1 = y;
                hi1_i = i;
            }
        }
    }
    let mut queried = [false; 8];
    let mut res = [0u32; 8];
    if m.f32(this + 0xdc8) < 0.0 {
        queried[lo1_i as usize] = true;
        queried[lo2_i as usize] = true;
    } else {
        queried[hi1_i as usize] = true;
        queried[hi2_i as usize] = true;
    }
    for i in 0..count {
        if queried[i as usize] {
            res[i as usize] = fn_00503e30(m, world, this, 0, pos + 12 * i, 0, i);
        }
    }
    let up = SCRATCH + 0xb00;
    let k = [f(m, 0x702630), f(m, 0x702634), f(m, 0x702638)];
    set3(
        m,
        up,
        [
            r(k[2] * mt[6] + k[0] * mt[0] + k[1] * mt[3]),
            r(k[0] * mt[1] + k[2] * mt[7] + k[1] * mt[4]),
            r(k[2] * mt[8] + k[1] * mt[5] + k[0] * mt[2]),
        ],
    );
    let wheel = |i: u32| this + 0xf8 * (i + 1);
    for i in 0..count {
        if queried[i as usize] {
            continue;
        }
        let (mut best, mut idx) = (1000.0f32, -1i32);
        let mi = v3(m, this + 0x1cbc + 12 * i);
        for k in 0..count {
            if queried[k as usize] && m.i16(wheel(k) + 0x18) != 0 {
                let mk = v3(m, this + 0x1cbc + 12 * k);
                let d = [r(mk[0] - mi[0]) as f64, r(mk[1] - mi[1]) as f64, mk[2] - mi[2]];
                let d2 = d[0] * d[0] + d[1] * d[1] + d[2] * d[2];
                if d2 < best as f64 {
                    best = d2 as f32;
                    idx = k as i32;
                }
            }
        }
        if idx >= 0 {
            let k = idx as u32;
            copy_words(m, this + 0x3510 + 0xd0 * i, this + 0x3510 + 0xd0 * k, 0x34);
            copy_words(m, wheel(i), wheel(k), 0x3e);
            let hit = SCRATCH + 0xb10;
            line_plane_dir(m, hit, wheel(k), wheel(k) + 0xc, pos + 12 * i, up);
            copy_words(m, wheel(i), hit, 3);
            res[i as usize] = res[k as usize];
        }
    }
    let it: [f64; 12] = core::array::from_fn(|k| f(m, inv + 4 * k as u32));
    let mut on_ground = 0u32;
    for i in 0..count {
        let (f4, v3o, iu) = (4 * i, 12 * i, i as usize);
        let w = wheel(i);
        let lone = !(!queried[iu] && res[iu] != 0);
        let contact = this + 0x64 + v3o;
        let local = this + 0x1844 + v3o;
        let comp_at = |m: &mut Mem| {
            copy_words(m, contact, w, 3);
            let p = v3(m, contact);
            set3(
                m,
                local,
                [
                    r(it[0] * p[0] + it[3] * p[1] + it[6] * p[2] + it[9]),
                    r(it[4] * p[1] + it[1] * p[0] + it[7] * p[2] + it[10]),
                    r(it[5] * p[1] + it[2] * p[0] + it[8] * p[2] + it[11]),
                ],
            );
            let c = r(f(m, this + 0x1cc4 + v3o) - f(m, this + 0x1b74 + f4) - f(m, local + 8));
            m.set_f32(this + 0xe18 + f4, c);
            m.set_f32(this + 0xdd8 + f4, c);
            c
        };
        let mut requery = false;
        if (queried[iu] && res[iu] == 1) || (!lone && res[iu] == 1) {
            let c = comp_at(m) as f64;
            if (c < 0.0 || c > f(m, this + 0x1f9c + f4)) && !lone {
                requery = true;
            }
        } else if !lone {
            requery = true;
        }
        if requery {
            res[iu] = fn_00503e30(m, world, this, 0, pos + v3o, 0, i);
            if res[iu] != 0 {
                comp_at(m);
            }
        }
        if res[iu] != 0 {
            mx::vec_add_in_place(m, this + 0xc4, SCRATCH + 0xb20, w + 0xc);
            m.set_f32(this + 0x26b0, r(f(m, this + 0x2670 + f4) + f(m, this + 0x26b0)));
            mx::vec_add_in_place(m, this + 0x2214, SCRATCH + 0xb20, contact);
            on_ground += 1;
            let travel = this + 0xdd8 + f4;
            if f(m, travel) > f(m, this + 0x1f9c + f4) {
                copy_words(m, travel, this + 0x1f9c + f4, 1);
                m.set_i16(this + 0x19a4 + 2 * i, 1);
            } else {
                if m.f32(travel) < 0.0 {
                    m.set_f32(travel, 0.0);
                }
                let load = f(m, w + 0x14).abs() * (f(m, this + 0x2640) / m.i32(this + 0xdd4) as f64) * 9.8;
                m.set_f32(this + 0xd20 + f4, r(load));
                m.set_i16(this + 0x19a4 + 2 * i, 0);
            }
        } else {
            // in the air: hang the wheel 1000 m down
            let s = [k[0] * 1000f32 as f64, k[1] * 1000f32 as f64, k[2] * 1000f32 as f64];
            let d = [
                r(s[0] * mt[0] + s[1] * mt[3] + s[2] * mt[6]) as f64,
                r(s[0] * mt[1] + s[2] * mt[7] + s[1] * mt[4]) as f64,
                s[2] * mt[8] + s[1] * mt[5] + s[0] * mt[2],
            ];
            let p = v3(m, pos + v3o);
            set3(m, contact, [r(p[0] - d[0]), r(p[1] - d[1]), r(p[2] - d[2])]);
            let mo = v3(m, this + 0x1cbc + v3o);
            set3(m, local, [r(mo[0] - s[0]), r(mo[1] - r(s[1]) as f64), r(mo[2] - r(s[2]) as f64)]);
            m.set_f32(this + 0xe18 + f4, 1000.0);
            copy_words(m, this + 0xdd8 + f4, this + 0x1f9c + f4, 1);
            m.set_u32(this + 0xd20 + f4, 0);
            m.set_i16(this + 0x19a4 + 2 * i, 1);
        }
    }
    let e = |m: &Mem, k: u32| m.f32(this + 0xe18 + 4 * k);
    let a = if e(m, 2) < e(m, 3) { e(m, 3) } else { e(m, 2) };
    let b = if e(m, 0) < e(m, 1) { e(m, 1) } else { e(m, 0) };
    m.set_f32(this + 0xe38, if b < a { a } else { b });
    if on_ground != 0 {
        crate::math::vec_normalize(m, this + 0xc4);
        let s = 1.0 / on_ground as f64;
        m.set_f32(this + 0x26b0, r(s * f(m, this + 0x26b0)));
        mx::vec_scale_in_place(m, this + 0x2214, SCRATCH + 0xb20, s as f32);
    } else {
        copy_words(m, this + 0xc4, 0x702630, 3);
        copy_words(m, this + 0x2214, 0x702650, 3);
    }
    let p1 = fn_00506750(m, this, 1);
    m.set_u32(this + 0x8c0, p1);
    let p3 = fn_00506750(m, this, 3);
    m.set_u32(this + 0x8c8, p3);
    let all = m.u32(this + 0x8bc) != 0 && m.u32(this + 0x8c0) != 0 && m.u32(this + 0x8c4) != 0 && p3 != 0;
    m.set_u32(this + 0x8b8, all as u32);
    let s = |m: &Mem, k: u32| m.i16(this + 0x19a4 + 2 * k) != 0;
    let n = m.i32(this + 0xdd4);
    let trike = n == 3 && m.i32(0x6ce33c) <= m.i32(this + 0x8cc) && m.i32(this + 0x8cc) < m.i32(0x6ce340);
    let (b7, b5, b6, b8) = if trike {
        (s(m, 0) && s(m, 1), m.rd8(this + 0x19a8) != 0, s(m, 0) && s(m, 2), s(m, 1) && s(m, 2))
    } else {
        (s(m, 2) && s(m, 3), s(m, 0) && s(m, 1), s(m, 0) && s(m, 2), s(m, 1) && s(m, 3))
    };
    m.wr8(this + 0x19b7, b7 as u8);
    if trike {
        let v = m.rd8(this + 0x19a8);
        m.wr8(this + 0x19b5, v);
    } else {
        m.wr8(this + 0x19b5, b5 as u8);
    }
    m.wr8(this + 0x19b6, b6 as u8);
    m.wr8(this + 0x19b8, b8 as u8);
    let (mut all_air, mut any_air) = (true, false);
    for k in 0..n.max(0) as u32 {
        all_air = all_air && s(m, k);
        any_air = any_air || s(m, k);
    }
    m.wr8(this + 0x19b4, all_air as u8);
    m.wr8(this + 0x19ba, any_air as u8);
    let any = [0x19b5, 0x19b6, 0x19b7, 0x19b8].iter().any(|o| m.rd8(this + o) != 0);
    m.wr8(this + 0x19b9, any as u8);
    fn_00506210(m, this, 0);
    on_ground
}

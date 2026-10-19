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

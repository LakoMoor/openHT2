//! `TechSystem` (`0x4eb460..0x4eb770`): one upgradable value of a vehicle (e.g. engine power,
//! fuel tank). It has up to three upgrade levels; each level maps a fraction `t` in `0..=1`
//! (how much of the value is left, or how far the upgrade has progressed) onto a range.
//!
//! Layout of the 0x34-byte object at `this`:
//! ```text
//! +0x00 f32[3] value at t = 0      +0x0c f32[3] value at t = 1
//! +0x18 f32[3] 1/rate of consumption   +0x24 f32 t
//! +0x28 i32 current level          +0x2c i32 number of levels defined
//! +0x30 i32 mode of switching level
//! ```

use crate::mem::Mem;

/// `0x4eb460`: reset to "nothing defined, t = 1".
pub fn reset(m: &mut Mem, this: u32) {
    m.set_i32(this + 0x28, 0);
    for i in 0..3 {
        m.set_f32(this + i * 4, 0.0);
        m.set_f32(this + 0xc + i * 4, 1.0);
        m.set_f32(this + 0x18 + i * 4, 1.0);
    }
    m.set_i32(this + 0x30, 0);
    m.set_i32(this + 0x2c, 0);
    m.set_f32(this + 0x24, 1.0);
}

/// `0x4eb580`: define level `idx` as the range `lo..hi`, consumed over `rate` (0 = never).
pub fn define(m: &mut Mem, this: u32, idx: i32, lo: f32, hi: f32, rate: f32) {
    if !(0..3).contains(&idx) {
        return;
    }
    let count = m.i32(this + 0x2c);
    if idx > count {
        return;
    }
    if idx == count {
        m.set_i32(this + 0x2c, count + 1);
    }
    let i = idx as u32 * 4;
    m.set_f32(this + i, lo);
    m.set_f32(this + 0xc + i, hi);
    // x87 compares of NaN leave the zero flag set, so a NaN rate counts as "no rate" like the original.
    if rate != 0.0 && !rate.is_nan() {
        m.set_f32(this + 0x18 + i, (1.0f64 / rate as f64) as f32);
    } else {
        m.set_f32(this + 0x18 + i, 0.0);
    }
}

fn level_lo(m: &Mem, this: u32) -> f32 {
    m.f32(this + m.i32(this + 0x28) as u32 * 4)
}

fn level_hi(m: &Mem, this: u32) -> f32 {
    m.f32(this + 0xc + m.i32(this + 0x28) as u32 * 4)
}

/// `0x4eb4a0`: switch to upgrade level `level`, keeping the current value where the mode asks for it.
pub fn set_upgrade(m: &mut Mem, this: u32, level: i32) {
    // The original logs an error for a level outside what is defined and carries on.
    let cur = m.i32(this + 0x28);
    if level == cur {
        return;
    }
    let l = level as u32 * 4;
    match m.i32(this + 0x30) {
        0 => {
            m.set_i32(this + 0x28, level);
            m.set_f32(this + 0x24, 1.0);
            return;
        }
        1 => {
            m.set_i32(this + 0x28, level);
            m.set_f32(this + 0x24, 0.0);
            return;
        }
        2 => {
            let span = m.f32(this + 0xc + l) - m.f32(this + l);
            if span != 0.0 {
                let c = cur as u32 * 4;
                let current = (m.f32(this + 0xc + c) as f64 - m.f32(this + c) as f64) * m.f32(this + 0x24) as f64 + m.f32(this + c) as f64;
                let mut t = ((current - m.f32(this + l) as f64) / span as f64) as f32;
                m.set_f32(this + 0x24, t);
                if 1.0 < t {
                    t = 1.0;
                    m.set_f32(this + 0x24, 1.0);
                }
                if t < 0.0 {
                    m.set_i32(this + 0x28, level);
                    m.set_f32(this + 0x24, 0.0);
                    return;
                }
            }
        }
        _ => {}
    }
    m.set_i32(this + 0x28, level);
}

/// `0x4eb5e0`: the current value `lo + (hi - lo) * t` of the current level.
pub fn value(m: &Mem, this: u32) -> f32 {
    let (lo, hi, t) = (level_lo(m, this) as f64, level_hi(m, this) as f64, m.f32(this + 0x24) as f64);
    ((hi - lo) * t + lo) as f32
}

/// `0x4eb600`: set `t` so that the value becomes `v` (clamped to the level's range).
pub fn set_value(m: &mut Mem, this: u32, v: f32) {
    let (lo, hi) = (level_lo(m, this), level_hi(m, this));
    let mut v = v;
    if hi <= lo {
        if lo < v {
            v = lo;
        }
        if hi > v {
            v = hi;
        }
    } else {
        if v < lo {
            v = lo;
        }
        if v > hi {
            v = hi;
        }
    }
    let span = hi - lo;
    if span != 0.0 {
        m.set_f32(this + 0x24, ((v as f64 - lo as f64) / span as f64) as f32);
    } else {
        m.set_f32(this + 0x24, 1.0);
    }
}

/// `0x4eb680`: the value at `t = 1` of the current level.
pub fn full_value(m: &Mem, this: u32) -> f32 {
    level_hi(m, this)
}

/// `0x4eb690`: `t`.
pub fn fraction(m: &Mem, this: u32) -> f32 {
    m.f32(this + 0x24)
}

/// `0x4eb6a0`: set `t` clamped to `0..=1`.
pub fn set_fraction(m: &mut Mem, this: u32, t: f32) {
    let t = if t < 0.0 {
        0.0
    } else if 1.0 < t {
        1.0
    } else {
        t
    };
    m.set_f32(this + 0x24, t);
}

/// `0x4eb6e0`: consume `amount` (scaled by the current level's rate); `t` stays within `0..=1`.
pub fn consume(m: &mut Mem, this: u32, amount: f32) {
    let rate = m.f32(this + 0x18 + m.i32(this + 0x28) as u32 * 4);
    let mut t = (m.f32(this + 0x24) as f64 - amount as f64 * rate as f64) as f32;
    m.set_f32(this + 0x24, t);
    if 1.0 < t {
        t = 1.0;
        m.set_f32(this + 0x24, t);
    }
    if t < 0.0 {
        m.set_f32(this + 0x24, 0.0);
    }
}

/// `0x4eb720`: set `t = 1 - amount * rate`, clamped to `0..=1`.
pub fn set_from_use(m: &mut Mem, this: u32, amount: f32) {
    let rate = m.f32(this + 0x18 + m.i32(this + 0x28) as u32 * 4);
    let mut t = (1.0 - amount as f64 * rate as f64) as f32;
    m.set_f32(this + 0x24, t);
    if 1.0 < t {
        t = 1.0;
        m.set_f32(this + 0x24, t);
    }
    if t < 0.0 {
        m.set_f32(this + 0x24, 0.0);
    }
}

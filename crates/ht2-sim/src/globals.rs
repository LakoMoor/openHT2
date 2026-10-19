//! Absolute addresses of the original's global state that the ported code reads.

use crate::mem::Mem;

/// `DAT_006974c4`: pointer to the clock object.
pub const CLOCK_PTR: u32 = 0x0069_74c4;
/// Offset of the current game time (`f64`, seconds) inside the clock object.
pub const CLOCK_TIME: u32 = 0xb98;
/// Offset of the last frame time (`f64`) inside the clock object.
pub const CLOCK_DT: u32 = 0xb90;

/// The current game time as the original reads it for filters: a `float`.
pub fn now(m: &Mem) -> f32 {
    m.f64(m.u32(CLOCK_PTR) + CLOCK_TIME) as f32
}

/// Install a clock object at `obj` and point the global at it.
pub fn install_clock(m: &mut Mem, obj: u32, time: f64, dt: f64) {
    m.set_u32(CLOCK_PTR, obj);
    m.set_f64(obj + CLOCK_TIME, time);
    m.set_f64(obj + CLOCK_DT, dt);
}

pub fn set_time(m: &mut Mem, time: f64) {
    let obj = m.u32(CLOCK_PTR);
    m.set_f64(obj + CLOCK_TIME, time);
}

/// `DAT_006974c2`: when non-zero the game runs with a fixed step of this many milliseconds.
pub const FIXED_STEP_MS: u32 = 0x0069_74c2;
/// `DAT_006974bc`: game time while the fixed step is active (`f32`).
pub const FIXED_TIME: u32 = 0x0069_74bc;

/// `0x4e27a5`: the current game time (`f32`).
pub fn time(m: &Mem) -> f32 {
    if m.i16(FIXED_STEP_MS) != 0 {
        m.f32(FIXED_TIME)
    } else {
        m.f64(m.u32(CLOCK_PTR) + CLOCK_TIME) as f32
    }
}

/// `0x4e2749`: the length of the current frame in seconds (`f32`), at most 0.25.
pub fn dt(m: &Mem) -> f32 {
    let step = m.i16(FIXED_STEP_MS);
    if step != 0 {
        (step as i32 as f64 / 1000.0) as f32
    } else {
        let d = m.f64(m.u32(CLOCK_PTR) + CLOCK_DT);
        if d <= 0.25 {
            d as f32
        } else {
            0.25
        }
    }
}

/// Static constant vectors the original initialises at start-up (`0x5ab350..0x5ab3fe`).
pub const VEC_Y: u32 = 0x0070_2620;
pub const VEC_UP: u32 = 0x0070_2630;
pub const VEC_X: u32 = 0x0070_2640;
pub const VEC_ZERO: u32 = 0x0070_2650;

/// Write the constant vectors (Z is up in the game world).
pub fn install_constants(m: &mut Mem) {
    for (addr, v) in [(VEC_Y, [0.0, 1.0, 0.0]), (VEC_UP, [0.0, 0.0, 1.0]), (VEC_X, [1.0, 0.0, 0.0]), (VEC_ZERO, [0.0, 0.0, 0.0])] {
        for (i, c) in v.iter().enumerate() {
            m.set_f32(addr + 4 * i as u32, *c);
        }
    }
}

/// `0x4e27a5` at full precision: the value the original leaves on the FPU stack (not rounded to `f32`).
pub fn time_hi(m: &Mem) -> f64 {
    if m.i16(FIXED_STEP_MS) != 0 {
        m.f32(FIXED_TIME) as f64
    } else {
        m.f64(m.u32(CLOCK_PTR) + CLOCK_TIME)
    }
}

/// `0x4e2749` at full precision.
pub fn dt_hi(m: &Mem) -> f64 {
    let step = m.i16(FIXED_STEP_MS);
    if step != 0 {
        step as i32 as f64 / 1000.0
    } else {
        let d = m.f64(m.u32(CLOCK_PTR) + CLOCK_DT);
        if d <= 0.25 {
            d
        } else {
            0.25
        }
    }
}

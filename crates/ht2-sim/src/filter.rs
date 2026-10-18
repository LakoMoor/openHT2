//! The time filter (`0x4eab58..0x4eb0c7`): a window of recent samples that is averaged with
//! weights depending on how old each sample is. The engine uses it to smooth engine speed.
//!
//! Object layout (`this`, 0x20 bytes, matching the original):
//! ```text
//! +0x00 f32 window length (s)      +0x04 f32 1/window
//! +0x08 *f32 sample times          +0x0c *f32 sample values
//! +0x10 f32 default value          +0x14 weight function address
//! +0x18 mix function address or 0  +0x1c i32 number of samples
//! ```

use crate::globals;
use crate::mem::Mem;

/// Addresses of the weight functions the original passes in (`x` is the age in windows, `-1..=0`).
pub const WEIGHT_STEP: u32 = 0x004e_aa0e;
pub const WEIGHT_LINEAR: u32 = 0x004e_aa41;
pub const WEIGHT_QUADRATIC: u32 = 0x004e_aa77;
pub const WEIGHT_COSINE: u32 = 0x004e_ab02;

fn in_window(x: f32) -> bool {
    !(x > 0.001f32 || x <= -1.0)
}

/// Evaluate a weight/mix function by its original address.
pub fn call_shape(addr: u32, x: f32) -> f32 {
    match addr {
        WEIGHT_STEP => {
            if in_window(x) {
                1.0
            } else {
                0.0
            }
        }
        WEIGHT_LINEAR => {
            if in_window(x) {
                (1.0 + x as f64) as f32
            } else {
                0.0
            }
        }
        WEIGHT_QUADRATIC => {
            if in_window(x) {
                (1.0 - x as f64 * x as f64) as f32
            } else {
                0.0
            }
        }
        WEIGHT_COSINE => {
            if in_window(x) {
                (((x as f64 * std::f64::consts::PI).cos() + 1.0) * 0.5) as f32
            } else {
                0.0
            }
        }
        other => panic!("unported filter shape function {other:#x}"),
    }
}

/// `0x4eab58`: construct a filter at `this`.
pub fn new(m: &mut Mem, this: u32, weight: u32, window: f32, mix: u32, default: f32) {
    // The original logs an error for a non-positive window and carries on.
    m.set_u32(this + 0x14, weight);
    m.set_u32(this + 0x18, mix);
    let n = (window as f64 / 0.03) as i32;
    m.set_i32(this + 0x1c, n);
    m.set_f32(this, window);
    m.set_f32(this + 4, (1.0f64 / window as f64) as f32);
    let times = m.alloc((n as u32) << 2);
    m.set_u32(this + 8, times);
    let values = m.alloc((n as u32) << 2);
    m.set_u32(this + 0xc, values);
    m.set_f32(this + 0x10, default);
    reset(m, this);
}

/// `0x4eac13`: set every sample to `(time, value)`.
pub fn fill(m: &mut Mem, this: u32, time: f32, value: f32) {
    let (times, values) = (m.u32(this + 8), m.u32(this + 0xc));
    for i in 0..m.i32(this + 0x1c).max(0) as u32 {
        m.set_f32(times + i * 4, time);
        m.set_f32(values + i * 4, value);
    }
}

/// `0x4eac5f`: make every sample ancient and zero.
pub fn reset(m: &mut Mem, this: u32) {
    let t = (globals::now(m) as f64 - 1e10) as f32;
    fill(m, this, t, 0.0);
}

/// `0x4eacce`: record `value` at the current time, replacing an expired or the oldest sample.
pub fn add(m: &mut Mem, this: u32, value: f32) {
    let now = globals::now(m);
    let (times, values) = (m.u32(this + 8), m.u32(this + 0xc));
    let n = m.i32(this + 0x1c);
    let window = m.f32(this);
    let (mut oldest, mut oldest_time) = (-1i32, now);
    for i in 0..n {
        let t = m.f32(times + i as u32 * 4);
        if now == t {
            m.set_f32(values + i as u32 * 4, value);
            return;
        }
        if (t as f64) < (now - window) as f64 {
            m.set_f32(times + i as u32 * 4, now);
            m.set_f32(values + i as u32 * 4, value);
            return;
        }
        if t < oldest_time {
            oldest_time = t;
            oldest = i;
        }
    }
    if oldest != -1 {
        m.set_f32(times + oldest as u32 * 4, now);
        m.set_f32(values + oldest as u32 * 4, value);
    }
    // Otherwise the original only writes an error message.
}

/// `0x4eaf98`: the weighted average of the samples inside the window (0 when there are none).
pub fn value(m: &Mem, this: u32) -> f32 {
    let (times, values) = (m.u32(this + 8), m.u32(this + 0xc));
    let (weight, mix) = (m.u32(this + 0x14), m.u32(this + 0x18));
    let (inv, default) = (m.f32(this + 4), m.f32(this + 0x10));
    let now = globals::now(m);
    let (mut sum, mut total, mut used) = (0.0f32, 0.0f32, 0);
    for i in 0..m.i32(this + 0x1c) {
        let x = ((m.f32(times + i as u32 * 4) as f64 - now as f64) * inv as f64) as f32;
        if x <= -1.0 || x as f64 > 0.001 {
            continue;
        }
        let w = call_shape(weight, x);
        let v = m.f32(values + i as u32 * 4);
        let term = if mix != 0 {
            let k = call_shape(mix, x);
            (((1.0 - k as f64) * default as f64 + k as f64 * v as f64) * w as f64 + sum as f64) as f32
        } else {
            (w as f64 * v as f64 + sum as f64) as f32
        };
        sum = term;
        total = (total as f64 + w as f64) as f32;
        used += 1;
    }
    if used == 0 {
        0.0
    } else {
        (sum as f64 / total as f64) as f32
    }
}

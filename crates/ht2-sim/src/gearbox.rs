//! Engine and automatic gearbox of `_Car_V` (`car2e.cpp`, `0x4ff150..0x500850`).
//!
//! `this` is the address of the vehicle object in [`Mem`]; fields are addressed by their byte
//! offset exactly as in the original. Known offsets:
//!
//! | offset | meaning |
//! |---|---|
//! | `+0x8cc` | index of the vehicle's `vehicle.tech` record |
//! | `+0xdc8` | forward speed |
//! | `+0xdd4` | number of wheels |
//! | `+0x1b30` | engine acceleration filter (object, see [`crate::filter`]) |
//! | `+0x1b50` | engine speed filter |
//! | `+0x1c14` | manual gearbox flag (1 = manual) |
//! | `+0x1c18` | highest gear |
//! | `+0x1c1c` | current gear (0 reverse, 1 neutral) |
//! | `+0x1c20` | gear being shifted to |
//! | `+0x1c84` | engine speed (rad/s), filtered |
//! | `+0x1c88` | engine speed at peak power |
//! | `+0x29d8` | throttle |
//! | `+0x3ebc`, `+0x3ef0`, `+0x3f8c`, `+0x3fc0`, `+0x3ff4` | `TechSystem`s (torque, fuel, ...) |
//!
//! Per-wheel arrays: `i16` at `+0x19a4`, `i32` at `+0x19d4` and `+0x1bb4`, `f32` at `+0xd20`
//! and `+0x2670`.

use crate::curves::{fn_00507360, fn_00507380, fn_00507440, fn_00507490};
use crate::{filter, globals, tech_system as ts};
use crate::mem::Mem;

const TACH_FILTER: u32 = 0x1b50;
const TS_TORQUE: u32 = 0x3ebc;
const TS_SCALE: u32 = 0x3fc0;
const TS_BRAKE: u32 = 0x3ff4;

/// `0x4ff150`: engine speed implied by the wheels that drive in gear `g` (0 for neutral).
pub fn fn_004ff150(m: &Mem, this: u32, g: i32) -> f32 {
    if g == 1 {
        return 0.0;
    }
    let factor = m.f32(this.wrapping_add(0x1c44).wrapping_add((g as u32).wrapping_mul(4)));
    let (mut sum, mut count) = (0.0f64, 0);
    for i in 0..m.i32(this + 0xdd4).max(0) as u32 {
        if m.i32(this + 0x1bb4 + 4 * i) != 0 && m.i16(this + 0x19a4 + 2 * i) == 0 {
            sum += m.f32(this + 0x1b70) as f64;
            count += 1;
        }
    }
    if count == 0 {
        return 0.0;
    }
    let r = (m.f32(this + 0xdc8) as f64 / (sum / count as f64)) * factor as f64 * 0.15915494309188485;
    let r = r as f32;
    let floor = m.f32(this + 0x1c88) as f64 * 0.05;
    if (r as f64) < floor {
        floor as f32
    } else {
        r
    }
}

/// `0x4ff210`: pick the engine speed from the drive wheels (when they grip) or the throttle (when
/// they spin or the car is in neutral), then run it through the speed filter.
pub fn fn_004ff210(m: &mut Mem, this: u32, input: f32) {
    let (mut sum, mut spinning, mut gripping, mut valid) = (0.0f64, 0, 0, 0);
    for i in 0..m.i32(this + 0xdd4).max(0) as u32 {
        if m.i32(this + 0x1bb4 + 4 * i) != 0 {
            valid += 1;
            if m.i16(this + 0x19a4 + 2 * i) != 0 {
                spinning += 1;
            }
            if m.i32(this + 0x19d4 + 4 * i) != 0 {
                sum += m.f32(this + 0x2670 + 4 * i) as f64;
                gripping += 1;
            }
        }
    }
    m.set_f32(this + 0x1cb8, 1.0);
    let (peak, throttle) = (m.f32(this + 0x1c88) as f64, m.f32(this + 0x29d8) as f64);
    let gear = m.i32(this + 0x1c1c);
    let mut v = input;
    if gear == 1 || spinning == valid {
        v = (throttle * peak * 3.0) as f32;
    } else if gripping == valid && gripping != 0 {
        let avg = (sum / gripping as f64).clamp(0.0, 1.0);
        v = ((((1.0 - avg) + avg) * 3.0) * throttle * peak) as f32;
    } else if input < 0.0 {
        m.set_f32(this + 0x1cb8, f32::from_bits(0x3e99999a));
        v = (throttle * peak * 3.0) as f32;
    } else if (input as f64) < peak * 0.5 {
        let k = input as f64 / peak;
        let k2 = k + k;
        let a = 1.0 - k2;
        let t = throttle * peak * a;
        v = (t + t + k2 * input as f64) as f32;
        m.set_f32(this + 0x1cb8, (k2 + a * 0.3) as f32);
    }
    let floor = peak * 0.05;
    if (v as f64) < floor {
        v = floor as f32;
    }
    filter::add(m, this + TACH_FILTER, v);
    let r = filter::value(m, this + TACH_FILTER);
    m.set_f32(this + 0x1c84, r);
}

/// `0x4ff3e0`: engine braking torque at engine speed `x` (0 while the clutch slips in state 1).
pub fn fn_004ff3e0(m: &Mem, this: u32, x: f32) -> f32 {
    let state = m.i32(this + 0x1c28);
    if state == 1 {
        return 0.0;
    }
    let level = ts::value(m, this + TS_BRAKE);
    let mut r = fn_00507360(x, m.f32(this + 0x1c8c), level, m.f32(this + 0x29dc)) as f64;
    if state > 1 {
        r *= 1.0 - ((state - 2) / 1000) as f64;
    }
    r as f32
}

/// `0x4ff460`: engine torque at engine speed `rpm`.
pub fn fn_004ff460(m: &Mem, this: u32, rpm: f32) -> f32 {
    let scale = ts::value(m, this + TS_SCALE);
    let torque = ts::value(m, this + TS_TORQUE);
    fn_00507380(rpm, m.f32(this + 0x1c88), torque, scale)
}

/// `0x4ff4a0`: engine torque at the current engine speed.
pub fn fn_004ff4a0(m: &Mem, this: u32) -> f32 {
    fn_004ff460(m, this, m.f32(this + 0x1c84))
}

/// `0x4ff4b0`: blend of drive torque and braking torque at engine speed `x`, weighted by `y`.
pub fn fn_004ff4b0(m: &Mem, this: u32, x: f32, y: f32) -> f32 {
    let drive = fn_004ff460(m, this, x);
    let brake = fn_004ff3e0(m, this, x);
    fn_00507440(drive, brake, y)
}

/// `0x4ff500`: `-(brake / (drive - brake))` at engine speed `x`.
pub fn fn_004ff500(m: &Mem, this: u32, x: f32) -> f32 {
    let drive = fn_004ff460(m, this, x) as f64;
    let brake = fn_004ff3e0(m, this, x) as f64;
    (-(brake / (drive - brake))) as f32
}

/// Base address of the table of `vehicle.tech` records in the original's memory.
pub const TECH_TABLE: u32 = 0x0069_7868;

/// The raw `vehicle.tech` record `id` from the table.
pub fn tech_record(m: &Mem, id: i32) -> Vec<u8> {
    m.read(TECH_TABLE.wrapping_add((id as u32).wrapping_mul(crate::tech_rec::RECORD_LEN as u32)), crate::tech_rec::RECORD_LEN)
}

/// `0x4ff570`: force at the wheels for engine torque `x` in gear `g`.
pub fn fn_004ff570(m: &Mem, this: u32, x: f32, g: i32) -> f32 {
    let rec = tech_record(m, m.i32(this + 0x8cc));
    fn_00507490(crate::TechRec::new(&rec), x, g)
}

/// `0x4ff550` (and `0x4ff530`, `0x4ff590`): wheel force for torque `x` in the current gear.
pub fn fn_004ff550(m: &Mem, this: u32, x: f32) -> f32 {
    fn_004ff570(m, this, x, m.i32(this + 0x1c1c))
}

/// `0x4ff610`: sum of `f32(+0xd20) * f32(+0x2670)` over the driven wheels that grip.
pub fn fn_004ff610(m: &Mem, this: u32) -> f32 {
    let mut sum = 0.0f64;
    for i in 0..m.i32(this + 0xdd4).max(0) as u32 {
        if m.i32(this + 0x1bb4 + 4 * i) != 0 && m.i16(this + 0x19a4 + 2 * i) == 0 {
            sum += m.f32(this + 0xd20 + 4 * i) as f64 * m.f32(this + 0x2670 + 4 * i) as f64;
        }
    }
    sum as f32
}

/// `0x4ff770`: the manual gearbox flag is set when `mode == 1`.
pub fn fn_004ff770(m: &mut Mem, this: u32, mode: i32) {
    m.set_u32(this + 0x1c14, (mode == 1) as u32);
}

/// `0x4ff790`: put the gearbox in neutral and restart the engine speed filter.
pub fn set_neutral(m: &mut Mem, this: u32) {
    m.set_i32(this + 0x1c1c, 1);
    m.set_i32(this + 0x1c24, 0);
    let t = globals::time(m);
    m.set_f32(this + 0x1c3c, t);
    let v = m.f32(this + 0x29d8) as f64 * m.f32(this + 0x1c88) as f64;
    let v = (v + v) as f32;
    m.set_f32(this + 0x1c84, v);
    filter::add(m, this + TACH_FILTER, v);
    let r = filter::value(m, this + TACH_FILTER);
    m.set_f32(this + 0x1c84, r);
}

/// `0x4ff660`: how far the drive wheels are from being able to hold the engine (`0..=0.9`;
/// 1 when the clutch is out or the engine is slow).
pub fn fn_004ff660(m: &Mem, this: u32) -> f32 {
    if m.i32(this + 0x1c1c) == 1 || m.f32(this + 0x1c84) < 1.0 {
        return 1.0;
    }
    let drive = fn_004ff4a0(m, this);
    let brake = fn_004ff3e0(m, this, m.f32(this + 0x1c84));
    let xa = fn_004ff550(m, this, drive);
    let xb = fn_004ff550(m, this, brake);
    let s = fn_004ff610(m, this);
    if (xa as f64).abs() < s as f64 || s == 0.0 {
        return 1.0;
    }
    let (xa, xb, s) = (xa as f64, xb as f64, s as f64);
    let r = if m.i32(this + 0x1c1c) == 0 {
        (xb.abs() + s) / (xa - xb).abs()
    } else {
        (s - xb) / (xa - xb)
    };
    let r = r * 0.9;
    if r > 0.9 {
        0.89999998f32
    } else if r < 0.0 {
        0.0
    } else {
        r as f32
    }
}

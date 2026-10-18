//! Engine, drivetrain and resistance curves (`0x507330..0x5077b0` in `king.exe`).
//!
//! These are the building blocks the automatic gearbox uses to decide when to shift: for a
//! gear they find the highest speed at which the engine can still overcome the resistance.
//! Arithmetic is done in `f64` and rounded to `f32` where the original stores a `float`
//! (the original computes in x87 extended precision).

use crate::TechRec;

/// `0x507330`: a saturating shape `((x+6) * x/(x+20)) / (2x+6) * y`.
pub fn fn_00507330(x: f32, y: f32) -> f32 {
    let x = x as f64;
    ((((x + 6.0) * (x / (x + 20.0))) / (x + 3.0 + x + 3.0)) * y as f64) as f32
}

/// `0x507360`: `-(a*b) - a*c*sqrt(d)`, a drag that grows with `sqrt(d)`.
pub fn fn_00507360(a: f32, b: f32, c: f32, d: f32) -> f32 {
    let (a, b, c, d) = (a as f64, b as f64, c as f64, d as f64);
    (-(a * b) - a * c * d.sqrt()) as f32
}

/// `0x507380`: engine torque at angular speed `rpm` (rad/s) for an engine whose peak is at
/// `peak`. Torque is `torque/scale`, bell-shaped around `peak` and fading out below it.
pub fn fn_00507380(rpm: f32, peak: f32, torque: f32, scale: f32) -> f32 {
    let peak_f = peak as f64;
    let mut rpm = rpm;
    let lo = (peak as f64 * 0.1) as f32;
    if rpm < lo {
        rpm = lo;
    }
    let d = ((rpm - peak) as f64).abs();
    let e = (-(d / peak_f).powf(4.0)).exp();
    let mut t = ((torque as f64 / scale as f64) * e) as f32;
    let mut factor = 1.0f64;
    if rpm < peak {
        let ratio = rpm as f64 / peak_f;
        t = ((ratio * 1.5707963f64).sin().sqrt() * t as f64) as f32;
        factor = ratio.powf(0.2);
    }
    (factor * t as f64) as f32
}

/// `0x507440`: linear blend `t*a + (1-t)*b` with `t` clamped to `0..=1`.
pub fn fn_00507440(a: f32, b: f32, t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0) as f64;
    (t * a as f64 + (1.0 - t) * b as f64) as f32
}

/// `0x507490`: force at the wheels for engine torque `x` in gear `g`: `ratio(g) * axle * x / radius`.
pub fn fn_00507490(rec: TechRec, x: f32, g: i32) -> f32 {
    (rec.gear_ratio(g) as f64 * rec.rear_axle() as f64 * x as f64 / rec.wheel_radius() as f64) as f32
}

/// `0x5074d0`: total resistance at road speed `v` with load factor `y`: an aerodynamic and rolling term.
pub fn fn_005074d0(rec: TechRec, v: f32, y: f32) -> f32 {
    let shape = fn_00507330(v, 0.1) as f64;
    let mut k = v as f64 * rec.air_press() as f64 * 0.033;
    if k > 1.0 {
        k = 1.0;
    }
    ((k + 1.0) * shape * y as f64 * 10.0 + v as f64 * rec.air_resist() as f64 * v as f64) as f32
}

/// `0x507550`: for gear `g`, the highest road speed the engine can hold against the resistance,
/// and the engine speed where it stops being able to (`.1`). `(0, 0)` if the gear does not exist.
pub fn fn_00507550(rec: TechRec, g: i32, y: f32) -> (f32, f32) {
    if !(0..=14).contains(&g) || rec.gear_ratio(g) == 0.0 || rec.rear_axle() == 0.0 {
        return (0.0, 0.0);
    }
    let r = rec.wheel_radius();
    let tau = 6.2831852f64 as f32;
    let total = rec.rear_axle() * rec.gear_ratio(g);
    let peak = rec.tach_peak();
    let mut best = 0.0f32;
    let limit = (peak as f64 * 3.0) as f32;
    let mut w = (peak as f64 * 0.5) as f32;
    while w < limit {
        let v = ((w as f64) * ((r as f64 * tau as f64) / (total as f64).abs())) as f32;
        let resistance = fn_005074d0(rec, v, y);
        let torque = fn_00507380(w, peak, rec.max_torque(), 1.0);
        let force = fn_00507490(rec, torque, g);
        if (force as f64).abs() < resistance as f64 {
            break;
        }
        if best < v {
            best = v;
        }
        w = (peak as f64 * 0.1 + w as f64) as f32;
    }
    if (w as f64) < peak as f64 * 0.6 {
        return (0.0, 0.0);
    }
    (best, w)
}

/// `0x507710`: the best forward gear (`2..=num_gears`) by `fn_00507550`: `(speed, gear, engine speed)`.
pub fn fn_00507710(rec: TechRec, y: f32) -> (f32, i32, f32) {
    let (mut best, mut best_gear, mut best_out) = (0.0f32, 0, 0.0f32);
    let mut g = 2;
    while g <= rec.num_gears() {
        let (v, out) = fn_00507550(rec, g, y);
        if best < v {
            best = v;
            best_out = out;
            best_gear = g;
        }
        g += 1;
    }
    (best, best_gear, best_out)
}

//! The driver of the computer-controlled vehicles (`SecondCar`, built on `_Car_V`; `0x4d....`).
//!
//! The driver sets the same inputs the player's keys set: the steering `+0x271c`, the throttle
//! `+0x270c` and the brake `+0x2710`, then the vehicle runs the ported step. Its own fields
//! follow the `_Car_V` object: `+0x40a8` the lane width (double), `+0x4070` flags, `+0x405c` the
//! lane number, `+0x40b8` the sideways offset from the lane (double).

use crate::mem::Mem;

/// Speed of the vehicle the driver looks at (`+0x332c`, from the recorder snapshot).
pub const SPEED: u32 = 0x332c;
pub const LANE_WIDTH: u32 = 0x40a8;
pub const FLAGS: u32 = 0x4070;
pub const STEER_INPUT: u32 = 0x271c;

/// Top speed of the vehicle with its current mass (`0x4ff5e0` → `0x507710`).
pub fn top_speed(m: &Mem, this: u32) -> f32 {
    let vid = m.i32(this + 0x8cc);
    let rec = crate::gearbox::tech_record(m, vid);
    crate::curves::fn_00507710(crate::tech_rec::TechRec::new(&rec), m.f32(this + 0x2640)).0
}

/// `0x4d74e0`: set the steering input from the wheel angle `angle` (radians): `4/pi` of it,
/// within `-1..=1`; above 10 m/s less, the faster the vehicle is (at most 0.4 either way), unless
/// `+0x274c`. Mirrored when the vehicle rolls backwards (`+0x3330 < 0`).
pub fn fn_004d74e0(m: &mut Mem, this: u32, angle: f64) {
    let mut s = (angle * 1.2732395664542882).clamp(-1.0, 1.0);
    let speed = m.f32(this + SPEED) as f64;
    if m.i32(this + 0x274c) == 0 && 10.0 <= speed {
        let fast = top_speed(m, this) as f64 * 0.7;
        s = if fast <= speed { s * 0.5 } else { (1.0 - (speed - 10.0) * 0.5 / (fast - 10.0)) * s };
        s = s.clamp(-0.4, 0.4);
    }
    if m.f32(this + 0x3330) < 0.0 {
        s = -s;
    }
    m.set_f32(this + STEER_INPUT, s as f32);
}

/// `0x4d5f50`: steer along the lane: `curvature` of the road ahead (per metre) times the wheel
/// base (`|+0x33c4 - +0x33dc|`), less a correction for the sideways `offset` from the lane
/// (in lane widths, at most half a width, ignored within 5 %, weaker at speed), less the
/// heading error `heading` (ignored below 0.1).
pub fn fn_004d5f50(m: &mut Mem, this: u32, curvature: f64, offset: f64, heading: f64) {
    let v = |o: u32| [m.f32(this + o) as f64, m.f32(this + o + 4) as f64, m.f32(this + o + 8) as f64];
    let (a, b) = (v(0x33c4), v(0x33dc));
    let d = [(a[0] - b[0]) as f32 as f64, (a[1] - b[1]) as f32 as f64, (a[2] - b[2]) as f32 as f64];
    let base = (d[2] * d[2] + d[1] * d[1] + d[0] * d[0]).sqrt();
    let mut lat = offset * if 0.0 < offset { 1.3 } else { 0.9 } / m.f64(this + LANE_WIDTH);
    if lat < -0.5 {
        lat = -0.5;
    } else if 0.5 < lat {
        lat = 0.5;
    } else if lat.abs() < 0.05 {
        lat = 0.0;
    }
    if m.u32(this + FLAGS) & 2 == 0 {
        let f = (1.0 - heading * heading).max(0.0);
        lat = f * lat * 0.5;
    }
    let sp = (m.f32(this + SPEED) as f64 * 0.100000001490116).max(0.5);
    let h = if heading.abs() < 0.1 { 0.0 } else { heading };
    fn_004d74e0(m, this, base * curvature - lat / (sp + sp) - h);
}

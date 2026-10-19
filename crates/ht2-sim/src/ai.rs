//! The driver of the computer-controlled vehicles (`SecondCar`, built on `_Car_V`; `0x4d....`).
//!
//! The driver sets the same inputs the player's keys set: the steering `+0x271c`, the throttle
//! `+0x270c` and the brake `+0x2710`, then the vehicle runs the ported step. Its own fields
//! follow the `_Car_V` object: `+0x40a8` the lane width (double), `+0x4070` flags, `+0x405c` the
//! lane number, `+0x40b8` the sideways offset from the lane (double).

use crate::mem::Mem;
use crate::road::{Place, RoadNet, RouteStep};

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


/// What `0x4cdc90` finds on the road ahead.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Ahead {
    /// Bit 1: the road turns (|curvature| above 0.0157); bit 4: the route ends within half the
    /// distance looked at.
    pub bits: u32,
    /// Turn per metre over the distance.
    pub curvature: f64,
    /// Change of the slope angle per metre.
    pub slope: f64,
    /// How far the route goes before it ends (the distance looked at when it does not).
    pub reach: f64,
}

fn flat_unit(v: [f64; 3]) -> [f64; 3] {
    let l = (v[0] * v[0] + v[1] * v[1]).sqrt();
    if l < 1e-5 {
        [0.0; 3]
    } else {
        [v[0] / l, v[1] / l, 0.0]
    }
}

fn unit(v: [f64; 3]) -> [f64; 3] {
    let l = (v[2] * v[2] + v[1] * v[1] + v[0] * v[0]).sqrt();
    if l < 1e-5 {
        [0.0; 3]
    } else {
        [v[0] / l, v[1] / l, v[2] / l]
    }
}

/// `0x4cdc90`: look `dist` metres ahead along the route (backwards along the road when `back`):
/// the directions here, a quarter of the way and at the end give the turn and the slope.
pub fn road_ahead(net: &RoadNet, p: &Place, route: &[RouteStep], dist: f64, back: bool) -> Ahead {
    let neg = |v: [f64; 3]| if back { [-v[0], -v[1], -v[2]] } else { v };
    let (d, q) = if back { (-dist, -dist * 0.25) } else { (dist, dist * 0.25) };
    let t0 = neg(net.tangent(p));
    let u0 = flat_unit(t0);
    let mut p1 = *p;
    let mut left = q;
    net.advance_route(&mut p1, &mut left, route);
    let t1 = neg(net.tangent(&p1));
    let u1 = flat_unit(t1);
    let d01 = u0[0] * u1[0] + u0[1] * u1[1] + u0[2] * u1[2];
    let mut p2 = *p;
    let mut rest = d;
    net.advance_route(&mut p2, &mut rest, route);
    let rest = rest.abs();
    let u2 = flat_unit(neg(net.tangent(&p2)));
    let d12 = u2[1] * u1[1] + u2[0] * u1[0] + u2[2] * u1[2];
    let mut c = if (d01.abs() * 4.0) < d12.abs() { d12 * 0.25 } else { d01 };
    c = c.clamp(-1.0, 1.0);
    let mut angle = c.acos();
    if t0[0] * t1[1] - t0[1] * t1[0] < 0.0 {
        angle = -angle;
    }
    let k = 4.0 / dist;
    let curvature = k * angle;
    let (n1, n0) = (unit(t1), unit(t0));
    let slope = (n1[2] - n0[2]).asin() * k;
    let mut bits = 0;
    if 0.015707963 < curvature.abs() {
        bits |= 1;
    }
    if dist * 0.5 < rest {
        bits |= 4;
    }
    Ahead { bits, curvature, slope, reach: dist - rest }
}

/// The game flags of the driver's owner (`0x43f620`) that change the target speed.
pub const OWNER_FAST_LANE: u32 = 0x4000;
pub const OWNER_CAPPED: u32 = 0x1000000;

/// `0x4d4700`: the speed the driver aims at: `base`, capped at 20 m/s for some owners, lowered
/// for the turn and the end of the route ahead (looking `5 * base` metres), when the vehicle
/// leans (`tilt`, the cosine between its up axis and the world's, below 0.8) and when it is off
/// course (`heading` above 0.1 or more than 2.1 m off the lane); `top` is used when nothing ahead
/// matters and `follow` is unset; an obstacle at `obstacle` metres (when positive) limits it to
/// `sqrt((d - 10) * 10)`, at least 5, and stops the vehicle closer than 10 m.
#[allow(clippy::too_many_arguments)]
pub fn target_speed(net: &RoadNet, p: &Place, route: &[RouteStep], owner: u32, follow: bool, mut base: f64, heading: f64, tilt: f64, lateral: f64, back: bool, top: f64, obstacle: f64) -> f64 {
    if owner & OWNER_FAST_LANE == 0 && follow && 20.0 < base {
        base = 20.0;
    }
    let mut off = 0;
    if 0.1 < heading.abs() {
        off = 2;
    }
    if 2.1 < lateral.abs() {
        off |= 4;
    }
    let ahead = road_ahead(net, p, route, base * 5.0, back);
    if ahead.bits != 0 || tilt < 0.8 || off != 0 {
        if 5.0 < base {
            if ahead.bits & 1 != 0 {
                let v = (5.0 / ahead.curvature).abs().sqrt().max(8.0);
                if v < base {
                    base = v;
                }
            }
            if ahead.bits & 2 != 0 && ahead.slope < -0.015707963 {
                base /= ahead.slope.abs() * 63.661978322714404;
            }
            if ahead.bits & 4 != 0 && ahead.reach < 50.0 {
                base = base * ahead.reach * 0.02 + 1.0;
            }
            if tilt < 0.8 {
                base = 5.0;
            } else {
                let d = (tilt - 0.8) * 5.0;
                base = (base - 5.0) * d * d + 5.0;
                if base < 5.0 {
                    base = 5.0;
                }
            }
        }
        if off != 0 && 10.0 < base {
            base *= 0.6;
        }
    } else if !follow {
        base = top;
    }
    if owner & OWNER_CAPPED != 0 && 20.0 < base {
        base = 20.0;
    }
    if 0.0 < obstacle {
        if obstacle < 10.0 {
            return 0.0;
        }
        let v = ((obstacle - 10.0) * 10.0).sqrt().max(5.0);
        if v < base {
            return v;
        }
    }
    base
}

/// `0x4f2f60`: how much of the brake the wheels can take: 1 in normal running, otherwise the
/// least ratio over the wheels on the ground of their load to what they can hold.
pub fn fn_004f2f60(m: &Mem, this: u32) -> f32 {
    if m.i32(this + 0x2638) == 1 {
        return 1.0;
    }
    let mut r = 1.0f32;
    for k in 0..m.i32(this + 0xdd4).max(0) as u32 {
        if m.i16(this + 0x19a4 + 2 * k) == 0 {
            let p = this + 0x1a7c + 4 * k;
            let f = (m.f32(p - 0x357 * 4) * m.f32(p + 0x2fd * 4) * m.f32(p)).abs();
            let hold = m.f32(p + 0x87 * 4);
            let v = if f < hold { f / hold } else { 1.0 };
            if v < r {
                r = v;
            }
        }
    }
    r
}

/// `0x4d60c0`: throttle and brake towards the target speed: full brake when the target is
/// almost 0; above it, brake by the difference (times 0.2, at most 1, scaled by `0x4f2f60`);
/// below 90 % of it, throttle by the difference (times 0.2, scaled by `0x4ff660`), full
/// throttle when slower than 10 m/s on a slope over 9 degrees, and only 0.1 while steering with
/// a wheel flag at `+0x346c..+0x3478` set.
pub fn fn_004d60c0(m: &mut Mem, this: u32, target: f64, slope: f64) {
    let brake_k = fn_004f2f60(m, this) as f64;
    let drive_k = crate::gearbox::fn_004ff660(m, this) as f64;
    m.set_u32(this + 0x2710, 0);
    m.set_u32(this + 0x270c, 0);
    let speed = m.f32(this + SPEED) as f64;
    if target < 0.1 {
        m.set_f32(this + 0x2710, 1.0);
        return;
    }
    if target < speed {
        let b = ((speed - target) * 0.2).min(1.0);
        m.set_f32(this + 0x2710, (b * brake_k) as f32);
        return;
    }
    if speed < target * 0.9 {
        let t = ((target - speed) * 0.2 * drive_k) as f32;
        m.set_f32(this + 0x270c, t);
        if speed < 10.0 && 0.15707963 < slope {
            m.set_f32(this + 0x270c, 1.0);
            return;
        }
        let flagged = (0..4).any(|k| m.i32(this + 0x346c + 4 * k) != 0);
        if flagged && 0.1 < m.f32(this + STEER_INPUT).abs() && 0.1 < t {
            m.set_f32(this + 0x270c, 0.1);
        }
    }
}

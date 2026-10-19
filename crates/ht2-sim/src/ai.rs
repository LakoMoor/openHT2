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
    let (d, q) = if back { (-dist, -dist * 0.25) } else { (dist, dist * 0.25) };
    let t0 = net.direction(p, back);
    let u0 = flat_unit(t0);
    let mut p1 = *p;
    let mut left = q;
    net.advance_route(&mut p1, &mut left, route);
    let t1 = net.direction(&p1, back);
    let u1 = flat_unit(t1);
    let d01 = u0[0] * u1[0] + u0[1] * u1[1] + u0[2] * u1[2];
    let mut p2 = *p;
    let mut rest = d;
    net.advance_route(&mut p2, &mut rest, route);
    let rest = rest.abs();
    let u2 = flat_unit(net.direction(&p2, back));
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

/// `0x4d4620`: the lanes of the half of the road the vehicle is on (`width` metres): one per
/// 3.5 m (rounded), each `+0x40a8` wide; the lane to take is the preferred one `+0x407c`
/// (random 0..3 when the driver is made, `0x4d2890`) or the outermost one. Owners flagged
/// 0x80000 or 0x8000000 (`0x43f620`) take one lane further out; the 0x4000 owners pick it in
/// `0x4d43a0` (not ported: they take the usual lane).
pub fn fn_004d4620(m: &mut Mem, this: u32, owner: u32, width: f64) -> i32 {
    let n = ((width + 1.75) * 0.285714286) as i32;
    let mut lane = 0;
    if n != 0 {
        lane = n - 1;
        m.set_f64(this + LANE_WIDTH, width / n as f64);
    } else {
        m.set_f64(this + LANE_WIDTH, width);
    }
    if owner & (0x80000 | 0x8000000) != 0 {
        return lane + 1;
    }
    m.i32(this + 0x407c).min(lane)
}

/// The driver's mode `+0x406c`: 0 stands, 1 drives along the lane, 2 backs off after being
/// stuck, 3 goes round an obstacle.
pub const MODE: u32 = 0x406c;
/// `+0x4074`: the vehicle drives against the direction of its place.
pub const BACK: u32 = 0x4074;
/// `+0x405c`: twice the lane the driver keeps to.
pub const LANE: u32 = 0x405c;
/// `+0x40b0`: distance to the obstacle ahead (negative for none).
pub const OBSTACLE: u32 = 0x40b0;
/// `+0x40b8`: sideways offset from the lane (double).
pub const LATERAL: u32 = 0x40b8;

/// `0x4d2890`: the fields of a driver when it is made: the preferred lane (`rand % 4`), a 3.5 m
/// lane, nothing followed.
pub fn init_driver(m: &mut Mem, this: u32, rand: u32) {
    let lane = (rand % 4) as i32;
    m.set_i32(this + 0x407c, lane);
    m.set_i32(this + LANE, lane * 2);
    m.set_f64(this + LANE_WIDTH, 3.5);
    m.set_u32(this + 0x4078, 0);
    m.set_i32(this + 0x4084, -1);
    m.set_i32(this + 0x4080, -1);
    m.set_u32(this + 0x4064, 0);
    // Not from 0x4d2890: the obstacle search that would set these is not ported.
    m.set_f64(this + OBSTACLE, -1.0);
    m.set_u32(this + MODE, 0);
    m.set_u32(this + 0x4068, 0);
    m.set_u32(this + 0x402c, 0);
    m.set_u32(this + BACK, 0);
}

/// What the driver of a computer vehicle knows about its way: the place on the road network
/// its front is at and the route ahead. The original keeps both on its owner (`+0x509c`, the
/// `PlayerId` of the road module); here the route is picked by [`Driver::extend`].
#[derive(Debug, Clone)]
pub struct Driver {
    pub place: Place,
    pub route: Vec<RouteStep>,
    seed: u32,
}

fn f3(m: &Mem, a: u32) -> [f64; 3] {
    [m.f32(a) as f64, m.f32(a + 4) as f64, m.f32(a + 8) as f64]
}

impl Driver {
    pub fn new(place: Place, seed: u32) -> Self {
        let mut d = Driver { place, route: Vec::new(), seed: seed | 1 };
        d.place.node = None;
        d
    }

    fn rand(&mut self) -> u32 {
        self.seed ^= self.seed << 13;
        self.seed ^= self.seed >> 17;
        self.seed ^= self.seed << 5;
        self.seed
    }

    /// Make the route reach `dist` metres past the place. Stand-in: at each node a random road
    /// among those going on within 70 degrees of the way in (the straightest one when there is
    /// none), never the one arriving (the planner of the road module is not ported).
    pub fn extend(&mut self, net: &RoadNet, dist: f64) {
        let mut p = self.place;
        let mut left = dist;
        for _ in 0..8 {
            net.advance_route(&mut p, &mut left, &self.route);
            let Some(node) = p.node else { return };
            if left == 0.0 || self.route.iter().any(|s| s.node == node) {
                return;
            }
            let n = &net.nodes[node];
            let roads: Vec<usize> = n.out.iter().chain(n.inc.iter()).copied().filter(|&r| r != p.road).collect();
            let flat = |v: [f64; 3]| {
                let l = (v[0] * v[0] + v[1] * v[1]).sqrt().max(1e-9);
                [v[0] / l, v[1] / l]
            };
            let way_in = flat(net.direction(&p, false));
            let out = |r: usize| {
                let q = if net.roads[r].start == node { net.road_start(r) } else { net.road_end(r) };
                let d = flat(net.direction(&Place { node: None, ..q }, false));
                d[0] * way_in[0] + d[1] * way_in[1]
            };
            let ahead: Vec<usize> = roads.iter().copied().filter(|&r| 0.34 < out(r)).collect();
            let road = if !ahead.is_empty() {
                ahead[self.rand() as usize % ahead.len()]
            } else {
                roads.iter().copied().max_by(|&a, &b| out(a).total_cmp(&out(b))).unwrap_or(p.road)
            };
            self.route.push(RouteStep { node, road: Some(road) });
        }
    }

    /// Follow the point `q` along the route (stand-in for `0x43bf10`, which finds the place of
    /// a point from the place of the owner): step along the road by how far `q` is ahead.
    pub fn track(&mut self, net: &RoadNet, q: [f64; 3]) {
        for _ in 0..4 {
            let c = net.coordinates(&self.place);
            let t = net.direction(&self.place, false);
            let l = (t[0] * t[0] + t[1] * t[1]).sqrt();
            if l < 1e-9 {
                return;
            }
            let mut d = ((q[0] - c[0]) * t[0] + (q[1] - c[1]) * t[1]) / l;
            if d.abs() < 0.01 {
                break;
            }
            let road = self.place.road;
            net.advance_route(&mut self.place, &mut d, &self.route);
            self.place.node = None;
            if self.place.road != road {
                // Forget the steps passed.
                if let Some(k) = self.route.iter().position(|s| s.road == Some(self.place.road)) {
                    self.route.drain(..=k);
                }
            }
        }
    }
}

/// `SecondCar::run` (`0x4d6510`) up to the vehicle step: sets the inputs from the road ahead.
/// `owner` are the flags of the owner (`0x43f620`), `dt` the frame time. Not ported: the
/// obstacle search `0x4d6340` / `0x4d57c0` (other vehicles: no obstacle, the lane is the one of
/// `0x4d4620`), so mode 3 and the checks of `0x4d3880`; the vehicle the camera follows and
/// owners flagged 4 (full brake) are not handled.
pub fn drive(m: &mut Mem, this: u32, net: &RoadNet, drv: &mut Driver, owner: u32, dt: f64) {
    let brake_k = fn_004f2f60(m, this) as f64;
    let drive_k = crate::gearbox::fn_004ff660(m, this) as f64;
    let back = m.u32(this + BACK) != 0;
    // The front (the back when reversing) of the vehicle: half its length (tech +0x554) from
    // its origin along its forward axis (the original starts from a point of the owner,
    // `+0x5230`).
    let vid = m.i32(this + 0x8cc) as u32;
    let len = m.f32(crate::gearbox::TECH_TABLE + vid.wrapping_mul(0x970) + 0x554) as f64;
    let half = len * if back { -0.5 } else { 0.5 };
    let fwd = f3(m, this + 0x1c);
    let pos = f3(m, this + 0x34);
    let front = [pos[0] + fwd[0] * half, pos[1] + fwd[1] * half, pos[2] + fwd[2] * half];
    drv.extend(net, 400.0);
    drv.track(net, front);
    drv.extend(net, 400.0);
    let p = drv.place;
    let route = drv.route.clone();
    let (side, width, _, _) = net.lateral(&p, back, front);
    m.set_f64(this + LATERAL, side);
    let (road, slope) = net.heading(&p, back);
    let lane = fn_004d4620(m, this, owner, width);
    m.set_i32(this + LANE, lane * 2);
    let obstacle = -1.0f64;
    let mut base = 30.0;
    if 0.0 < obstacle && obstacle < 30.0 && m.u32(this + 0x4078) != 0 {
        base = obstacle;
    }
    let curv = net.curvature(&p, 15.0, back, &route);
    let lw = m.f64(this + LANE_WIDTH);
    let lat = (m.i32(this + LANE) + 1) as f64 * lw * 0.5 + m.f64(this + LATERAL);
    m.set_f64(this + LATERAL, lat);
    // The flat forward direction `+0x4050`; the heading error is the sine of the angle from
    // the road to it.
    let fl = (fwd[0] * fwd[0] + fwd[1] * fwd[1]).sqrt().max(1e-9);
    let dir = [fwd[0] / fl, fwd[1] / fl];
    m.set_f32(this + 0x4050, dir[0] as f32);
    m.set_f32(this + 0x4054, dir[1] as f32);
    m.set_f32(this + 0x4058, 0.0);
    let heading = (road[0] * dir[1] - road[1] * dir[0]).clamp(-0.9999, 0.9999).asin();
    if 0.0 < obstacle {
        if m.u32(this + MODE) == 1 {
            m.set_f64(this + 0x4048, lat);
            m.set_u32(this + MODE, 3);
        }
    } else if m.u32(this + MODE) == 3 {
        m.set_i16(this + 0x2746, 100);
        m.set_u32(this + MODE, 1);
    }
    // Stuck: slower than 2 m/s with nothing ahead for 5 s, then back off for 3 s.
    let speed = m.f32(this + SPEED) as f64;
    if speed < 2.0 && m.u32(this + MODE) != 2 && m.u32(this + FLAGS) & 8 == 0 && m.f64(this + OBSTACLE) < 0.0 {
        if m.u32(this + 0x402c) == 0 {
            m.set_f64(this + 0x4038, 0.0);
            m.set_u32(this + 0x402c, 1);
        } else if 5.0 < m.f64(this + 0x4038) {
            m.set_u32(this + 0x402c, 0);
            m.set_f64(this + 0x4038, 3.0);
            m.set_i16(this + 0x2746, 0x14);
            m.set_u32(this + MODE, 2);
        } else {
            let t = m.f64(this + 0x4038) + dt;
            m.set_f64(this + 0x4038, t);
        }
    } else {
        m.set_u32(this + 0x402c, 0);
    }
    if owner & 0x8000000 != 0 {
        if lat.abs() < 0.5 {
            let f = m.u32(this + 0x4068) | 4;
            m.set_u32(this + 0x4068, f);
        }
    } else {
        let f = m.u32(this + 0x4068) & !4;
        m.set_u32(this + 0x4068, f);
    }
    if m.u32(this + 0x4068) != 0 {
        m.set_u32(this + MODE, 0);
    } else if m.u32(this + MODE) == 0 {
        m.set_u32(this + MODE, 1);
    }
    match m.u32(this + MODE) {
        0 => {
            fn_004d74e0(m, this, 0.0);
            m.set_u32(this + 0x270c, 0);
            m.set_f32(this + 0x2710, brake_k as f32);
        }
        1 => {
            let f = m.u32(this + FLAGS);
            if f & 2 != 0 && f & 8 != 0 && owner & 0x4000 == 0 {
                m.set_f64(this + OBSTACLE, 9.0);
            }
            if f & 8 != 0 {
                fn_004d5f50(m, this, 0.0, 0.0, 0.0);
            } else {
                fn_004d5f50(m, this, curv, lat, heading);
            }
            // The cosine between the up axes of the vehicle and of the world.
            let tilt = m.f32(this + 0x30) as f64;
            let top = top_speed(m, this) as f64;
            let follow = m.u32(this + 0x4078) != 0;
            let target = target_speed(net, &p, &route, owner, follow, base, heading, tilt, lat, back, top, m.f64(this + OBSTACLE));
            fn_004d60c0(m, this, target, slope);
        }
        2 => {
            fn_004d74e0(m, this, 0.0);
            let x = m.f64(this + 0x4038).min(1.0);
            if x * 8.0 < speed {
                m.set_u32(this + 0x270c, 0);
                m.set_f32(this + 0x2710, brake_k as f32);
            }
            if speed < x * 4.0 {
                m.set_u32(this + 0x2710, 0);
                m.set_f32(this + 0x270c, drive_k as f32);
            }
            let t = m.f64(this + 0x4038) - dt;
            m.set_f64(this + 0x4038, t);
            // (The original also leaves when 0x4d3880 finds the way clear.)
            if t < 0.0 {
                m.set_i16(this + 0x2746, 100);
                m.set_u32(this + MODE, 1);
            }
        }
        _ => {}
    }
}

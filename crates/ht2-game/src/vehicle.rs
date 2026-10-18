//! Vehicle: engine, gearbox and brakes use the real `vehicle.tech` numbers; steering is a
//! kinematic bicycle model and the body rests on the ground mesh. This is not the original
//! game physics, which has not been reverse engineered yet.

use crate::ground::Ground;
use glam::{Mat4, Vec3};
use ht2_formats::tech::Tech;

const G: f32 = 9.81;
const GRIP: f32 = 0.9;
const DRIVETRAIN_EFFICIENCY: f32 = 0.85;
const MAX_REVERSE: f32 = -8.0;

#[derive(Default, Clone, Copy)]
pub struct Input {
    /// -1..1, positive steers left.
    pub steer: f32,
    pub throttle: f32,
    pub brake: f32,
}

/// Everything the simulation needs to know about one vehicle type.
#[derive(Clone)]
pub struct Params {
    pub mass: f32,
    pub wheel_radius: f32,
    /// Ratios of gears 1..=n.
    pub gears: Vec<f32>,
    pub axle_ratio: f32,
    pub reverse_ratio: f32,
    pub max_torque: f32,
    pub peak_power_w: f32,
    pub brake_force: f32,
    /// Cd * frontal area, m^2 (estimated from the file's single air resistance value).
    pub drag_area: f32,
    /// Front axle and mean rear axle position along the model's Y axis, and half the track.
    pub front_y: f32,
    pub rear_y: f32,
    pub half_track: f32,
}

impl Params {
    pub fn from_tech(t: &Tech, front_y: f32, rear_y: f32, half_track: f32) -> Self {
        let n = (t.num_gears as usize).clamp(1, 13);
        Params {
            mass: t.mass,
            wheel_radius: t.wheel_radius,
            gears: t.gear_ratios[1..=n].to_vec(),
            axle_ratio: t.rear_axle_ratio,
            reverse_ratio: t.reverse_ratio.abs(),
            max_torque: t.max_torque,
            peak_power_w: t.horse_power as f32 * 745.7,
            brake_force: t.brake_power,
            drag_area: t.air_resistance * 16.0,
            front_y,
            rear_y,
            half_track,
        }
    }

    pub fn wheelbase(&self) -> f32 {
        self.front_y - self.rear_y
    }
}

pub struct Vehicle {
    pub params: Params,
    pub pos: Vec3,
    /// Radians; 0 faces +Y, positive turns left.
    pub heading: f32,
    pub pitch: f32,
    pub roll: f32,
    /// Metres per second along the heading.
    pub speed: f32,
    /// 1-based forward gear.
    pub gear: usize,
    steer: f32,
    /// Distance from the model origin to its lowest point.
    pub ride_height: f32,
}

impl Vehicle {
    pub fn new(params: Params, pos: Vec3, heading: f32, ride_height: f32) -> Self {
        Self { params, pos, heading, pitch: 0.0, roll: 0.0, speed: 0.0, gear: 1, steer: 0.0, ride_height }
    }

    pub fn forward(&self) -> Vec3 {
        Vec3::new(-self.heading.sin(), self.heading.cos(), 0.0)
    }

    fn total_ratio(&self, gear: usize) -> f32 {
        self.params.gears[gear - 1] * self.params.axle_ratio
    }

    /// Engine speed in rpm for the current road speed in `gear`.
    pub fn rpm_in(&self, gear: usize) -> f32 {
        self.speed.abs() / self.params.wheel_radius * self.total_ratio(gear) * 60.0 / std::f32::consts::TAU
    }

    pub fn rpm(&self) -> f32 {
        self.rpm_in(self.gear).max(900.0)
    }

    /// Torque at the crankshaft, N*m: flat in the middle, weaker at the ends, limited by power.
    fn torque(&self, rpm: f32) -> f32 {
        let shape = if rpm < 900.0 {
            0.6
        } else if rpm < 1800.0 {
            1.0
        } else {
            (1.0 - (rpm - 1800.0) / 1500.0).max(0.3)
        };
        let omega = rpm * std::f32::consts::TAU / 60.0;
        (self.params.max_torque * shape).min(self.params.peak_power_w / omega)
    }

    pub fn step(&mut self, input: Input, ground: &Ground, dt: f32) {
        let p = &self.params;
        // Steering eases towards the target and has tighter limits at speed.
        let limit = 0.55 / (1.0 + self.speed.abs() * 0.08);
        self.steer += (input.steer * limit - self.steer) * (6.0 * dt).min(1.0);

        // Automatic gearbox.
        if self.rpm_in(self.gear) > 1950.0 && self.gear < p.gears.len() {
            self.gear += 1;
        } else if self.rpm_in(self.gear) < 1100.0 && self.gear > 1 {
            self.gear -= 1;
        }

        let traction = GRIP * p.mass * G;
        let reversing = input.brake > 0.0 && self.speed < 0.5;
        let drive = if reversing {
            -(self.torque(self.rpm()) * 0.8 * p.reverse_ratio * p.axle_ratio * DRIVETRAIN_EFFICIENCY / p.wheel_radius * input.brake)
                .min(traction)
        } else if self.speed >= -0.1 {
            (input.throttle * self.torque(self.rpm()) * self.total_ratio(self.gear) * DRIVETRAIN_EFFICIENCY / p.wheel_radius)
                .min(traction)
        } else {
            0.0
        };
        let braking = if input.brake > 0.0 && self.speed > 0.5 { input.brake * p.brake_force.min(traction) } else { 0.0 };
        let resist = 0.5 * 1.2 * p.drag_area * self.speed * self.speed.abs() + 0.012 * p.mass * G * self.speed.signum();
        let accel = (drive - braking - resist) / p.mass;
        self.speed = (self.speed + accel * dt).max(MAX_REVERSE);
        if input.throttle == 0.0 && input.brake == 0.0 && self.speed.abs() < 0.2 {
            self.speed = 0.0;
        }

        self.heading += self.speed / p.wheelbase() * self.steer.tan() * dt;
        let fwd = self.forward();
        self.pos.x += fwd.x * self.speed * dt;
        self.pos.y += fwd.y * self.speed * dt;
        self.settle(ground);
    }

    /// Put the four wheel contact points on the ground and derive height, pitch and roll.
    fn settle(&mut self, ground: &Ground) {
        let fwd = self.forward();
        let right = Vec3::new(fwd.y, -fwd.x, 0.0);
        let (fy, ry, ht) = (self.params.front_y, self.params.rear_y, self.params.half_track);
        let at = |along: f32, side: f32, me: &Self| {
            let p = me.pos + fwd * along + right * side;
            ground.height_at(p.x, p.y, me.pos.z, 1.0)
        };
        let samples = [at(fy, -ht, self), at(fy, ht, self), at(ry, -ht, self), at(ry, ht, self)];
        let known: Vec<f32> = samples.iter().flatten().copied().collect();
        if known.is_empty() {
            // Off the map: fall.
            self.pos.z -= 0.2;
            return;
        }
        let z = |i: usize| samples[i].unwrap_or(known.iter().sum::<f32>() / known.len() as f32);
        let (fl, fr, rl, rr) = (z(0), z(1), z(2), z(3));
        // The origin sits between the axles, not necessarily in the middle.
        let target = (fl + fr + rl + rr) / 4.0;
        self.pos.z += (target - self.pos.z) * 0.5;
        self.pitch = ((fl + fr) / 2.0 - (rl + rr) / 2.0).atan2(fy - ry);
        self.roll = ((fl + rl) / 2.0 - (fr + rr) / 2.0).atan2(ht * 2.0);
    }

    /// Model matrix for a model with +Y forward and +Z up.
    pub fn model(&self) -> Mat4 {
        Mat4::from_translation(self.pos + Vec3::Z * self.ride_height)
            * Mat4::from_rotation_z(self.heading)
            * Mat4::from_rotation_x(self.pitch)
            * Mat4::from_rotation_y(self.roll)
    }
}

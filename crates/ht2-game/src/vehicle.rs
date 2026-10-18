//! Arcade vehicle: kinematic bicycle model on the ground mesh. Not the original physics.

use crate::ground::Ground;
use glam::{Mat4, Vec3};

#[derive(Default, Clone, Copy)]
pub struct Input {
    /// -1..1, positive steers left.
    pub steer: f32,
    pub throttle: f32,
    pub brake: f32,
}

pub struct Vehicle {
    pub pos: Vec3,
    /// Radians; 0 faces +Y, positive turns left.
    pub heading: f32,
    pub pitch: f32,
    pub roll: f32,
    /// Metres per second along the heading.
    pub speed: f32,
    steer: f32,
    /// Distance from the model origin to its lowest point.
    pub ride_height: f32,
}

const WHEELBASE: f32 = 5.0;
const HALF_TRACK: f32 = 1.2;
const MAX_SPEED: f32 = 28.0;
const MAX_REVERSE: f32 = -8.0;

impl Vehicle {
    pub fn new(pos: Vec3, heading: f32, ride_height: f32) -> Self {
        Self { pos, heading, pitch: 0.0, roll: 0.0, speed: 0.0, steer: 0.0, ride_height }
    }

    pub fn forward(&self) -> Vec3 {
        Vec3::new(-self.heading.sin(), self.heading.cos(), 0.0)
    }

    pub fn step(&mut self, input: Input, ground: &Ground, dt: f32) {
        // Steering eases towards the target and gets tighter limits at speed.
        let limit = 0.55 / (1.0 + self.speed.abs() * 0.08);
        self.steer += (input.steer * limit - self.steer) * (6.0 * dt).min(1.0);

        let drive = if self.speed >= -0.1 { input.throttle * 5.0 } else { 0.0 };
        let reverse = if input.brake > 0.0 && self.speed < 0.5 { -input.brake * 3.0 } else { 0.0 };
        let braking = if input.brake > 0.0 && self.speed > 0.5 { input.brake * 12.0 } else { 0.0 };
        let drag = 0.015 * self.speed * self.speed.abs() + 0.4 * self.speed.signum();
        self.speed += (drive + reverse - braking - drag) * dt;
        if input.throttle == 0.0 && input.brake == 0.0 && self.speed.abs() < 0.3 {
            self.speed = 0.0;
        }
        self.speed = self.speed.clamp(MAX_REVERSE, MAX_SPEED);

        self.heading += self.speed / WHEELBASE * self.steer.tan() * dt;
        let fwd = self.forward();
        self.pos.x += fwd.x * self.speed * dt;
        self.pos.y += fwd.y * self.speed * dt;
        self.settle(ground);
    }

    /// Put the four wheels on the ground and derive height, pitch and roll from them.
    fn settle(&mut self, ground: &Ground) {
        let fwd = self.forward();
        let right = Vec3::new(fwd.y, -fwd.x, 0.0);
        let at = |along: f32, side: f32, me: &Self| {
            let p = me.pos + fwd * along + right * side;
            ground.height_at(p.x, p.y, me.pos.z, 1.0)
        };
        let samples = [
            at(WHEELBASE / 2.0, -HALF_TRACK, self),
            at(WHEELBASE / 2.0, HALF_TRACK, self),
            at(-WHEELBASE / 2.0, -HALF_TRACK, self),
            at(-WHEELBASE / 2.0, HALF_TRACK, self),
        ];
        let known: Vec<f32> = samples.iter().flatten().copied().collect();
        if known.is_empty() {
            // Off the map: fall.
            self.pos.z -= 0.2;
            return;
        }
        let z = |i: usize| samples[i].unwrap_or(known.iter().sum::<f32>() / known.len() as f32);
        let (fl, fr, rl, rr) = (z(0), z(1), z(2), z(3));
        let target = (fl + fr + rl + rr) / 4.0;
        self.pos.z += (target - self.pos.z) * 0.5;
        self.pitch = ((fl + fr) / 2.0 - (rl + rr) / 2.0).atan2(WHEELBASE);
        self.roll = ((fl + rl) / 2.0 - (fr + rr) / 2.0).atan2(HALF_TRACK * 2.0);
    }

    /// Model matrix for a model whose origin is at its centre, +Y forward, +Z up.
    pub fn model(&self) -> Mat4 {
        Mat4::from_translation(self.pos + Vec3::Z * self.ride_height)
            * Mat4::from_rotation_z(self.heading)
            * Mat4::from_rotation_x(self.pitch)
            * Mat4::from_rotation_y(self.roll)
    }
}

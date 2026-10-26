//! The player's vehicle. Normally the whole vehicle runs on the code ported from the original
//! ([`crate::simcar`]: constructor and step, verified against it in the emulator), colliding with
//! the ground mesh. With `HT2_ARCADE` set (or when the ported vehicle cannot be built) the old
//! stand-in is used: the ported engine, gearbox, fuel and steering input ([`crate::powertrain`])
//! on a kinematic bicycle model resting on the ground mesh.

use crate::ground::Ground;
use crate::powertrain::Powertrain;
use crate::simcar::{Pose, SimCar, WheelNodes};
use glam::{Mat4, Vec3};
use ht2_formats::tech::Tech;

const G: f32 = 9.81;
const GRIP: f32 = 0.9;
const MAX_REVERSE: f32 = -8.0;
/// `fn_004f0090`: the front wheels turn up to this angle at full steering (pi/4).
const MAX_STEER_ANGLE: f32 = 0.785398185;

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
    /// Index of the vehicle in `vehicle.tech`.
    pub record: usize,
    pub mass: f32,
    pub wheel_radius: f32,
    pub brake_force: f32,
    /// Cd * frontal area, m^2 (estimated from the file's single air resistance value).
    pub drag_area: f32,
    /// Front axle and mean rear axle position along the model's Y axis, and half the track.
    pub front_y: f32,
    pub rear_y: f32,
    pub half_track: f32,
    pub wheels: usize,
    pub driven: Vec<bool>,
    /// The model's `<prefix>Wheel%dSpace` and `<prefix>AddWheel%dSpace` transforms.
    pub wheel_nodes: WheelNodes,
    pub add_nodes: WheelNodes,
}

impl Params {
    pub fn from_tech(t: &Tech, wheels: usize, front_y: f32, rear_y: f32, half_track: f32, wheel_nodes: WheelNodes, add_nodes: WheelNodes) -> Self {
        Params {
            record: t.index,
            mass: t.mass,
            wheel_radius: t.wheel_radius,
            brake_force: t.brake_power,
            drag_area: t.air_resistance * 16.0,
            front_y,
            rear_y,
            half_track,
            wheels,
            driven: t.driving_wheels[..wheels.min(8)].to_vec(),
            wheel_nodes,
            add_nodes,
        }
    }

    pub fn wheelbase(&self) -> f32 {
        self.front_y - self.rear_y
    }
}

pub struct Vehicle {
    pub params: Params,
    powertrain: Powertrain,
    sim: Option<SimCar>,
    tech_file: std::rc::Rc<Vec<u8>>,
    pub pos: Vec3,
    /// Radians; 0 faces +Y, positive turns left.
    pub heading: f32,
    pub pitch: f32,
    pub roll: f32,
    /// Metres per second along the heading.
    pub speed: f32,
    /// 0 reverse, 1 neutral, 2.. forward gears.
    pub gear: i32,
    /// Engine speed, revolutions per second.
    pub engine_rev: f32,
    /// Fuel left, litres.
    pub fuel: f32,
    /// Diagnostics from the last powertrain step.
    pub last: Option<crate::powertrain::Output>,
    /// Accumulated wheel rotation in radians (grows when driving forward).
    pub wheel_spin: f32,
    steer_angle: f32,
    /// Distance from the model origin to its lowest point.
    pub ride_height: f32,
}

impl Vehicle {
    pub fn new(params: Params, tech_file: std::rc::Rc<Vec<u8>>, pos: Vec3, heading: f32, ride_height: f32, ground: &Ground) -> Self {
        let powertrain = Powertrain::new(&tech_file, params.record, params.wheels, &params.driven, params.mass)
            .expect("vehicle.tech has the expected layout");
        let sim = if std::env::var_os("HT2_ARCADE").is_some() {
            None
        } else {
            match SimCar::new(&tech_file, params.record, &params.wheel_nodes, &params.add_nodes, ground, pos, heading) {
                Ok(s) => Some(s),
                Err(e) => {
                    eprintln!("ported vehicle not available ({e}), using the stand-in");
                    None
                }
            }
        };
        let mut v = Self {
            params,
            powertrain,
            sim,
            tech_file,
            pos,
            heading,
            pitch: 0.0,
            roll: 0.0,
            speed: 0.0,
            gear: 1,
            engine_rev: 0.0,
            fuel: 0.0,
            last: None,
            wheel_spin: 0.0,
            steer_angle: 0.0,
            ride_height,
        };
        v.sync();
        v
    }

    /// A fresh vehicle of the same type (used when the player resets).
    pub fn respawned(&self, pos: Vec3, heading: f32, ground: &Ground) -> Self {
        Self::new(self.params.clone(), self.tech_file.clone(), pos, heading, self.ride_height, ground)
    }

    /// The room the ported vehicle is in (0 for none) and whether rooms are used at all.
    pub fn room(&self) -> Option<(u32, bool)> {
        self.sim.as_ref().map(|s| (s.room(), s.uses_rooms()))
    }

    /// For diagnostics: the room whose road is below the ported vehicle.
    pub fn room_below(&mut self, ground: &Ground) -> Option<u32> {
        self.sim.as_mut().and_then(|s| s.room_below(ground))
    }

    /// Let the ported computer driver drive this vehicle.
    pub fn set_autopilot(&mut self, ground: &Ground) {
        if let Some(s) = &mut self.sim {
            s.set_autopilot(ground);
        }
    }

    /// Put `count` computer vehicles on the roads around (ported physics only).
    pub fn add_traffic(&mut self, ground: &Ground, count: usize, seed: u32) {
        if let Some(s) = &mut self.sim {
            s.add_traffic(ground, count, seed);
        }
    }

    /// Let the AI world `w` place the computer vehicles (`pool` of them at most).
    pub fn set_ai_world(&mut self, w: ht2_sim::world::World, pool: usize, models: &[(usize, String, crate::simcar::WheelNodes, crate::simcar::WheelNodes)]) {
        if let Some(s) = &mut self.sim {
            s.set_ai_world(w, pool, models);
        }
    }

    /// Pending original game callbacks; no movie/menu request is silently discarded.
    pub fn pending_game_actions(&self) -> &[ht2_sim::game_message::Action] {
        self.sim.as_ref().map_or(&[], |s| s.pending_game_actions())
    }
    pub fn rating_world(&self) -> Option<&ht2_sim::world::World> {
        self.sim.as_ref().and_then(|s| s.rating_world())
    }

    /// The computer vehicles, for drawing.
    pub fn traffic(&self) -> Vec<Pose> {
        self.sim.as_ref().map_or_else(Vec::new, |s| s.traffic())
    }

    /// For diagnostics: the state of computer vehicle `k`.
    pub fn traffic_state(&self, k: usize) -> Option<String> {
        self.sim.as_ref().and_then(|s| s.traffic_state(k))
    }

    /// Whether the vehicle runs on the ported code.
    pub fn is_ported(&self) -> bool {
        self.sim.is_some()
    }

    /// Copy the state of the ported vehicle into the fields the rest of the game reads.
    fn sync(&mut self) {
        let Some(s) = &self.sim else { return };
        let body = s.body();
        let fwd = body.y_axis.truncate();
        self.pos = s.pos();
        self.heading = (-fwd.x).atan2(fwd.y);
        self.speed = s.speed();
        self.gear = s.gear();
        self.engine_rev = s.engine_rev();
        self.fuel = s.fuel();
        self.steer_angle = s.steer_angle();
    }

    /// Rotation of wheel `k` in radians.
    pub fn wheel_angle(&self, k: usize) -> f32 {
        self.sim.as_ref().map_or(self.wheel_spin, |s| s.wheel_angle(k))
    }

    /// How far wheel `k` hangs below where the model draws it (suspension travel).
    pub fn wheel_drop(&self, k: usize) -> f32 {
        self.sim.as_ref().map_or(0.0, |s| s.wheel_drop(k))
    }

    pub fn forward(&self) -> Vec3 {
        Vec3::new(-self.heading.sin(), self.heading.cos(), 0.0)
    }

    /// Current steering angle of the front wheels in radians, positive to the left.
    pub fn steer_angle(&self) -> f32 {
        self.steer_angle
    }

    pub fn step(&mut self, input: Input, ground: &Ground, dt: f32) {
        if let Some(s) = &mut self.sim {
            let keys = if input.steer > 0.5 { 1 } else if input.steer < -0.5 { -1 } else { 0 };
            s.step(ground, dt, input.throttle, input.brake, keys);
            self.sync();
            return;
        }
        let p = &self.params;
        self.powertrain.set_steer_keys(if input.steer > 0.5 { 1 } else if input.steer < -0.5 { -1 } else { 0 });
        // In reverse the "brake" key is the gas pedal.
        let reversing = self.gear == 0 && input.throttle == 0.0;
        let pedal = if reversing { input.brake } else { input.throttle };
        let out = self.powertrain.step(dt, pedal, input.brake, self.speed);
        self.gear = out.gear;
        self.engine_rev = out.engine_rev;
        self.fuel = out.fuel;
        self.last = Some(out);
        self.steer_angle = MAX_STEER_ANGLE * self.powertrain.steering();

        let traction = GRIP * p.mass * G;
        let drive = out.drive_force.clamp(-traction, traction);
        let braking = if input.brake > 0.0 && !reversing && self.speed > 0.5 { input.brake * p.brake_force.min(traction) } else { 0.0 };
        let resist = 0.5 * 1.2 * p.drag_area * self.speed * self.speed.abs() + 0.012 * p.mass * G * self.speed.signum();
        let accel = (drive - braking - resist) / p.mass;
        self.speed = (self.speed + accel * dt).max(MAX_REVERSE);
        if pedal == 0.0 && input.brake == 0.0 && self.speed.abs() < 0.2 {
            self.speed = 0.0;
        }

        self.wheel_spin += self.speed / p.wheel_radius * dt;
        self.heading += self.speed / p.wheelbase() * self.steer_angle.tan() * dt;
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
        let target = (fl + fr + rl + rr) / 4.0;
        self.pos.z += (target - self.pos.z) * 0.5;
        self.pitch = ((fl + fr) / 2.0 - (rl + rr) / 2.0).atan2(fy - ry);
        self.roll = ((fl + rl) / 2.0 - (fr + rr) / 2.0).atan2(ht * 2.0);
    }

    /// Model matrix for a model with +Y forward and +Z up.
    pub fn model(&self) -> Mat4 {
        if let Some(s) = &self.sim {
            return s.body();
        }
        Mat4::from_translation(self.pos + Vec3::Z * self.ride_height)
            * Mat4::from_rotation_z(self.heading)
            * Mat4::from_rotation_x(self.pitch)
            * Mat4::from_rotation_y(self.roll)
    }

    /// Engine speed in rpm for the dashboard.
    pub fn rpm(&self) -> f32 {
        self.engine_rev * 60.0
    }
}

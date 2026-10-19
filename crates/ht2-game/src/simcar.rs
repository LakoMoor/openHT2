//! The player's vehicle run entirely by the code ported from the original: the constructor
//! `_Car_V::_Car_V` (`0x4ebd00`) and the step `_Car_V::run1` (`0x4fdc80`), on a flat memory image
//! with the original's field offsets. The world they collide with is the scene graph of the
//! map's rooms ([`crate::world`]); a scene without rooms falls back to the drawn ground
//! ([`MeshWorld`]).

use crate::ground::Ground;
use glam::{Mat4, Vec3};
use ht2_sim::contact::World;
use ht2_sim::gearbox::TECH_TABLE;
use ht2_sim::mem::Mem;
use ht2_sim::{ai, contact, ctor, globals, loader, step, tech_rec::RECORD_LEN, tech_system as ts};

const CAR: u32 = 0x1000_0000;
/// The computer vehicles follow the player's in memory, this far apart.
const CAR_STRIDE: u32 = 0x10000;
const CLOCK: u32 = 0x1800_0000;
/// Stands for the room the vehicle is in (the mesh world has only one).
const ROOM: u32 = 0x1900_0000;
/// The surface record returned when the scene has no collision meshes and the drawn ground is
/// used instead: type 1, kind 1 (dry asphalt), no parameters.
const SURFACE: u32 = 0x1900_0100;
/// A stand-in for the scene collision when the map has no rooms: the nearest crossing of the
/// query segment with the drawn ground, all of it dry asphalt.
pub struct MeshWorld<'a> {
    pub ground: &'a Ground,
}

impl World for MeshWorld<'_> {
    fn collide(&mut self, m: &mut Mem, _obj: u32, mode: u32, rec: u32) -> u32 {
        let v = |a: u32| Vec3::new(m.f32(a), m.f32(a + 4), m.f32(a + 8));
        if mode & 4 == 0 {
            return 0;
        }
        let (a, b) = (v(rec + 0x4c), v(rec + 0x58));
        let Some((p, n)) = self.ground.segment_hit(a, b) else { return 0 };
        let surface = SURFACE;
        for (i, c) in p.to_array().iter().enumerate() {
            m.set_f32(rec + 0x34 + 4 * i as u32, *c);
        }
        for (i, c) in n.to_array().iter().enumerate() {
            m.set_f32(rec + 0x40 + 4 * i as u32, *c);
        }
        m.set_u32(rec + 0x64, surface);
        1
    }
}

/// Wheel `Space` transforms of a model (12 floats: three axes, then the origin).
pub type WheelNodes = [Option<[f32; 12]>; 8];

pub struct SimCar {
    m: Mem,
    time: f64,
    /// Whether the scene graph (rooms) is used; otherwise the drawn ground.
    rooms: bool,
    /// Computer vehicles: their address and their driver.
    traffic: Vec<(u32, ai::Driver)>,
    vid: i32,
}

/// What is needed to draw a vehicle.
pub struct Pose {
    pub body: Mat4,
    pub steer: f32,
    pub wheel_angle: [f32; 8],
    pub wheel_drop: [f32; 8],
}

fn with_world<R>(ground: &Ground, rooms: bool, f: impl FnOnce(&mut dyn World) -> R) -> R {
    if rooms {
        f(&mut crate::world::SceneWorld { scene: &ground.scene })
    } else {
        f(&mut MeshWorld { ground })
    }
}

/// Gear request codes the original's gearbox understands (`+0x2746`).
const REQUEST_REVERSE: i16 = 0x14;
const REQUEST_NEUTRAL: i16 = 0x15;
const REQUEST_DRIVE: i16 = 0x16;

impl SimCar {
    /// Build vehicle `record` of `tech_file` (the contents of `vehicle.tech`) at `pos`, facing
    /// `heading` (0 is +Y, positive turns left).
    pub fn new(tech_file: &[u8], record: usize, wheels: &WheelNodes, add: &WheelNodes, ground: &Ground, pos: Vec3, heading: f32) -> Result<Self, String> {
        let name_len = u32::from_le_bytes(tech_file.get(0..4).ok_or("short vehicle.tech")?.try_into().unwrap()) as usize;
        let hdr = 4 + name_len;
        let rec_len = u32::from_le_bytes(tech_file[hdr + 4..hdr + 8].try_into().unwrap()) as usize;
        let count = u32::from_le_bytes(tech_file[hdr + 8..hdr + 12].try_into().unwrap()) as usize;
        if rec_len != RECORD_LEN || record >= count {
            return Err("unexpected vehicle.tech layout".into());
        }
        let mut m = Mem::new();
        for i in 0..count {
            m.write(TECH_TABLE + (i * RECORD_LEN) as u32, &tech_file[57 + i * rec_len..57 + (i + 1) * rec_len]);
        }
        globals::install_clock(&mut m, CLOCK, 0.0, 0.0);
        globals::install_constants(&mut m);
        contact::install_surface_tables(&mut m);
        for i in 0..3 {
            m.set_f32(0x6974a8 + 4 * i, 1.0);
        }
        m.set_i16(0x696bda, 1); // wheel queries go through the room (set at start-up)
        m.set_u32(globals::RAND_SEED, 1);
        for (o, v) in [(0u32, 1u32), (4, 1), (8, 0), (12, 0)] {
            m.set_u32(SURFACE + o, v);
        }
        ground.scene.install(&mut m);
        let vid = record as i32;
        loader::init_vehicle_nodes(&mut m, vid, wheels, add);
        let (c, s) = (heading.cos(), heading.sin());
        let mat = [c, s, 0.0, -s, c, 0.0, 0.0, 0.0, 1.0, pos.x, pos.y, pos.z];
        let start_room = ground.scene.room_at(&mut m, pos.to_array());
        let rooms = start_room.is_some();
        let room = start_room.unwrap_or(ROOM);
        with_world(ground, rooms, |w| ctor::fn_004ebd00(&mut m, w, CAR, room, mat, vid));
        // The player starts with the engine running, automatic gearbox, keyboard control.
        m.set_i32(CAR + 0x2774, 1);
        m.set_i32(CAR + 0x2760, 1);
        m.set_i32(CAR + 0x1b24, 2);
        m.set_i16(CAR + 0x2704, 1);
        m.set_i16(CAR + 0x2706, 1);
        ht2_sim::gearbox::fn_004ff770(&mut m, CAR, 1);
        Ok(SimCar { m, time: 0.0, rooms, traffic: Vec::new(), vid })
    }

    /// One step of `dt` seconds with the pedals in `0..=1` and the steering keys (-1 right, 1 left).
    pub fn step(&mut self, ground: &Ground, dt: f32, throttle: f32, brake: f32, steer: i32) {
        let m = &mut self.m;
        self.time += dt as f64;
        globals::install_clock(m, CLOCK, self.time, dt as f64);
        m.wr8(CAR + 0x2702, (steer > 0) as u8);
        m.wr8(CAR + 0x2703, (steer < 0) as u8);
        let speed = m.f32(CAR + 0xdc8);
        let gear = m.i32(CAR + 0x1c1c);
        let shifting = m.i32(CAR + 0x1c24) != 0;
        // Drive or reverse the way the game's keyboard control picks them: brake at standstill
        // engages reverse, where the brake key is the gas pedal.
        let reversing = gear == 0 && throttle == 0.0;
        let request = if shifting {
            0
        } else if throttle > 0.0 && gear < 2 && speed > -0.5 {
            REQUEST_DRIVE
        } else if brake > 0.0 && throttle == 0.0 && speed < 0.5 && gear != 0 {
            REQUEST_REVERSE
        } else if throttle == 0.0 && brake == 0.0 && speed.abs() < 0.2 && gear != 1 {
            REQUEST_NEUTRAL
        } else {
            0
        };
        m.set_i16(CAR + 0x2746, request);
        let (pedal, braking) = if reversing { (brake, 0.0) } else { (throttle, brake) };
        m.set_f32(CAR + 0x270c, pedal.clamp(0.0, 1.0));
        m.set_f32(CAR + 0x2710, 0.0);
        m.set_i16(CAR + 0x2714, (braking > 0.0) as i16);
        with_world(ground, self.rooms, |w| step::fn_004fdc80(m, w, CAR));
        for (car, drv) in &mut self.traffic {
            ai::drive(m, *car, &ground.roads, drv, 0, dt as f64);
            with_world(ground, self.rooms, |w| step::fn_004fdc80(m, w, *car));
        }
    }

    /// Put `count` computer vehicles of the player's type on the roads 60 to 400 m from the
    /// player, in the right-hand lane. Stand-in: the original places traffic from the road
    /// module's own rules (not ported) and with its own vehicle types.
    pub fn add_traffic(&mut self, ground: &Ground, count: usize, seed: u32) {
        let net = &ground.roads;
        let me = self.pos();
        let mut taken: Vec<Vec3> = vec![me];
        let mut rng = seed | 1;
        let mut next = move || {
            rng ^= rng << 13;
            rng ^= rng >> 17;
            rng ^= rng << 5;
            rng
        };
        for _ in 0..count * 200 {
            if self.traffic.len() >= count || net.roads.is_empty() {
                break;
            }
            let road = next() as usize % net.roads.len();
            let mut place = net.road_start(road);
            let mut d = (next() % 1000) as f64 * 0.001 * net.roads[road].length;
            net.advance(&mut place, &mut d);
            place.node = None;
            if next() & 1 == 1 {
                place.reversed = true;
            }
            let c = net.coordinates(&place);
            let t = net.direction(&place, false);
            let l = (t[0] * t[0] + t[1] * t[1]).sqrt();
            if l < 1e-6 {
                continue;
            }
            let (fx, fy) = ((t[0] / l) as f32, (t[1] / l) as f32);
            let at = Vec3::new(c[0] as f32 + fy * 1.75, c[1] as f32 - fx * 1.75, c[2] as f32);
            let dist = (at - me).truncate().length();
            if !(60.0..=400.0).contains(&dist) || taken.iter().any(|p| (*p - at).length() < 20.0) {
                continue;
            }
            let Some(z) = ground.height_at(at.x, at.y, at.z + 3.0, 0.0) else { continue };
            let pos = Vec3::new(at.x, at.y, z + 1.0);
            let car = CAR + CAR_STRIDE * (self.traffic.len() as u32 + 1);
            let Some(room) = ground.scene.room_at(&mut self.m, pos.to_array()).or((!self.rooms).then_some(ROOM)) else { continue };
            let heading = (-fx).atan2(fy);
            let (c, s) = (heading.cos(), heading.sin());
            let mat = [c, s, 0.0, -s, c, 0.0, 0.0, 0.0, 1.0, pos.x, pos.y, pos.z];
            let m = &mut self.m;
            let vid = self.vid;
            with_world(ground, self.rooms, |w| ctor::fn_004ebd00(m, w, car, room, mat, vid));
            // Engine running, automatic gearbox, pedals and steering set by the driver.
            m.set_i32(car + 0x2774, 1);
            m.set_i32(car + 0x2760, 1);
            m.set_i32(car + 0x1b24, 2);
            m.set_i16(car + 0x2704, 1);
            m.set_i16(car + 0x2706, 1);
            ht2_sim::gearbox::fn_004ff770(m, car, 1);
            ai::init_driver(m, car, next());
            m.set_i16(car + 0x2746, REQUEST_DRIVE);
            self.traffic.push((car, ai::Driver::new(place, next())));
            taken.push(pos);
        }
    }

    /// The computer vehicles, for drawing.
    pub fn traffic(&self) -> Vec<Pose> {
        self.traffic.iter().map(|(car, _)| self.pose(*car)).collect()
    }

    fn pose(&self, car: u32) -> Pose {
        let m = &self.m;
        let g = |i: u32| m.f32(car + 0x10 + 4 * i);
        Pose {
            body: Mat4::from_cols(
                Vec3::new(g(0), g(1), g(2)).extend(0.0),
                Vec3::new(g(3), g(4), g(5)).extend(0.0),
                Vec3::new(g(6), g(7), g(8)).extend(0.0),
                Vec3::new(g(9), g(10), g(11)).extend(1.0),
            ),
            steer: m.f32(car + 0x1a20),
            wheel_angle: core::array::from_fn(|k| m.f32(car + 0xe9c + 4 * k as u32)),
            wheel_drop: core::array::from_fn(|k| m.f32(car + 0xdd8 + 4 * k as u32) - 0.75 * m.f32(car + 0x1f9c + 4 * k as u32)),
        }
    }

    /// For diagnostics: speed, mode, lane offset and inputs of computer vehicle `k`.
    pub fn traffic_state(&self, k: usize) -> Option<String> {
        let (car, d) = self.traffic.get(k)?;
        let m = &self.m;
        Some(format!(
            "car{k}: pos={:6.1?} v={:5.1} gear={} mode={} lat={:5.2} steer={:5.2} thr={:.2} brk={:.2} road={} t={:.3}",
            [m.f32(car + 0x34), m.f32(car + 0x38), m.f32(car + 0x3c)],
            m.f32(car + 0xdc8),
            m.i32(car + 0x1c1c),
            m.u32(car + ai::MODE),
            m.f64(car + ai::LATERAL),
            m.f32(car + ai::STEER_INPUT),
            m.f32(car + 0x270c),
            m.f32(car + 0x2710),
            d.place.road,
            d.place.t
        ))
    }

    /// The body transform (the original's matrix: rows are the body axes, then the origin).
    pub fn body(&self) -> Mat4 {
        let g = |i: u32| self.m.f32(CAR + 0x10 + 4 * i);
        Mat4::from_cols(
            Vec3::new(g(0), g(1), g(2)).extend(0.0),
            Vec3::new(g(3), g(4), g(5)).extend(0.0),
            Vec3::new(g(6), g(7), g(8)).extend(0.0),
            Vec3::new(g(9), g(10), g(11)).extend(1.0),
        )
    }

    pub fn pos(&self) -> Vec3 {
        let g = |i: u32| self.m.f32(CAR + 0x34 + 4 * i);
        Vec3::new(g(0), g(1), g(2))
    }

    /// Forward speed, m/s.
    pub fn speed(&self) -> f32 {
        self.m.f32(CAR + 0xdc8)
    }

    pub fn gear(&self) -> i32 {
        self.m.i32(CAR + 0x1c1c)
    }

    /// Engine speed, revolutions per second.
    pub fn engine_rev(&self) -> f32 {
        self.m.f32(CAR + 0x1c84)
    }

    pub fn fuel(&self) -> f32 {
        ts::value(&self.m, CAR + 0x3ef0)
    }

    /// Angle of the steered wheels, radians, positive to the left.
    pub fn steer_angle(&self) -> f32 {
        self.m.f32(CAR + 0x1a20)
    }

    /// Rotation angle of wheel `k`, radians.
    pub fn wheel_angle(&self, k: usize) -> f32 {
        self.m.f32(CAR + 0xe9c + 4 * k as u32)
    }

    /// How far wheel `k` hangs below its rest position, m: `+0xdd8` is the distance from the mount
    /// down to the wheel centre (0.75 of the travel when the vehicle is built). For drawing the
    /// wheels only; the original's wheel rendering is not ported.
    /// The id of the room the vehicle is in (`+0xf4`), 0 for none.
    pub fn room(&self) -> u32 {
        self.m.u32(CAR + 0xf4)
    }

    /// For diagnostics: the room whose road is right below the vehicle.
    pub fn room_below(&mut self, ground: &Ground) -> Option<u32> {
        let p = self.pos().to_array();
        ground.scene.room_at(&mut self.m, p)
    }

    /// Whether the scene graph is used for collision.
    pub fn uses_rooms(&self) -> bool {
        self.rooms
    }

    pub fn wheel_drop(&self, k: usize) -> f32 {
        self.m.f32(CAR + 0xdd8 + 4 * k as u32) - 0.75 * self.m.f32(CAR + 0x1f9c + 4 * k as u32)
    }
}

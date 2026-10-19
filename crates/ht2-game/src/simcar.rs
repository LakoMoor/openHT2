//! The player's vehicle run entirely by the code ported from the original: the constructor
//! `_Car_V::_Car_V` (`0x4ebd00`) and the step `_Car_V::run1` (`0x4fdc80`), on a flat memory image
//! with the original's field offsets. The world they collide with is the drivable ground mesh
//! (see [`MeshWorld`]); the original's scene collision is not ported yet.

use crate::ground::Ground;
use glam::{Mat4, Vec3};
use ht2_sim::contact::World;
use ht2_sim::gearbox::TECH_TABLE;
use ht2_sim::mem::Mem;
use ht2_sim::{contact, ctor, globals, loader, step, tech_rec::RECORD_LEN, tech_system as ts};

const CAR: u32 = 0x1000_0000;
const CLOCK: u32 = 0x1800_0000;
/// Stands for the room the vehicle is in (the mesh world has only one).
const ROOM: u32 = 0x1900_0000;
/// The surface record returned for every hit: type 1, kind 1 (dry asphalt), no parameters.
const SURFACE: u32 = 0x1900_0100;

/// The scene-collision query (`vtable + 0x34` of the scene nodes) answered from the ground mesh:
/// the segment of the query record is intersected with the drivable triangles.
pub struct MeshWorld<'a> {
    pub ground: &'a Ground,
}

impl World for MeshWorld<'_> {
    fn collide(&mut self, m: &mut Mem, _obj: u32, _mode: u32, rec: u32) -> u32 {
        let v = |a: u32| Vec3::new(m.f32(a), m.f32(a + 4), m.f32(a + 8));
        let (a, b) = (v(rec + 0x4c), v(rec + 0x58));
        let Some((p, n)) = self.ground.segment_hit(a, b) else { return 0 };
        for (i, c) in p.to_array().iter().enumerate() {
            m.set_f32(rec + 0x34 + 4 * i as u32, *c);
        }
        for (i, c) in n.to_array().iter().enumerate() {
            m.set_f32(rec + 0x40 + 4 * i as u32, *c);
        }
        m.set_u32(rec + 0x64, SURFACE);
        1
    }
}

/// Wheel `Space` transforms of a model (12 floats: three axes, then the origin).
pub type WheelNodes = [Option<[f32; 12]>; 8];

pub struct SimCar {
    m: Mem,
    time: f64,
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
        let vid = record as i32;
        loader::init_vehicle_nodes(&mut m, vid, wheels, add);
        let (c, s) = (heading.cos(), heading.sin());
        let mat = [c, s, 0.0, -s, c, 0.0, 0.0, 0.0, 1.0, pos.x, pos.y, pos.z];
        let mut world = MeshWorld { ground };
        ctor::fn_004ebd00(&mut m, &mut world, CAR, ROOM, mat, vid);
        // The player starts with the engine running, automatic gearbox, keyboard control.
        m.set_i32(CAR + 0x2774, 1);
        m.set_i32(CAR + 0x2760, 1);
        m.set_i32(CAR + 0x1b24, 2);
        m.set_i16(CAR + 0x2704, 1);
        m.set_i16(CAR + 0x2706, 1);
        ht2_sim::gearbox::fn_004ff770(&mut m, CAR, 1);
        Ok(SimCar { m, time: 0.0 })
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
        let mut world = MeshWorld { ground };
        step::fn_004fdc80(m, &mut world, CAR);
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
    pub fn wheel_drop(&self, k: usize) -> f32 {
        self.m.f32(CAR + 0xdd8 + 4 * k as u32) - 0.75 * self.m.f32(CAR + 0x1f9c + 4 * k as u32)
    }
}

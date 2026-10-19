//! Initialisation of a new vehicle object. The engine part of `_Car_V::_Car_V` (`0x4ebd00`).
//!
//! The constructor copies the vehicle's `vehicle.tech` record and derives from it the gearbox
//! gear factors, the engine and fuel [`TechSystem`](crate::tech_system)s and the speed filters.

use crate::filter;
use crate::gearbox::tech_record;
use crate::globals;
use crate::mem::Mem;
use crate::tech_system as ts;

fn rec_f32(rec: &[u8], off: usize) -> f32 {
    f32::from_le_bytes(rec[off..off + 4].try_into().unwrap())
}

fn rec_i32(rec: &[u8], off: usize) -> i32 {
    i32::from_le_bytes(rec[off..off + 4].try_into().unwrap())
}

/// Set up the engine, gearbox, fuel and the objects they use for vehicle `id` of the tech table.
pub fn init_engine(m: &mut Mem, this: u32, id: i32) {
    let rec = tech_record(m, id);
    let (mass, radius) = (rec_f32(&rec, 96), rec_f32(&rec, 100));
    let (tank, endurance) = (rec_f32(&rec, 108), rec_f32(&rec, 112));
    let (torque, brake_coef, tach) = (rec_f32(&rec, 192), rec_f32(&rec, 196), rec_f32(&rec, 268));
    let (num_gears, axle) = (rec_i32(&rec, 144), rec_f32(&rec, 204));

    m.set_i32(this + 0x8cc, id);
    m.set_f32(this + 0x2640, mass);
    m.set_f32(this + 0x1b70, radius);
    let rim = rec_f32(&rec, 104);
    for i in 0..8 {
        m.set_f32(this + 0x1b74 + 4 * i, radius);
        m.set_f32(this + 0x1b94 + 4 * i, rim);
    }

    // The two speed filters.
    filter::new(m, this + 0x1b30, filter::WEIGHT_QUADRATIC, 0.5, 0, 0.0);
    filter::new(m, this + 0x1b50, filter::WEIGHT_COSINE, f32::from_bits(0x3f33_3333), 0, 0.0);

    // Seven TechSystems, all empty.
    for k in 0..7 {
        ts::reset(m, this + 0x3ebc + 0x34 * k);
    }

    // Gearbox: manual, in neutral, gear factors = transmission ratio * final drive.
    m.set_i32(this + 0x1c1c, 1);
    m.set_i32(this + 0x1c20, -1);
    m.set_i32(this + 0x1c14, 1);
    m.set_i32(this + 0x1c18, num_gears);
    m.set_i32(this + 0x3bdc, num_gears);
    m.set_f32(this + 0x1c80, axle);
    let mut i = 0;
    while i <= num_gears.max(-1) {
        let ratio = rec_f32(&rec, 208 + 4 * i as usize);
        m.set_f32(this + 0x1c44 + 4 * i as u32, ratio * axle);
        i += 1;
    }
    m.set_f32(this + 0x1c84, 0.0);
    m.set_f32(this + 0x1c88, tach);
    m.set_f32(this + 0x1c8c, brake_coef);

    // Engine torque: 15 %..100 % of the peak, or 25 %..140 % when upgraded.
    m.set_i32(this + 0x3eec, 3);
    ts::define(m, this + 0x3ebc, 0, (torque as f64 * 0.15) as f32, torque, 1.0);
    ts::define(m, this + 0x3ebc, 1, torque * 0.25, (torque as f64 * 1.4) as f32, 1.0);
    // Fuel: empty..tank, used up over `endurance`.
    m.set_i32(this + 0x3f20, 2);
    ts::define(m, this + 0x3ef0, 0, 0.0, tank, endurance);
    ts::define(m, this + 0x3ef0, 1, 0.0, tank + tank, endurance + endurance);
    // Engine braking.
    ts::define(m, this + 0x3ff4, 0, 0.0, 0.0, 0.0);
    ts::define(m, this + 0x3ff4, 1, brake_coef * 0.25, brake_coef + brake_coef, 0.0);
    // Scale of the engine torque curve.
    ts::define(m, this + 0x3fc0, 0, 1.0, 1.0, 1.0);
    ts::define(m, this + 0x3fc0, 1, f32::from_bits(0x3f4c_cccd), f32::from_bits(0x3f4c_cccd), 1.0);

    m.set_f32(this + 0x1c94, 0.0);
    m.set_f32(this + 0x1cb8, 1.0);
    m.set_f32(this + 0x1c38, f32::from_bits(0x3dcc_cccd));
    m.set_f32(this + 0x1c30, 0.5);
    m.set_f32(this + 0x1c34, f32::from_bits(0x3ba3_d70a));
    let t = globals::time(m);
    m.set_f32(this + 0x1c3c, t);
    m.set_f32(this + 0x1c40, 0.5);
    m.set_f32(this + 0x1c2c, 0.0);
    m.set_i32(this + 0x1c24, 0);

    ts::define(m, this + 0x3f58, 0, f32::from_bits(0x3dcc_cccd), 1.0, 60.0);
    ts::define(m, this + 0x3f58, 1, 0.5, 1.0, 120.0);
    ts::define(m, this + 0x3f8c, 0, 0.0, 1.0, 10.0);
    ts::define(m, this + 0x3f8c, 1, 0.0, 1.0, 20.0);
    ts::define(m, this + 0x3f24, 0, f32::from_bits(0x3f19_999a), 1.0, 30000.0);
    ts::define(m, this + 0x3f24, 1, f32::from_bits(0x3f4c_cccd), 1.0, 60000.0);
    ts::define(m, this + 0x3f24, 2, f32::from_bits(0x3f4c_cccd), 1.0, 60000.0);

    // At the end of the constructor a snapshot of the block `+0x1b70..+0x1d5c` is kept at `+0x1d5c`;
    // the per-frame code restores the number of gears and the shift times from it.
    for i in 0..0x7b {
        let v = m.u32(this + 0x1b70 + 4 * i);
        m.set_u32(this + 0x1d5c + 4 * i, v);
    }
}

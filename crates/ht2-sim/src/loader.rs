//! The physics part of the vehicle loader `InitVehicleNodes` (`0x53fdb0`): the wheel count and
//! wheel positions of a `vehicle.tech` record and the model matrix of the vehicle type, taken from
//! the `<name>Wheel%dSpace` / `<name>AddWheel%dSpace` nodes of the vehicle model.
//!
//! The original also builds the collision hull of the body (`qhdll.dll`, written to the vehicle's
//! `.ini` file) and looks up the cab, lights and other nodes; that is not part of this.

use crate::gearbox::TECH_TABLE;
use crate::mem::Mem;

/// Per vehicle type (0x5d0 bytes each): the transforms of its wheel `Space` nodes (31 entries of
/// 0x30 bytes from `+0`), the model matrix at `+0x1b0` and its inverse at `+0x1e0`.
pub const MODEL_TABLE: u32 = 0x6d2200;
pub const MODEL_ENTRY: u32 = 0x5d0;

fn f(m: &Mem, a: u32) -> f64 {
    m.f32(a) as f64
}

fn r(x: f64) -> f32 {
    x as f32
}

/// Set up vehicle type `vid` from the transforms (12 floats: axes, then origin) of its wheel
/// nodes: `wheel[k]` is `<name>Wheel{k}Space`, `add[k]` is `<name>AddWheel{k}Space`.
pub fn init_vehicle_nodes(m: &mut Mem, vid: i32, wheel: &[Option<[f32; 12]>; 8], add: &[Option<[f32; 12]>; 8]) {
    let tech = TECH_TABLE + (vid as u32).wrapping_mul(0x970);
    let entry = MODEL_TABLE + (vid as u32).wrapping_mul(MODEL_ENTRY);
    let (main, total) = (tech + 0x860, tech + 0x864);
    m.set_i32(total, 0);
    m.set_i32(main, 0);
    let put = |m: &mut Mem, k: u32, t: &[f32; 12]| {
        for (i, v) in t.iter().enumerate() {
            m.set_f32(entry + 0x30 * k + 4 * i as u32, *v);
        }
    };
    for k in 0..8u32 {
        match wheel[k as usize] {
            Some(t) if m.i32(total) == m.i32(main) => {
                put(m, k, &t);
                m.set_i32(main, m.i32(main) + 1);
            }
            _ => match add[k as usize] {
                Some(t) => put(m, k, &t),
                None => break,
            },
        }
        m.set_i32(total, m.i32(total) + 1);
    }
    let x = [f(m, 0x702640), f(m, 0x702644), f(m, 0x702648)];
    let model = entry + 0x1b0;
    if m.i32(tech + 0x35c) == 0 {
        // the model is placed so that the first wheel node is at the first wheel of the record
        let front = f(m, tech + 0xb8);
        let h = [r(r(front * x[0]) as f64 * 0.5) as f64, r(r(front * x[1]) as f64 * 0.5) as f64, r(front * x[2] * 0.5) as f64];
        let c = [r(f(m, entry + 0x24) - h[0]) as f64, r(f(m, entry + 0x28) - h[1]) as f64, f(m, entry + 0x2c) - h[2]];
        let w = [f(m, tech + 0x1a8), f(m, tech + 0x1ac), f(m, tech + 0x1b0)];
        crate::matrix::mat_translation(m, model, r(c[0] - w[0]), r(c[1] - w[1]), r(c[2] - w[2]));
        crate::math::mat_inverse(m, entry + 0x1e0, model);
        return;
    }
    for i in 0..12 {
        let v = m.u32(tech + 4 * i);
        m.set_u32(model + 4 * i, v);
    }
    crate::math::mat_inverse(m, entry + 0x1e0, model);
    if m.i32(tech + 0x2c8) != 0 {
        return;
    }
    // wheel positions: the node origin moved by the track along X (the first two wheels use the
    // front track, the others the rear one; even wheels to the left, odd ones to the right)
    let (front, rear) = (f(m, tech + 0xb8), f(m, tech + 0xbc));
    let n_main = m.i32(main);
    for k in 0..m.i32(total).max(0) {
        let (s, t) = match k {
            0 => (-1.0, front),
            1 => (1.0, front),
            _ if k & 1 == 0 => (-1.0, rear),
            _ => (1.0, rear),
        };
        let o = [r(s * x[0] * t) as f64, r(s * x[1] * t) as f64, r(s * x[2] * t) as f64];
        let node = entry + 0x30 * k as u32;
        let p = [r(o[0] + f(m, node + 0x24)), r(o[1] + f(m, node + 0x28)), r(o[2] + f(m, node + 0x2c))];
        let dst = if k < n_main { tech + 0x1a8 + 12 * k as u32 } else { tech + 0x208 + 12 * (k - n_main) as u32 };
        for (i, v) in p.iter().enumerate() {
            m.set_f32(dst + 4 * i as u32, *v);
        }
    }
    if n_main < 8 {
        // the saddle of a tractor is the wheel after the last one, a little below the first
        let dst = tech + 0x1a8 + 12 * n_main as u32;
        for i in 0..3 {
            let v = m.u32(tech + 0x7dc + 4 * i);
            m.set_u32(dst + 4 * i, v);
        }
        m.set_f32(dst + 8, r(f(m, tech + 0x1b0) - 0.4));
    }
}

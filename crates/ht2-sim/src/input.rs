//! Driver input processing of `_Car_V` (`0x500850`): steering smoothing and the pedal / key
//! state, ending in the engine and gearbox update.
//!
//! Inputs: `+0x2701` (fast-steer key), `+0x2702` / `+0x2703` (steer left / right keys),
//! `+0x2706` (keyboard control active), `+0x2724` (steering speed), `+0x2728` (steering
//! damping). Output: `+0x29e0`, the steering position in `-1..=1` (positive is left).

use crate::gearbox_update::fn_004ff800;
use crate::globals::{self, VEC_ZERO};
use crate::mem::Mem;

fn r(x: f64) -> f32 {
    x as f32
}

/// The static float the original writes (`0x67484c`) as a scratch value for the steering speed.
const STEER_SCRATCH: u32 = 0x0067_484c;

pub fn fn_00500850(m: &mut Mem, this: u32) {
    macro_rules! f {
        ($o:expr) => {
            m.f32(this + $o)
        };
    }
    macro_rules! s {
        ($o:expr) => {
            m.i16(this + $o)
        };
    }

    let dt = r(globals::dt_hi(m));
    let (left, right) = (m.rd8(this + 0x2702), m.rd8(this + 0x2703));
    let key_steering = left == 0 && right == 0 && s!(0x2706) != 0;
    let diff = left as i32 - right as i32;
    let fast = m.rd8(this + 0x2701);
    if fast as u32 != m.u32(this + 0x1b20) && (s!(0x2740) != 0 || m.i32(this + 0x274c) != 0) {
        let t = globals::time_hi(m) - dt as f64;
        m.set_f64(this + 0x2738, t);
        m.set_f64(this + 0x2730, t);
    }

    // Damping factor for the steering returning to the centre.
    let damping = f!(0x2728) as f64;
    let mut local4 = if damping < 0.05 {
        1.0f32
    } else if damping <= 0.95 {
        r(((1.0 - f!(0x2728) as f64) * 0.99 + 0.01).powf(dt as f64 * 10.0))
    } else {
        f32::from_bits(0x3dcc_cccd)
    };

    if key_steering {
        let v = r((f!(0x2724) as f64 + f!(0x2724) as f64 + 0.2) * f!(0x271c) as f64);
        m.set_f32(this + 0x271c, v);
        for _ in 0..2 {
            if 1.0 < f!(0x271c) {
                m.set_f32(this + 0x271c, 1.0);
            }
            if f!(0x271c) < -1.0 {
                m.set_f32(this + 0x271c, -1.0);
            }
        }
        if m.i32(this + 0x29e8) == 0 {
            let t = f!(0x271c);
            m.set_f32(this + 0x29e0, t);
        } else {
            let k = r((1.0 - local4 as f64) * (f!(0x271c) as f64 - f!(0x29e0) as f64) + f!(0x29e0) as f64);
            m.set_f32(this + 0x29e0, k);
            if ((k - f!(0x271c)) as f64).abs() < dt as f64 * 0.5 {
                m.set_i32(this + 0x29e8, 0);
            }
        }
    } else {
        m.set_i32(this + 0x29e8, 1);
        let speed = f!(0xdc8).abs() as f64;
        let mut local_c = r(20.0 / (speed + 20.0));
        if s!(0x2740) != 0 && fast == 0 {
            let v = if (0.1f64) <= speed {
                let k = f!(0x26b0) as f64 * f!(0x263c) as f64 * f!(0x1f60) as f64;
                let a = k / (f!(0xdc0) as f64 * f!(0xdc0) as f64);
                a.atan() * 1.5
            } else {
                1.0
            };
            local_c = r(v);
            if 1.0 < v {
                local_c = 1.0;
            }
        }
        if fast == 0 {
            m.set_f32(STEER_SCRATCH, 0.66);
        } else {
            local_c = 1.0;
            m.set_f32(STEER_SCRATCH, 2.0);
        }
        if !(left == 1 && right == 1) {
            let now = globals::time_hi(m);
            let rate = |m: &Mem, stamp: u32| -> f64 {
                ((now - m.f64(this + stamp)) + 0.2)
                    * (1.0 / (m.f32(this + 0xdc8).abs() as f64 * 0.25 + 1.0))
                    * m.f32(this + 0x2724) as f64
                    * dt as f64
                    * 20.0
                    * m.f32(STEER_SCRATCH) as f64
            };
            if left == 0 {
                if right == 0 {
                    let v = r(local4 as f64 * f!(0x29e0) as f64);
                    m.set_f32(this + 0x29e0, v);
                } else {
                    let fv = rate(m, 0x2738);
                    let neg_c = -local_c;
                    if 0.0 < f!(0x29e0) {
                        local4 = r(local4 as f64 * f!(0x29e0) as f64);
                        local4 = r(local4 as f64 - local4.abs() as f64 * dt as f64);
                        m.set_f32(this + 0x29e0, local4);
                        if local4 < 0.0 {
                            m.set_f32(this + 0x29e0, 0.0);
                        }
                        let v = f!(0x29e0) as f64 - fv;
                        m.set_f32(this + 0x29e0, r(v));
                        if v < neg_c as f64 {
                            m.set_f32(this + 0x29e0, neg_c);
                        }
                    } else if fast != 0 || neg_c as f64 - dt as f64 <= f!(0x29e0) as f64 {
                        let v = f!(0x29e0) as f64 - fv;
                        m.set_f32(this + 0x29e0, r(v));
                        if v < neg_c as f64 {
                            m.set_f32(this + 0x29e0, neg_c);
                        }
                    } else {
                        if f!(0x29e0) < -1.0 {
                            m.set_f32(this + 0x29e0, -1.0);
                        }
                        let v4 = r(dt as f64 + f!(0x29e0) as f64);
                        m.set_f32(this + 0x29e0, v4);
                        if (v4 as f64) < dt as f64 - local_c as f64 && neg_c < v4 {
                            m.set_f32(this + 0x29e0, neg_c);
                        }
                    }
                }
            } else {
                let fv = rate(m, 0x2730);
                if f!(0x29e0) < 0.0 {
                    local4 = r(local4 as f64 * f!(0x29e0) as f64);
                    local4 = r(local4 as f64 + local4.abs() as f64 * dt as f64);
                    m.set_f32(this + 0x29e0, local4);
                    if 0.0 < local4 {
                        m.set_f32(this + 0x29e0, 0.0);
                    }
                    let v = fv + f!(0x29e0) as f64;
                    m.set_f32(this + 0x29e0, r(v));
                    if (local_c as f64) < v {
                        m.set_f32(this + 0x29e0, local_c);
                    }
                } else if fast != 0 || f!(0x29e0) as f64 <= dt as f64 + local_c as f64 {
                    let v = fv + f!(0x29e0) as f64;
                    m.set_f32(this + 0x29e0, r(v));
                    if (local_c as f64) < v {
                        m.set_f32(this + 0x29e0, local_c);
                    }
                } else {
                    if 1.0 < f!(0x29e0) {
                        m.set_f32(this + 0x29e0, 1.0);
                    }
                    let v3 = r(f!(0x29e0) as f64 - dt as f64);
                    m.set_f32(this + 0x29e0, v3);
                    if (local_c as f64 - dt as f64) < v3 as f64 && (v3 as f64) < local_c as f64 {
                        m.set_f32(this + 0x29e0, local_c);
                    }
                }
            }
        }
    }

    if diff < 1 {
        let t = globals::time_hi(m);
        m.set_f64(this + 0x2730, t);
        let v = f!(0x29e0);
        m.set_f32(this + 0x1b18, v);
    }
    if -1 < diff {
        let t = globals::time_hi(m);
        m.set_f64(this + 0x2738, t);
        let v = f!(0x29e0);
        m.set_f32(this + 0x1b1c, v);
    }
    m.set_u32(this + 0x1b20, fast as u32);

    let sv = s!(0x2718);
    if s!(0x271a) == 0 {
        if sv == 1 {
            m.set_i16(this + 0x2718, 0);
        } else if sv == 3 {
            m.set_i16(this + 0x2718, 2);
        }
    } else if sv == 0 {
        m.set_i16(this + 0x2718, 3);
    } else if sv == 2 {
        m.set_i16(this + 0x2718, 1);
    }
    if s!(0x2718) < 2 {
        m.set_i32(this + 0x3c50, 0);
    } else {
        m.set_i16(this + 0x2716, 1);
        m.set_i32(this + 0x3c50, 1);
    }

    fn_004ff800(m, this);
    m.set_i16(this + 0x2746, 0);
    m.set_i32(this + 0x3c58, 0);
    for k in 0..3 {
        let z = m.f32(VEC_ZERO + 4 * k);
        m.set_f32(this + 0x3c5c + 4 * k, z);
    }
    m.set_i32(this + 0x3c84, 0);
}

/// `0x5300b0` (thiscall on the device object `0x6d1db8`): value of axis `index`. Each axis has two
/// optional sources (`+0x280 + 4*index` and `+0x28c + 4*index`, pointers to floats): both give their
/// mean, one gives its value, none gives 0.
pub fn device_axis(m: &Mem, device: u32, index: u32) -> f32 {
    let (a, b) = (m.u32(device + index * 4 + 0x280), m.u32(device + index * 4 + 0x28c));
    match (a != 0, b != 0) {
        (true, true) => ((m.f32(b) as f64 + m.f32(a) as f64) * 0.5) as f32,
        (true, false) => m.f32(a),
        (false, true) => m.f32(b),
        (false, false) => 0.0,
    }
}

/// `0x53b590..0x53b684` of the player's input update: the pedal axes into `+0x270c` (gas) and
/// `+0x2710` (the opposite side), then the engine-off override. `filter` is the `injoyfilterA`
/// filter object (`controls + 0x118`); `two_axes` is the global `0x6d2068` (separate gas and
/// brake axes instead of one axis split at zero).
pub fn pedal_axes(m: &mut Mem, car: u32, filter: u32, device: u32, two_axes: bool) {
    if two_axes {
        let gas = device_axis(m, device, 2);
        let brake = device_axis(m, device, 1);
        crate::filter::add(m, filter, gas);
        let f = crate::filter::value(m, filter);
        m.set_f32(car + 0x270c, ((f as f64 + 1.0) * 0.5) as f32);
        m.set_f32(car + 0x2710, ((brake as f64 + 1.0) * 0.5) as f32);
    } else {
        let axis = device_axis(m, device, 2);
        crate::filter::add(m, filter, axis);
        let f = crate::filter::value(m, filter);
        if f < 0.0 || f.is_nan() {
            m.set_f32(car + 0x270c, 0.0);
            let f = f as f64;
            m.set_f32(car + 0x2710, (-(f * f * f)) as f32);
        } else {
            m.set_f32(car + 0x270c, f);
            m.set_f32(car + 0x2710, 0.0);
        }
    }
    if m.rd8(car + 0x2700) == 0 {
        m.set_i16(car + 0x2716, 1);
        m.set_f32(car + 0x2710, 1.0);
    }
}

#[cfg(test)]
mod pedal_axes_tests {
    use super::*;
    fn f32_of(s: &str) -> f32 {
        f32::from_bits(u32::from_str_radix(&s.chars().collect::<Vec<_>>().chunks(2).rev().map(|c| c.iter().collect::<String>()).collect::<String>(), 16).unwrap())
    }
    #[test]
    fn native_pedal_axes() {
        let (car, ctl, vals, clock, device) = (0x2a01_0000u32, 0x2a02_0000u32, 0x2a03_0000u32, 0x2a10_0000u32, 0x6d1db8u32);
        let mut cases = 0;
        for line in include_str!("../../../tools/emu/golden_pedal_axes.tsv").lines() {
            let (head, body) = line.trim_end().split_once('\t').unwrap();
            let h: Vec<&str> = head.split(',').collect();
            let mut m = Mem::new();
            globals::install_clock(&mut m, clock, 0.0, 0.0);
            let two_axes = h[0] == "1";
            let mix: u32 = h[3].parse().unwrap();
            crate::filter::new(&mut m, ctl + 0x118, h[1].parse().unwrap(), f32_of(h[2]), mix, f32_of(h[4]));
            let cfg: Vec<u32> = vec![h[5].parse().unwrap(), h[6].parse().unwrap()];
            for (k, idx) in [1u32, 2].into_iter().enumerate() {
                let a = vals + 16 * idx;
                m.set_u32(device + idx * 4 + 0x280, if cfg[k] & 1 != 0 { a } else { 0 });
                m.set_u32(device + idx * 4 + 0x28c, if cfg[k] & 2 != 0 { a + 4 } else { 0 });
            }
            for (n, frame) in body.split(';').enumerate() {
                let f: Vec<&str> = frame.split(',').collect();
                globals::set_time(&mut m, f[0].parse().unwrap());
                let v: Vec<f32> = f[1].split('/').map(f32_of).collect();
                for idx in [1u32, 2] {
                    m.set_f32(vals + 16 * idx, v[(idx as usize - 1) * 2]);
                    m.set_f32(vals + 16 * idx + 4, v[(idx as usize - 1) * 2 + 1]);
                }
                m.wr8(car + 0x2700, f[2].parse().unwrap());
                m.set_i16(car + 0x2716, 0);
                pedal_axes(&mut m, car, ctl + 0x118, device, two_axes);
                let got = (m.u32(car + 0x270c), m.u32(car + 0x2710), m.i16(car + 0x2716));
                let want = (f[3].parse::<u32>().unwrap(), f[4].parse::<u32>().unwrap(), f[5].parse::<i16>().unwrap());
                assert_eq!(got, want, "{line:.40} frame {n}: {:?} vs {:?}", (f32::from_bits(got.0), f32::from_bits(got.1)), (f32::from_bits(want.0), f32::from_bits(want.1)));
                cases += 1;
            }
        }
        assert!(cases > 400);
    }
}

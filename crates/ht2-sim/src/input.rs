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

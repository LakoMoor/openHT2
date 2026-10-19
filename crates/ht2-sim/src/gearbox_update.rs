//! `0x4ff800`: the per-frame update of the engine, clutch and automatic gearbox of `_Car_V`.
//!
//! Besides the offsets in [`crate::gearbox`]:
//!
//! | offset | meaning |
//! |---|---|
//! | `+0x17d8`, `+0x17dc` | times the throttle / drive input last changed |
//! | `+0x1b24` | engine state: 0 off, 1 starting, 2 running, 3 stalled / out of fuel |
//! | `+0x1c24` | shift in progress (the requested shift code, 0 = none) |
//! | `+0x1c2c`, `+0x1c3c` | time the shift / last shift started |
//! | `+0x1c30..+0x1c38` | shift durations (automatic, manual, into neutral) |
//! | `+0x1c90` | engine speed when the shift started |
//! | `+0x1c94` | engine torque at the wheels (filtered by the filter at `+0x1b30`) |
//! | `+0x2714`, `+0x2716` | `i16` brake and handbrake input |
//! | `+0x270c`, `+0x2710` | `f32` throttle input and clutch input |
//! | `+0x2746` | `i16` gear request: `0x14 + g` go to gear `g`, `0x15` neutral, `1`/`-1` shift up/down |
//!
//! The control flow follows the original closely (it is one 4 KB function), so the `goto`s
//! of the decompiled code appear as flags.

use crate::gearbox::*;
use crate::globals::{self, VEC_UP};
use crate::mem::Mem;
use crate::{filter, tech_system as ts};

fn r(x: f64) -> f32 {
    x as f32
}

pub fn fn_004ff800(m: &mut Mem, this: u32) {
    macro_rules! f {
        ($o:expr) => {
            m.f32(this + $o)
        };
    }
    macro_rules! i {
        ($o:expr) => {
            m.i32(this + $o)
        };
    }
    macro_rules! s {
        ($o:expr) => {
            m.i16(this + $o)
        };
    }

    m.set_f32(this + 0x29dc, 0.0);
    if f!(0x3bd4) < 0.6f32 {
        m.set_u32(this + 0x29d8, 0x3d4ccccd);
        m.set_i32(this + 0x1c1c, 1);
    }
    let now = globals::time_hi(m);
    let t = now as f32;
    let since_input = r(now - f!(0x17dc) as f64);

    // Throttle demand into +0x29dc.
    let mut input_mode_off = true; // bVar11 in the decompilation: true when the pedal input is overridden
    if s!(0x2716) == 0 && s!(0x2714) == 0 && s!(0x2704) != 0 {
        input_mode_off = false;
        if s!(0x2718) < 2 {
            let v = f!(0x2710);
            if 0.0 <= v && v as f64 <= 1.0 {
                m.set_f32(this + 0x29dc, v);
            }
        } else {
            m.set_f32(this + 0x29dc, 1.0);
        }
    } else {
        m.set_f32(this + 0x29dc, 0.0);
        if s!(0x2716) == 1 {
            m.set_f32(this + 0x29dc, r((since_input as f64 + 0.001f64 as f32 as f64) / (since_input as f64 + 1.0)));
        } else if s!(0x2716) == 2 {
            m.set_f32(this + 0x29dc, r((since_input as f64 + 0.001f64 as f32 as f64) / (since_input as f64 + 0.1f64 as f32 as f64)));
        }
    }
    let b11 = input_mode_off;

    if f!(0x29dc) <= 0.001f64 as f32 {
        m.set_f32(this + 0x17dc, t);
    } else if (0.5f64 as f32) < f!(0xdc8) && (0.5f64 as f32) < f!(0x29dc) && s!(0x19a4) == 0 && s!(0x19a6) == 0 {
        let v = ts::value(m, this + 0x3f8c);
        m.set_i32(this + 0x3c18, if 0.0 < v { 1 } else { 2 });
        let up = [m.f32(VEC_UP), m.f32(VEC_UP + 4), m.f32(VEC_UP + 8)];
        // `a` and `b` are rounded to f32 (the product first, then the sum); `c` stays in extended
        // precision on the FPU stack in the original.
        let offset = |m: &Mem, k: f32, o: u32| -> (f32, f32, f64) {
            let t0 = r(k as f64 * up[0] as f64);
            let t1 = r(k as f64 * up[1] as f64);
            let a = r(t0 as f64 + m.f32(this + o) as f64);
            let b = r(t1 as f64 + m.f32(this + o + 4) as f64);
            let c = k as f64 * up[2] as f64 + m.f32(this + o + 8) as f64;
            (a, b, c)
        };
        let to_world = |m: &Mem, (a, b, c): (f32, f32, f64)| -> [f32; 3] {
            let g = |o: u32| m.f32(this + o) as f64;
            [
                r(c * g(0x28) + b as f64 * g(0x1c) + a as f64 * g(0x10) + g(0x34)),
                r(c * g(0x2c) + b as f64 * g(0x20) + a as f64 * g(0x14) + g(0x38)),
                r(c * g(0x30) + b as f64 * g(0x24) + a as f64 * g(0x18) + g(0x3c)),
            ]
        };
        let p1 = to_world(m, offset(m, f!(0x1b74), 0x1844));
        for (k, v) in p1.iter().enumerate() {
            m.set_f32(this + 0x3c1c + 4 * k as u32, *v);
        }
        let p2 = to_world(m, offset(m, f!(0x1b78), 0x1850));
        for (k, v) in p2.iter().enumerate() {
            m.set_f32(this + 0x3c28 + 4 * k as u32, *v);
        }
    }

    if i!(0x2774) == 0 {
        m.set_i32(this + 0x1b24, 0);
    } else if i!(0x1b24) == 2 {
        let mut since_throttle = r(t as f64 - f!(0x17d8) as f64);
        let engine_in = fn_004ff150(m, this, i!(0x1c1c));
        m.set_i32(this + 0x1c28, 0);
        fn_004ff210(m, this, engine_in);
        let grip = fn_004ff660(m, this);

        let mut jump_to_limit = false; // LAB_004ffc5d
        if b11 {
            let u = (0 < s!(0x2714)) as i32 as f32;
            m.set_f32(this + 0x29d8, u);
            if i!(0x1c28) != 0 {
                m.set_f32(this + 0x29d8, 1.0);
            }
            if s!(0x2714) < 2 {
                if s!(0x2714) == 1 {
                    since_throttle = r((since_throttle as f64 * 0.85f64 as f32 as f64) / (since_throttle as f64 + 0.6f64 as f32 as f64) * f!(0x29d8) as f64);
                    m.set_f32(this + 0x29d8, since_throttle);
                    jump_to_limit = true;
                } else {
                    m.set_f32(this + 0x17d8, t);
                }
            } else if i!(0x1c28) == 0 {
                let k = since_throttle as f64 / (since_throttle as f64 + 0.2f64 as f32 as f64);
                m.set_f32(this + 0x29d8, r(k * f!(0x29d8) as f64));
            }
        } else {
            let v = f!(0x270c);
            if v < 0.0 || 1.0 < v {
                if v <= 1.0 {
                    if v < 0.0 {
                        m.set_f32(this + 0x29d8, 0.0);
                    }
                } else {
                    m.set_f32(this + 0x29d8, 1.0);
                }
            } else {
                m.set_f32(this + 0x29d8, v);
            }
            if f!(0x29d8) < 0.1f64 as f32 {
                m.set_f32(this + 0x17d8, t);
            }
            if (i!(0x2754) != 0 || (m.i32(0x696c4c) != 0 && s!(0x2740) != 0)) && m.rd8(this + 0x2701) == 0 {
                since_throttle = f!(0x29d8);
                jump_to_limit = true;
            }
        }
        if jump_to_limit && (grip as f64 * 0.9f64 as f32 as f64) < since_throttle as f64 {
            m.set_f32(this + 0x29d8, r(grip as f64 * 0.9f64 as f32 as f64));
        }

        let gear = i!(0x1c1c);
        if gear == 0 || gear == 2 {
            let speed = f!(0xdc4);
            let extra = if (1.3888888359069824f64 as f32) <= speed.abs() {
                0.0
            } else {
                r(((1.3888888359069824f64 as f32 - speed.abs()) as f64) * 0.72f64 as f32 as f64)
            };
            let base = fn_004ff500(m, this, f!(0x1c84)) as f64;
            let target = extra as f64 * 0.2 + base;
            if gear == 2 && (f!(0x29d8) as f64) < target && speed.abs() < 2.0 {
                m.set_f32(this + 0x29d8, target as f32);
            }
            if gear == 0 && (f!(0x29d8) as f64) < target * 0.2 && speed.abs() < 2.0 {
                m.set_f32(this + 0x29d8, (target * 0.2) as f32);
            }
        } else {
            if f!(0x29d8) < 0.05f64 as f32 {
                m.set_u32(this + 0x29d8, 0x3d4ccccd);
            }
            if (gear == 2 || gear == 0)
                && f!(0x29d8) < 0.1f64 as f32
                && f!(0xdc8).abs() < 1.0
                && (!b11 || s!(0x2714) == 0)
            {
                m.set_i16(this + 0x2746, 0x15);
            }
        }

        let b8 = if b11 { s!(0x2714) >= 1 } else { (0.1f64 as f32) < f!(0x270c) };

        // Choose a gear change.
        let (manual, g) = (i!(0x1c14), i!(0x1c1c));
        let c24 = i!(0x1c24);
        let mut at_141 = false;
        if manual != 0 && 0 < g && (c24 == 0 || s!(0x2746) != 0) {
            if s!(0x2746) == -1 {
                m.set_i16(this + 0x2746, 0x14);
            }
            if !b8 && s!(0x2746) != 0x14 && g != 2 {
                m.set_i16(this + 0x2746, 0);
            }
            let sv = s!(0x2746);
            // Label LAB_004fff4b is reached from three places; `l4b` marks them.
            let mut l4b = false;
            if (sv == 1 || sv == 100) && (1 < g || (sv == 100 && g == 1 && c24 != 0 && 1 < i!(0x1c20))) {
                m.set_i16(this + 0x2746, 0);
                l4b = true;
            } else if sv != 0x14 {
                l4b = true;
            } else if g == 1 {
                if c24 != 0 && i!(0x1c20) == 0 {
                    m.set_i16(this + 0x2746, 0);
                }
                l4b = true;
            }
            if l4b && g == 1 {
                if c24 == 0 && m.rd8(this + 0x2700) == 2 && b8 && s!(0x2746) != 0x14 && s!(0x2746) != 0x15 {
                    m.set_i16(this + 0x2746, 1);
                }
                at_141 = true;
            }
            if !at_141 && s!(0x2746) == 0 {
                let lim_gear = fn_004ff550(m, this, fn_004ff460(m, this, engine_in));
                if f!(0x1c88) * 0.2f64 as f32 <= engine_in.abs()
                    || (0.1f64 as f32 <= f!(0x29d8) && f!(0x29dc) <= 0.1f64 as f32)
                    || (b11 && s!(0x2714) != 0)
                {
                    let scale = (lim_gear as f64 * 1.2f64 as f32 as f64) as f64;
                    let candidates = |m: &mut Mem, from: i32, to: i32, step: i32| {
                        let mut best = 0.0f32;
                        let mut g = from;
                        while if step < 0 { 1 < g } else { g <= to } {
                            let w = fn_004ff150(m, this, g);
                            let force = fn_004ff570(m, this, fn_004ff460(m, this, w), g);
                            if scale < force as f64 && (best as f64) < force as f64 {
                                best = force;
                                m.set_i16(this + 0x2746, (g + 0x14) as i16);
                            }
                            g += step;
                        }
                    };
                    if 2 < i!(0x1c1c) {
                        let from = i!(0x1c1c) - 1;
                        candidates(m, from, 0, -1);
                    }
                    if i!(0x1c1c) < i!(0x1c18) {
                        let (from, to) = (i!(0x1c1c) + 1, i!(0x1c18));
                        if from <= to {
                            candidates(m, from, to, 1);
                        }
                    }
                } else {
                    m.set_i16(this + 0x2746, 0x15);
                }
            }
        }

        // LAB_00500141: manual gearbox requests.
        if i!(0x1c14) == 0
            && ((s!(0x2746) == 1 && i!(0x1c1c) < i!(0x1c18)) || (s!(0x2746) == -1 && 0 < i!(0x1c1c)))
        {
            let ref_force = fn_004ff550(m, this, fn_004ff4a0(m, this));
            let scale = ref_force as f64 * 1.1f64;
            let mut best = 0.0f32;
            let mut try_gear = |m: &mut Mem, g: i32| {
                if g != 1 {
                    let w = fn_004ff150(m, this, g);
                    let force = fn_004ff570(m, this, fn_004ff460(m, this, w), g);
                    if scale < force as f64 && (best as f64) < force as f64 {
                        best = force;
                        m.set_i16(this + 0x2746, (g + 0x14) as i16);
                    }
                }
            };
            if s!(0x2746) < 1 {
                let mut g = i!(0x1c1c);
                loop {
                    g -= 1;
                    if g <= 0 {
                        break;
                    }
                    try_gear(m, g);
                }
            } else {
                let mut g = i!(0x1c1c) + 1;
                while g <= i!(0x1c18) {
                    try_gear(m, g);
                    g += 1;
                }
            }
        }

        // Apply the request.
        let sv = s!(0x2746) as i32;
        if sv != 0 {
            let mut changed = false;
            let set20 = |m: &mut Mem, v: i32| {
                m.set_i32(this + 0x1c20, v);
            };
            let to_two = |m: &mut Mem, changed: &mut bool| {
                if m.i32(this + 0x1c20) != 2 {
                    m.set_i32(this + 0x1c20, 2);
                    *changed = true;
                }
            };
            if sv == 0x15 {
                if i!(0x1c20) != 1 && i!(0x1c1c) != 1 {
                    set20(m, 1);
                    changed = true;
                }
            } else if sv == 0x16 {
                if i!(0x1c1c) != 2 {
                    to_two(m, &mut changed);
                }
            } else if sv < 0x16 || i!(0x1c18) + 0x16 <= sv {
                if sv == 100 {
                    if i!(0x1c1c) < 2 {
                        to_two(m, &mut changed);
                    }
                } else if sv == 0x14 {
                    if i!(0x1c1c) != 0 && i!(0x1c20) != 0 {
                        set20(m, 0);
                        changed = true;
                    }
                } else {
                    changed = i!(0x1c24) == 0;
                    if changed {
                        let g = i!(0x1c1c);
                        set20(m, g);
                    }
                    let v = i!(0x1c20) + sv;
                    set20(m, v);
                }
            } else {
                let target = sv - 0x14;
                if i!(0x1c1c) != target && i!(0x1c20) != target {
                    set20(m, target);
                    changed = true;
                }
            }
            if i!(0x1c20) < 0 || i!(0x1c18) + 1 <= i!(0x1c20) {
                let g = i!(0x1c1c);
                m.set_i32(this + 0x1c20, g);
            } else if changed {
                m.set_i32(this + 0x1c24, sv);
                m.set_f32(this + 0x1c2c, t);
                let v = f!(0x1c84);
                m.set_f32(this + 0x1c90, v);
            }
        }

        // Carry out a shift in progress.
        m.set_i32(this + 0x3c34, 0);
        if i!(0x1c24) != 0 {
            let target = i!(0x1c20);
            let duration = if target == 1 {
                f!(0x1c38)
            } else if i!(0x1c14) == 0 {
                f!(0x1c30)
            } else {
                f!(0x1c34)
            };
            let elapsed = r(t as f64 - f!(0x1c2c) as f64);
            if elapsed <= duration {
                let k = r(elapsed as f64 / duration as f64);
                let v = r(k as f64 * f!(0x1c88) as f64 + (1.0 - k as f64) * f!(0x1c90) as f64);
                m.set_f32(this + 0x1c84, v);
                filter::add(m, this + 0x1b50, v);
                let fv = filter::value(m, this + 0x1b50);
                m.set_i32(this + 0x1c1c, 1);
                m.set_f32(this + 0x1c84, fv);
                let c = i!(0x1c24);
                m.set_i32(this + 0x3c34, c);
            } else {
                m.set_i32(this + 0x1c1c, target);
                if i!(0x1c18) < target {
                    let v = i!(0x1c18);
                    m.set_i32(this + 0x1c1c, v);
                }
                if i!(0x1c1c) < 0 {
                    m.set_i32(this + 0x1c1c, 0);
                }
                let v = r(f!(0x1c88) as f64 * f!(0x29d8) as f64);
                let v = r(v as f64 + v as f64);
                m.set_i32(this + 0x1c24, 0);
                m.set_f32(this + 0x1c2c, 0.0);
                m.set_f32(this + 0x1c3c, t);
                m.set_f32(this + 0x1c84, v);
                filter::add(m, this + 0x1b50, v);
                let fv = filter::value(m, this + 0x1b50);
                m.set_f32(this + 0x1c84, fv);
                m.set_i32(this + 0x1c20, -1);
            }
        }

        let torque = fn_004ff4b0(m, this, f!(0x1c84), f!(0x29d8));
        let at_wheels = fn_004ff550(m, this, torque);
        m.set_f32(this + 0x1c94, at_wheels);
        if f!(0x1c40) <= r(t as f64 - f!(0x1c3c) as f64) {
            if i!(0x1c14) == 0 && i!(0x1c1c) != 1 && f!(0x1c84) < r(f!(0x1c88) as f64 * 0.1f64 as f32 as f64) {
                m.set_i32(this + 0x277c, -1);
            }
            m.set_f32(this + 0x1c3c, 0.0);
        }
        m.set_i32(this + 0x34cc, 0);
        if r(t as f64 - f!(0x17d8) as f64) < 2.0 && f!(0xdc8) < 10.0 && 0.5 < f!(0x29d8) as f64 {
            m.set_i32(this + 0x34cc, 1);
        }
        let dt = r(globals::dt_hi(m));
        if i!(0x2760) != 0 {
            let level = ts::value(m, this + 0x3fc0) as f64;
            let use_ = ((f!(0x1c84).abs() as f64 * f!(0x29d8) as f64) / f!(0x1c88) as f64) * level * dt as f64;
            ts::consume(m, this + 0x3ef0, use_ as f32);
        }
        if 0.0 < ts::value(m, this + 0x3ef0) {
            m.set_i32(this + 0x3c7c, 0);
            m.set_i32(this + 0x1b24, 2);
        } else {
            m.set_f32(this + 0x1c94, 0.0);
            m.set_i32(this + 0x34cc, 0);
            m.set_i32(this + 0x3c7c, 1);
            m.set_i32(this + 0x1b24, 3);
        }
        let fuel = ts::value(m, this + 0x3ef0);
        m.set_f32(this + 0x3c74, fuel);
    } else {
        m.set_f32(this + 0x1c94, 0.0);
        m.set_f32(this + 0x1c84, 0.0);
        filter::add(m, this + 0x1b50, 0.0);
        let fv = filter::value(m, this + 0x1b50);
        m.set_f32(this + 0x1c84, fv);
        m.set_i32(this + 0x34cc, 0);
        m.set_i32(this + 0x1c1c, 1);
    }

    // Common tail.
    let (a, b, c) = (i!(0x1c20), f!(0x1c2c), f!(0x1c3c));
    m.set_i32(this + 0x3be4, a);
    m.set_f32(this + 0x3c10, b);
    m.set_f32(this + 0x3c14, c);
    let torque_in = f!(0x1c94);
    filter::add(m, this + 0x1b30, torque_in);
    let filtered = filter::value(m, this + 0x1b30);
    m.set_f32(this + 0x1c94, filtered);

    if i!(0x1b24) == 2 && i!(0x277c) == -1 && 0.6f32 as f64 * 4.0 < r(t as f64 - f!(0x1b28) as f64) as f64 {
        m.set_i32(this + 0x1b24, 3);
    }
    if i!(0x1b24) == 3 {
        let waited = r(t as f64 - f!(0x1b2c) as f64);
        let dt = globals::dt_hi(m);
        if waited as f64 <= 2.98f32 as f64 - dt {
            m.set_i32(this + 0x277c, 0);
            return;
        }
    } else {
        m.set_f32(this + 0x1b2c, t);
        if i!(0x277c) == 1 {
            m.set_i32(this + 0x1b24, 1);
        } else if i!(0x1b24) == 0 {
            m.set_f32(this + 0x1b28, t);
        }
        if i!(0x1b24) != 1
            || (((r(t as f64 - f!(0x1b28) as f64) as f64) <= 0.6f32 as f64 || s!(0x2740) != 1 || i!(0x277c) != 0)
                && s!(0x2740) != 0)
        {
            m.set_i32(this + 0x277c, 0);
            return;
        }
        if 0.0 < ts::value(m, this + 0x3ef0) {
            m.set_i32(this + 0x1b24, 2);
            m.set_i32(this + 0x277c, 0);
            return;
        }
    }
    m.set_i32(this + 0x1b24, 0);
    m.set_i32(this + 0x277c, 0);
}


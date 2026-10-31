//! State setters of the engine's car object (`Car_V`, `__thiscall`, `this` = the car): the velocity and angular
//! velocity entry points `scene_return_to_last` reaches through `0x4f1dd0` / `0x4f1e20` and the derived values
//! they refresh (`0x4f0530`, `0x4f0720`, `0x4f07e0`). Intermediate values stay in `f64` where the original keeps
//! them on the x87 stack, and are rounded where it does `fstp dword`.
use crate::globals::VEC_ZERO;
use crate::math::vec_len;
use crate::mem::Mem;

fn f(m: &Mem, a: u32) -> f64 {
    m.f32(a) as f64
}

/// `0x4f0530`: from the velocity `+0x40` and the direction `+0xd0`: speed `+0xdc0`, the cross product `+0x9f0`
/// with its length `+0xdc4`, the projection `+0xdc8` on the axis `+0xa38` (`+0xa20`) and the unit cross
/// product `+0x9fc` (the axis itself when the length is not above 0.001).
pub fn car_derive_velocity(m: &mut Mem, c: u32) {
    let len = vec_len(m, c + 0x40);
    let prev = f(m, c + 0xdc0);
    // reset when |prev - len| > 5, or the speed is below 1 (or unordered)
    let reset = (prev - len).abs() > 5.0 || !(len >= 1.0);
    if reset {
        for i in 0..3 {
            let z = m.u32(VEC_ZERO + 4 * i);
            m.set_u32(c + 0xc90 + 4 * i, z);
            m.set_u32(c + 0xcd8 + 4 * i, z);
        }
    }
    m.set_f32(c + 0xdc0, len as f32);
    let v = [f(m, c + 0x40), f(m, c + 0x44), f(m, c + 0x48)];
    let d = [f(m, c + 0xd0), f(m, c + 0xd4), f(m, c + 0xd8)];
    let x = v[1] * d[2] - v[2] * d[1];
    let y = d[0] * v[2] - d[2] * v[0];
    let z = d[1] * v[0] - d[0] * v[1];
    m.set_f32(c + 0x9f0, (z * d[1] - y * d[2]) as f32);
    m.set_f32(c + 0x9f4, (x * d[2] - z * d[0]) as f32);
    m.set_f32(c + 0x9f8, (y * d[0] - x * d[1]) as f32);
    let len2 = vec_len(m, c + 0x9f0);
    m.set_f32(c + 0xdc4, len2 as f32);
    let o = [f(m, c + 0x9f0), f(m, c + 0x9f4), f(m, c + 0x9f8)];
    let a = [f(m, c + 0xa38), f(m, c + 0xa3c), f(m, c + 0xa40)];
    let dot = o[2] * a[2] + o[1] * a[1] + o[0] * a[0];
    m.set_f32(c + 0xdc8, dot as f32);
    m.set_f32(c + 0xa20, (dot * a[0]) as f32);
    m.set_f32(c + 0xa24, (dot * a[1]) as f32);
    m.set_f32(c + 0xa28, (dot * a[2]) as f32);
    if !(len2 <= 0.001) {
        for i in 0..3 {
            m.set_f32(c + 0x9fc + 4 * i, (o[i as usize] / len2) as f32);
        }
    } else {
        for i in 0..3 {
            let w = m.u32(c + 0xa38 + 4 * i);
            m.set_u32(c + 0x9fc + 4 * i, w);
        }
    }
}

/// `0x4f0720`: publish the derived values to the network-visible block (`+0x2a50..`, `+0x3328..`, wheel words
/// `+0x3bb0 + 4 i` from `+0x3bb0 - 0x1540 + 4 i`).
pub fn car_publish(m: &mut Mem, c: u32) {
    let copy = |m: &mut Mem, to: u32, from: u32| {
        let v = m.u32(c + from);
        m.set_u32(c + to, v);
    };
    for (to, from) in [(0x2a50, 0x40), (0x2a54, 0x44), (0x2a58, 0x48), (0x2a5c, 0x9f0), (0x2a60, 0x9f4), (0x2a64, 0x9f8),
        (0x2a68, 0xa20), (0x2a6c, 0xa24), (0x2a70, 0xa28), (0x332c, 0xdc4), (0x3328, 0xdc0), (0x3330, 0xdc8), (0x3338, 0xee0), (0x333c, 0xee4)]
    {
        copy(m, to, from);
    }
    for i in 0..m.i32(c + 0xdd4).max(0) as u32 {
        copy(m, 0x3bb0 + 4 * i, 0x3bb0 - 0x1540 + 4 * i);
    }
}

/// `0x4f07e0`: `+0x2a74..` = `+0x58..`.
pub fn car_publish_angular(m: &mut Mem, c: u32) {
    for i in 0..3 {
        let v = m.u32(c + 0x58 + 4 * i);
        m.set_u32(c + 0x2a74 + 4 * i, v);
    }
}

/// `0x4f1dd0(vec)`: the velocity entry. `check(vec)` stands for the error report `0x4e2821(vec, "Car_V::set V", 100.0)`.
pub fn car_set_velocity(m: &mut Mem, c: u32, vec: u32, check: impl FnOnce(&mut Mem, u32)) {
    check(m, vec);
    for i in 0..3 {
        let v = m.u32(vec + 4 * i);
        m.set_u32(c + 0x40 + 4 * i, v);
    }
    car_derive_velocity(m, c);
    car_publish(m, c);
}

/// `0x4f1e20(vec)`: the angular entry. `+0x4c = vec`; `+0x58 = (vec rotated by +0x900) / +0x1f48` componentwise.
pub fn car_set_angular(m: &mut Mem, c: u32, vec: u32) {
    for i in 0..3 {
        let v = m.u32(vec + 4 * i);
        m.set_u32(c + 0x4c + 4 * i, v);
    }
    let v = [f(m, c + 0x4c), f(m, c + 0x50), f(m, c + 0x54)];
    let l = [
        (f(m, c + 0x918) * v[2] + f(m, c + 0x90c) * v[1] + v[0] * f(m, c + 0x900)) as f32,
        (f(m, c + 0x91c) * v[2] + f(m, c + 0x904) * v[0] + f(m, c + 0x910) * v[1]) as f32,
        (f(m, c + 0x920) * v[2] + f(m, c + 0x908) * v[0] + f(m, c + 0x914) * v[1]) as f32,
    ];
    for i in 0..3 {
        let q = l[i as usize] / m.f32(c + 0x1f48 + 4 * i);
        m.set_f32(c + 0x58 + 4 * i, q);
    }
    car_publish_angular(m, c);
}

#[cfg(test)]
mod tests {
    use super::*;
    fn unhex(s: &str) -> Vec<u8> {
        (0..s.len()).step_by(2).map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap()).collect()
    }
    #[test]
    fn native_car_state() {
        let (car, vec) = (0x2a010000u32, 0x2a030000u32);
        let (mut n, mut checks) = (0, 0);
        for (c, line) in include_str!("../../../tools/emu/golden_car_state.tsv").lines().enumerate() {
            let v: Vec<&str> = line.split('\t').collect();
            let mut m = Mem::new();
            crate::globals::install_constants(&mut m);
            for it in v[1].split(',').filter(|s| !s.is_empty()) {
                let (o, x) = it.split_once(':').unwrap();
                m.write(car + u32::from_str_radix(o, 16).unwrap(), &unhex(x));
            }
            m.write(vec, &unhex(v[2]));
            let before = m.read(car, 0x3c00);
            let mut checked = false;
            match v[0] {
                "vel" => car_set_velocity(&mut m, car, vec, |_, _| checked = true),
                "ang" => car_set_angular(&mut m, car, vec),
                "der" => car_derive_velocity(&mut m, car),
                _ => unreachable!(),
            }
            if checked {
                checks += 1;
            }
            let after = m.read(car, 0x3c00);
            let mut diff = vec![];
            for i in (0..0x3c00).step_by(4) {
                if after[i..i + 4] != before[i..i + 4] {
                    let w: String = after[i..i + 4].iter().map(|b| format!("{b:02x}")).collect();
                    diff.push(format!("{i:x}:{w}"));
                }
            }
            assert_eq!(diff.join(","), v[3], "case {c} ({})", v[0]);
            n += 1;
        }
        assert_eq!(n, 400);
        assert!(checks > 0);
    }
}

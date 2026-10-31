//! `0x54d0b0` (`__fastcall`, ecx = vehicle scene): fills the request slots of the scene (flags at `+0x5468`,
//! levels at `+0x5518`, 3-float positions at `+0x5494`) from the impact level, the wheel loads and the
//! cargo state, then sends them with the network message `0x546360` (`notify`, an external owner).
//!
//! The x87 stack of the original carries values between the blocks (the level `L` stays in `st0`, the
//! cross product component `cy` is added unrounded), so the arithmetic here follows the assembly with `f64`
//! temporaries and an `as f32` only where the original does `fstp dword`.
use crate::instruments::world_object_by_id;
use crate::mem::Mem;

fn len3(v: [f32; 3]) -> f64 {
    let (x, y, z) = (v[0] as f64, v[1] as f64, v[2] as f64);
    (z * z + y * y + x * x).sqrt()
}

pub fn scene_requests(m: &mut Mem, s: u32, notify: &mut dyn FnMut(&mut Mem, u32)) {
    let f = |m: &Mem, o: u32| m.f32(s + o) as f64;
    let net = m.u32(s + 0x5460);
    if net == 0 || m.i32(net + 0x34ac) != 1 {
        return;
    }
    let mut sent = false;
    if m.u32(s + 0x38c) != 0 {
        sent = true;
        // rotate by the matrix rows and divide by the scale `+0x1a4`
        let v = [
            (f(m, 0x2a0) * f(m, 0x1e4) + f(m, 0x2ac) * f(m, 0x1e8) + f(m, 0x294) * f(m, 0x1e0)) as f32,
            (f(m, 0x2a4) * f(m, 0x1e4) + f(m, 0x2b0) * f(m, 0x1e8) + f(m, 0x298) * f(m, 0x1e0)) as f32,
            (f(m, 0x2a8) * f(m, 0x1e4) + f(m, 0x2b4) * f(m, 0x1e8) + f(m, 0x29c) * f(m, 0x1e0)) as f32,
        ];
        let out = [v[0] / m.f32(s + 0x1a4), v[1] / m.f32(s + 0x1a8), v[2] / m.f32(s + 0x1ac)];
        let a = [
            (f(m, 0x1b0) - f(m, 0x1d4)) as f32,
            (f(m, 0x1b4) - f(m, 0x1d8)) as f32,
            (f(m, 0x1b8) - f(m, 0x1dc)) as f32,
        ];
        let b = [
            (f(m, 0x1bc) - out[0] as f64) as f32,
            (f(m, 0x1c0) - out[1] as f64) as f32,
            (f(m, 0x1c4) - out[2] as f64) as f32,
        ];
        let len1 = len3(a) as f32;
        let len2 = len3(b);
        let level = ((len2 * 5.0f32 as f64 + len1 as f64) * 0.1f32 as f64) as f32;
        let mut l = if world_object_by_id(m, m.u32(s + 0x414)) != 0 {
            if !(level >= 1.0) {
                let d = level as f64 + level as f64;
                d.sqrt().sqrt()
            } else {
                level as f64
            }
        } else {
            level as f64 - 0.01
        };
        if !(l >= 0.05) {
            return;
        }
        let (c0, c1, c2) = (f(m, 0x3d4), f(m, 0x3d8), f(m, 0x3dc));
        let px = (f(m, 0x27c) * c2 + f(m, 0x270) * c1 + c0 * f(m, 0x264) + f(m, 0x288)) as f32;
        let py = (f(m, 0x268) * c0 + f(m, 0x280) * c2 + f(m, 0x274) * c1 + f(m, 0x28c)) as f32;
        let pz = f(m, 0x26c) * c0 + f(m, 0x284) * c2 + f(m, 0x278) * c1 + f(m, 0x290);
        let (pxd, pyd) = (px as f64, py as f64);
        let cx = (pz * f(m, 0x1c0) - pyd * f(m, 0x1c4)) as f32;
        let cy = pxd * f(m, 0x1c4) - pz * f(m, 0x1bc);
        let cz = (pyd * f(m, 0x1bc) - pxd * f(m, 0x1c0)) as f32;
        let qx = (cx as f64 + f(m, 0x1b0)) as f32;
        let qy = cy + f(m, 0x1b4);
        let qz = cz as f64 + f(m, 0x1b8);
        let qxd = qx as f64;
        let rx = (qz * f(m, 0x21c) + qy * f(m, 0x210) + qxd * f(m, 0x204)) as f32;
        let ry = (qz * f(m, 0x220) + qy * f(m, 0x214) + qxd * f(m, 0x208)) as f32;
        let rz = (qz * f(m, 0x224) + qy * f(m, 0x218) + qxd * f(m, 0x20c)) as f32;
        if l > 1.0 {
            l = 1.0f32 as f64;
        }
        let dot = (f(m, 0x3e8) * rz as f64 + f(m, 0x3e4) * ry as f64 + f(m, 0x3e0) * rx as f64).abs();
        let thr = f(m, 0x388) * 0.2;
        let pos = [m.u32(s + 0x3d4), m.u32(s + 0x3d8), m.u32(s + 0x3dc)];
        let put = |m: &mut Mem, at: u32| {
            for i in 0..3 {
                m.set_u32(s + at + 4 * i, pos[i as usize]);
            }
        };
        if thr > dot {
            m.set_u32(s + 0x5474, 1);
            m.set_u32(s + 0x5524, 0x3f800000);
            put(m, 0x54b8);
        }
        let apx = pxd.abs();
        if pyd > apx && (len1 as f64) < 5.0 {
            m.set_f32(s + 0x5518, l as f32);
            put(m, 0x5494);
            m.set_u32(s + 0x5468, 1);
        } else if -apx > pyd {
            m.set_f32(s + 0x5520, l as f32);
            put(m, 0x54ac);
            m.set_u32(s + 0x5470, 1);
        } else {
            m.set_f32(s + 0x551c, l as f32);
            put(m, 0x54a0);
            m.set_u32(s + 0x546c, 1);
        }
    }
    if m.u32(net + 0x3344) == 0 {
        sent = true;
        let count = m.i32(s + 0x28bc);
        let mut any = false;
        let mut sum = 0.0f64;
        if count > 0 {
            let scale = m.f32(s + 0x19c) as f64 * 10.0f32 as f64;
            for i in 0..count as u32 {
                let w = m.i32(net + 0x3be8 + 4 * i);
                if w > 0 {
                    any = true;
                }
                sum += w as f64 / scale;
            }
            if any {
                if sum > 1.0f32 as f64 {
                    sum = 1.0;
                }
                m.set_f32(s + 0x5534, sum as f32);
                m.set_u32(s + 0x5484, 1);
                for i in 0..3 {
                    let v = m.u32(s + 0x228 + 4 * i);
                    m.set_u32(s + 0x54e8 + 4 * i, v);
                }
            }
        }
    }
    let kind = m.u32(s + 0x594);
    if matches!(kind, 1 | 4 | 5 | 6) {
        let slot = if kind == 5 || kind == 6 { 9 } else { 8 };
        m.set_u32(s + 0x5468 + slot * 4, 1);
        let v = m.f32(s + 0x598);
        let level = if v > 1.0 { 0x3f800000 } else { m.u32(s + 0x598) };
        m.set_u32(s + 0x5518 + slot * 4, level);
        for i in 0..3 {
            let p = m.u32(s + 0x3d4 + 4 * i);
            m.set_u32(s + 0x5494 + slot * 12 + 4 * i, p);
        }
        notify(m, s);
        return;
    }
    if sent {
        notify(m, s);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_scene_requests() {
        let hex = |s: &str| u32::from_str_radix(s, 16).unwrap();
        let (scene, net) = (0x2a010000u32, 0x2a020000u32);
        let mut n = 0;
        for (c, line) in include_str!("../../../tools/emu/golden_scene_requests.tsv").lines().enumerate() {
            let v: Vec<&str> = line.split('\t').collect();
            let mut m = Mem::new();
            // the float fields come as little-endian byte hex, the integer ones as numbers
            for it in v[0].split(',').filter(|s| !s.is_empty()) {
                let (o, x) = it.split_once(':').unwrap();
                let o = hex(o);
                let val = if matches!(o, 0x38c | 0x414 | 0x594 | 0x5460 | 0x28bc) || (0x5468..0x5540).contains(&o) {
                    hex(x)
                } else {
                    let b = (0..4).map(|i| u8::from_str_radix(&x[2 * i..2 * i + 2], 16).unwrap()).collect::<Vec<_>>();
                    u32::from_le_bytes([b[0], b[1], b[2], b[3]])
                };
                m.set_u32(scene + o, val);
            }
            for it in v[1].split(',').filter(|s| !s.is_empty()) {
                let (o, x) = it.split_once(':').unwrap();
                m.set_u32(net + hex(o), hex(x));
            }
            let elems: Vec<u32> = v[2].split(',').filter(|s| !s.is_empty()).map(hex).collect();
            for (i, &a) in elems.iter().enumerate() {
                m.set_u32(a + 8, if i + 1 < elems.len() { elems[i + 1] + 4 } else { 0 });
            }
            m.set_u32(0x6f653c, if elems.is_empty() { 0 } else { elems[0] + 4 });
            let mut calls = 0;
            scene_requests(&mut m, scene, &mut |_, _| calls += 1);
            assert_eq!(calls.to_string(), v[4], "case {c}: calls");
            for it in v[5].split(',') {
                let (o, x) = it.split_once(':').unwrap();
                assert_eq!(m.u32(scene + hex(o)), hex(x), "case {c}: +{o}");
            }
            n += 1;
        }
        assert_eq!(n, 600);
    }
}

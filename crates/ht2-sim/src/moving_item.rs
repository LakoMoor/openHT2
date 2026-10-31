//! Velocity setters of the engine's moving item (`MovingItem`, the body object at `scene+0x10`, `__thiscall`), called
//! by `scene_return_to_last` as `0x5812a0` and `0x5816d0`. The range reports (`0x4e2821(vec, text, limit)`, with a
//! `sprintf`-built message that only the report reads) are an external owner: `report(vec_address, limit)`.
use crate::globals::VEC_ZERO;
use crate::math::vec_len;
use crate::mem::Mem;

/// `0x5812a0(vec, name)` ("MovingItem::set V"): `+0x1a0 = vec`, report of `+0x1a0` against 150, `+0xce4 = +0x1a0`,
/// speed `|+0x1a0|` to `+0x378` and `+0xd84`.
pub fn moving_item_set_v(m: &mut Mem, this: u32, vec: u32, report: impl FnOnce(&mut Mem, u32, f32)) {
    for i in 0..3 {
        let v = m.u32(vec + 4 * i);
        m.set_u32(this + 0x1a0 + 4 * i, v);
    }
    report(m, this + 0x1a0, 150.0);
    for i in 0..3 {
        let v = m.u32(this + 0x1a0 + 4 * i);
        m.set_u32(this + 0xce4 + 4 * i, v);
    }
    let len = vec_len(m, this + 0x1a0) as f32;
    m.set_f32(this + 0x378, len);
    m.set_f32(this + 0xd84, len);
}

/// `0x5816d0(vec, name)` ("MovingItem::set w"): `+0x1b8 = vec`, `+0x1ac = (vec rotated by the rows `+0x254`, `+0x258`,
/// `+0x25c` ...) / +0x194` componentwise, report of `+0x1ac` against 50, `+0xcf0 = +0x1ac`, `+0xcfc = +0x1b8`.
pub fn moving_item_set_w(m: &mut Mem, this: u32, vec: u32, report: impl FnOnce(&mut Mem, u32, f32)) {
    for i in 0..3 {
        let v = m.u32(vec + 4 * i);
        m.set_u32(this + 0x1b8 + 4 * i, v);
    }
    let f = |m: &Mem, o: u32| m.f32(this + o) as f64;
    let w = [f(m, 0x1b8), f(m, 0x1bc), f(m, 0x1c0)];
    let l = [
        (f(m, 0x26c) * w[2] + f(m, 0x260) * w[1] + f(m, 0x254) * w[0]) as f32,
        (f(m, 0x270) * w[2] + f(m, 0x264) * w[1] + f(m, 0x258) * w[0]) as f32,
        (f(m, 0x274) * w[2] + f(m, 0x268) * w[1] + f(m, 0x25c) * w[0]) as f32,
    ];
    for i in 0..3 {
        let q = l[i as usize] / m.f32(this + 0x194 + 4 * i);
        m.set_f32(this + 0x1ac + 4 * i, q);
    }
    report(m, this + 0x1ac, 50.0);
    for i in 0..3 {
        let a = m.u32(this + 0x1ac + 4 * i);
        m.set_u32(this + 0xcf0 + 4 * i, a);
        let b = m.u32(this + 0x1b8 + 4 * i);
        m.set_u32(this + 0xcfc + 4 * i, b);
    }
}

/// `0x580ce0` (virtual `+0x88`, `__thiscall`): one integration step with the frame length `d` (`0x4e2749`, full precision):
/// `+0x1a0 += d * (+0x314, +0x318, +0x31c)` (the x product stays unrounded on the FPU, y and z are stored as `f32` first),
/// report of `+0x1a0` against 150, `+0xc4c = d * +0x1a0`, `+0xc58 = d * +0x1ac` (each stored as `f32`).
pub fn moving_item_step(m: &mut Mem, this: u32, report: impl FnOnce(&mut Mem, u32, f32)) {
    let d = crate::globals::dt_hi(m);
    let f = |m: &Mem, o: u32| m.f32(this + o) as f64;
    let a = d * f(m, 0x314);
    let b = (d * f(m, 0x318)) as f32 as f64;
    let c = (d * f(m, 0x31c)) as f32 as f64;
    let x = (a + f(m, 0x1a0)) as f32;
    let y = (b + f(m, 0x1a4)) as f32;
    let z = (c + f(m, 0x1a8)) as f32;
    m.set_f32(this + 0x1a0, x);
    m.set_f32(this + 0x1a4, y);
    m.set_f32(this + 0x1a8, z);
    for i in 0..3 {
        let v = (d * f(m, 0x1a0 + 4 * i)) as f32;
        m.set_f32(this + 0xc4c + 4 * i, v);
    }
    for i in 0..3 {
        let v = (d * f(m, 0x1ac + 4 * i)) as f32;
        m.set_f32(this + 0xc58 + 4 * i, v);
    }
    report(m, this + 0x1a0, 150.0);
}

/// What `0x580f80` calls outside itself: the basis check `0x58e7f0(matrix, text, 0)` (only logs and range reports, no state
/// effects as read from the disassembly: stand-in), and the virtuals `[[this+0x20]]+0x24` and `[this]+0x28`.
pub trait PoseOwner {
    fn check(&mut self, m: &mut Mem, this: u32, matrix: u32);
    /// `[node->vtable + 0x24]()` with `ecx = node` (`[this+0x20]`).
    fn node_notify(&mut self, m: &mut Mem, node: u32);
    /// `[this->vtable + 0x28](a, b)`.
    fn apply(&mut self, m: &mut Mem, this: u32, a: u32, b: u32);
}

/// `0x580f80(matrix, value)` (virtual `+0x90`/`+0x94` of the MovingItem tables, `__thiscall`): check of the basis, then with
/// the byte `+0x46c` set only the notifications (`apply(0, this+0x46c)`), otherwise the transform of the body node
/// `[[this+0x20]+0x44]` is set to `matrix` (`0x5f2190`, mode 0), `+0x38 = value` and `apply(value, 0)`.
pub fn moving_item_pose(m: &mut Mem, this: u32, matrix: u32, value: u32, owner: &mut dyn PoseOwner) {
    owner.check(m, this, matrix);
    let node = m.u32(this + 0x20);
    if m.rd8(this + 0x46c) != 0 {
        owner.node_notify(m, node);
        owner.apply(m, this, 0, this + 0x46c);
        return;
    }
    let body = m.u32(node + 0x44);
    crate::instruments::scene_node_set_transform(m, body, matrix, 0);
    owner.node_notify(m, node);
    m.set_u32(this + 0x38, value);
    owner.apply(m, this, value, 0);
}

/// A virtual call `[this->vtable + slot](args)` of the moving item (not ported yet: stand-in owner).
pub trait ItemVirtuals {
    fn virt(&mut self, m: &mut Mem, this: u32, slot: u32, args: &[u32]);
}

/// `0x581230(a, b)` ("place"): when the float `b+0x2c` is below -2000 (or NaN) it becomes -1000 and the virtuals
/// `+0x90(VEC_ZERO, "...")` and `+0x94(VEC_ZERO, "...")` are called first; then `+0x88(a, b)` and `+0x3c()`.
pub fn moving_item_place(m: &mut Mem, this: u32, a: u32, b: u32, owner: &mut dyn ItemVirtuals) {
    let z = m.f32(b + 0x2c) as f64;
    if !(z >= -2000.0) {
        m.set_u32(b + 0x2c, 0xc47a_0000);
        owner.virt(m, this, 0x90, &[VEC_ZERO, 0x67d4cc]);
        owner.virt(m, this, 0x94, &[VEC_ZERO, 0x67d4c0]);
    }
    owner.virt(m, this, 0x88, &[a, b]);
    owner.virt(m, this, 0x3c, &[]);
}

#[cfg(test)]
mod tests {
    use super::*;
    fn unhex(s: &str) -> Vec<u8> {
        (0..s.len()).step_by(2).map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap()).collect()
    }
    #[test]
    fn native_moving_item() {
        let (this, vec) = (0x2a010000u32, 0x2a030000u32);
        let mut n = 0;
        for (c, line) in include_str!("../../../tools/emu/golden_moving_item.tsv").lines().enumerate() {
            let v: Vec<&str> = line.split('\t').collect();
            let mut m = Mem::new();
            for it in v[1].split(',').filter(|s| !s.is_empty()) {
                let (o, x) = it.split_once(':').unwrap();
                m.write(this + u32::from_str_radix(o, 16).unwrap(), &unhex(x));
            }
            m.write(vec, &unhex(v[2]));
            let before = m.read(this, 0x1000);
            let reported = std::cell::RefCell::new(String::new());
            let rep = |m: &mut Mem, p: u32, limit: f32| {
                *reported.borrow_mut() = format!("{}|{:08x}", m.read(p, 12).iter().map(|b| format!("{b:02x}")).collect::<String>(), limit.to_bits());
            };
            if v[0] == "v" {
                moving_item_set_v(&mut m, this, vec, rep);
            } else {
                moving_item_set_w(&mut m, this, vec, rep);
            }
            let after = m.read(this, 0x1000);
            let mut diff = vec![];
            for i in (0..0x1000).step_by(4) {
                if after[i..i + 4] != before[i..i + 4] {
                    diff.push(format!("{i:x}:{}", after[i..i + 4].iter().map(|b| format!("{b:02x}")).collect::<String>()));
                }
            }
            assert_eq!(diff.join(","), v[3], "case {c}");
            assert_eq!(*reported.borrow(), v[4], "case {c}: report");
            n += 1;
        }
        assert_eq!(n, 300);
    }

    #[test]
    fn native_moving_item_place() {
        struct Log(Vec<String>);
        impl ItemVirtuals for Log {
            fn virt(&mut self, _: &mut Mem, this: u32, slot: u32, args: &[u32]) {
                let a: Vec<String> = args.iter().map(|x| format!("{x:x}")).collect();
                self.0.push(format!("{slot:x}|{this:x}|{}", a.join(",")));
            }
        }
        let (this, a, b) = (0x2a010000u32, 0x2a012000u32, 0x2a013000u32);
        let mut n = 0;
        for (c, line) in include_str!("../../../tools/emu/golden_moving_item_place.tsv").lines().enumerate() {
            let v: Vec<&str> = line.split('\t').collect();
            let mut m = Mem::new();
            m.write(b,&unhex(v[0]));
            let mut log = Log(vec![]);
            moving_item_place(&mut m, this, a, b, &mut log);
            assert_eq!(log.0.join(";"), v[1], "case {c}");
            let z: String = m.read(b + 0x2c, 4).iter().map(|x| format!("{x:02x}")).collect();
            assert_eq!(z, v[2], "case {c}: z");
            n += 1;
        }
        assert_eq!(n, 200);
    }

    #[test]
    fn native_moving_item_step() {
        let this = 0x2a010000u32;
        let clock = 0x2a030000u32;
        let mut n = 0;
        for (c, line) in include_str!("../../../tools/emu/golden_moving_item_step.tsv").lines().enumerate() {
            let v: Vec<&str> = line.split('\t').collect();
            let mut m = Mem::new();
            for it in v[0].split(',').filter(|s| !s.is_empty()) {
                let (o, x) = it.split_once(':').unwrap();
                m.write(this + u32::from_str_radix(o, 16).unwrap(), &unhex(x));
            }
            m.write(0x6974c2, &(v[1].parse::<i16>().unwrap()).to_le_bytes());
            m.set_u32(0x6974c4, clock);
            m.write(clock + 0xb90, &unhex(v[2]));
            let before = m.read(this, 0x1000);
            let reported = std::cell::RefCell::new(String::new());
            moving_item_step(&mut m, this, |m, p, limit| {
                *reported.borrow_mut() = format!("{}|{:08x}", m.read(p, 12).iter().map(|b| format!("{b:02x}")).collect::<String>(), limit.to_bits());
            });
            let after = m.read(this, 0x1000);
            let mut diff = vec![];
            for i in (0..0x1000).step_by(4) {
                if after[i..i + 4] != before[i..i + 4] {
                    diff.push(format!("{i:x}:{}", after[i..i + 4].iter().map(|b| format!("{b:02x}")).collect::<String>()));
                }
            }
            assert_eq!(diff.join(","), v[3], "case {c}");
            assert_eq!(*reported.borrow(), v[4], "case {c}: report");
            n += 1;
        }
        assert_eq!(n, 300);
    }

    #[test]
    fn native_moving_item_pose() {
        struct Log(Vec<String>);
        impl PoseOwner for Log {
            fn check(&mut self, _: &mut Mem, this: u32, matrix: u32) {
                self.0.push(format!("chk|{this:x}|{matrix:x}|0"));
            }
            fn node_notify(&mut self, _: &mut Mem, node: u32) {
                self.0.push(format!("24|{node:x}"));
            }
            fn apply(&mut self, _: &mut Mem, this: u32, a: u32, b: u32) {
                self.0.push(format!("28|{this:x}|{a:x}|{b:x}"));
            }
        }
        let base = 0x2a010000u32;
        let (obj, aux, node, mat) = (base, base + 0x2000, base + 0x8000, base + 0xa000);
        let hex = |m: &Mem, a: u32, n: u32| (0..n).map(|i| format!("{:02x}", m.rd8(a + i))).collect::<String>();
        let mut n = 0;
        for (c, line) in include_str!("../../../tools/emu/golden_moving_item_pose.tsv").lines().enumerate() {
            let v: Vec<&str> = line.split('\t').collect();
            let mut m = Mem::new();
            m.set_u32(obj + 0x20, aux);
            m.set_u32(aux + 0x44, node);
            m.wr8(obj + 0x46c, v[0].parse().unwrap());
            m.write(mat, &unhex(v[1]));
            let (ob, nb) = (m.read(obj, 0x1000), m.read(node, 0x100));
            let mut log = Log(vec![]);
            moving_item_pose(&mut m, obj, mat, u32::from_str_radix(v[2], 16).unwrap(), &mut log);
            let diff = |b: &[u8], a: &Mem, at: u32| {
                (0..b.len()).step_by(4).filter(|&i| m.read(at + i as u32, 4) != b[i..i + 4]).map(|i| format!("{i:x}:{}", hex(a, at + i as u32, 4))).collect::<Vec<_>>().join(",")
            };
            assert_eq!(diff(&ob, &m, obj), v[3], "case {c}: object");
            assert_eq!(diff(&nb, &m, node), v[4], "case {c}: node");
            assert_eq!(log.0.join(";"), v[5], "case {c}: events");
            n += 1;
        }
        assert_eq!(n, 200);
    }
}

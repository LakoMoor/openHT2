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
}

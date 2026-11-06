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

/// The callees of the collision query dispatcher `0x58dff0` (the body's collision machinery, not ported: stand-ins).
pub trait CollideOwner {
    /// `0x5843c0(this, a, b, c, rec)` (cdecl), mode bit 4.
    fn query_road(&mut self, m: &mut Mem, this: u32, args: [u32; 4]) -> u32;
    /// `0x5855a0(this, a, b, c, rec, mode)`, mode bit 2.
    fn query_objects(&mut self, m: &mut Mem, this: u32, args: [u32; 4], mode: u32) -> u32;
    /// `0x58b120(this, a, b, c, rec, mode)`.
    fn query_other(&mut self, m: &mut Mem, this: u32, args: [u32; 4], mode: u32) -> u32;
    /// `0x58cf30` (thiscall) after a successful `0x58b120`.
    fn after_other(&mut self, m: &mut Mem, this: u32);
    /// The debug trace of the road branch (log text and `0x4e9ce4` / `0x4e9d90`), only when `[0x6f6620]` is set.
    fn trace(&mut self, m: &mut Mem, this: u32, rec: u32);
}

/// `0x58dff0(a, b, c, rec, mode)` (virtual `+0x94` of the tables of the 7 MovingItem classes, `thiscall`, `ret 0x14`):
/// dispatches by `mode`: bit 4 to the road query (with the debug trace when `[0x6f6620] != 0`, the result has bit 0 and
/// `[rec+0x74] == 0`), else bit 2 to the object query, else to `0x58b120` followed by `0x58cf30` when it returned non-zero.
pub fn collide_dispatch(m: &mut Mem, this: u32, a: u32, b: u32, c: u32, rec: u32, mode: u32, owner: &mut dyn CollideOwner) -> u32 {
    let args = [a, b, c, rec];
    if mode & 4 != 0 {
        let r = owner.query_road(m, this, args);
        if m.u32(0x6f6620) != 0 && r & 1 != 0 && m.u32(rec + 0x74) == 0 {
            owner.trace(m, this, rec);
        }
        return r;
    }
    if mode & 2 != 0 {
        return owner.query_objects(m, this, args, mode);
    }
    let r = owner.query_other(m, this, args, mode);
    if r != 0 {
        owner.after_other(m, this);
    }
    r
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

/// What `0x587e20` calls outside itself (stand-ins, not ported): the magnitude `0x5ab4d0(this+0x1c4)` (an `fld` result),
/// the registry lookup `0x5dbec0`, the flags `0x56f210`, the binding `0x56f120`, the error log `0x4e260c` and the clock
/// `0x4e27a5` (an `fld` result).
pub trait BindOwner {
    fn magnitude(&mut self, m: &mut Mem, vec: u32) -> f32;
    /// `0x5dbec0` (thiscall on `[0x6ef1fc]`): looks the key `[this+0x320]` and the 12 bytes `pos` up; `None` leaves the handle -1.
    fn find(&mut self, m: &mut Mem, key: u32, pos: [u32; 3], callback: u32) -> Option<u32>;
    fn flags(&mut self, m: &mut Mem, handle: u32) -> u32;
    fn attach(&mut self, m: &mut Mem, handle: u32, kind: u32, this: u32);
    fn error(&mut self);
    fn now(&mut self) -> f32;
}

/// `0x587e20(kind, what, _, a3, a4, vec, extra)` (thiscall, `ret 0x1c`): for `kind == 2` stores `what` at `+0x4b8` and by `what`
/// records the request (`1/4/5/6`: `a3` (non-zero) at `+0x4c0`, `a4` at `+0x4bc`, 12 bytes of `vec` at `+0x4c4`, for 5 a dword
/// of `extra` at `+0x4d0`, for 6 two consecutive C strings of `extra` at `+0x4d0`; `2`: `+0x4bc = *extra`; `3`: when
/// the magnitude of `+0x1c4` is >= 5 looks the object up and, depending on the low half of `*extra`, binds it); then sets
/// bit 2 of `+0x37c` and stores the clock at `+0x388`. Returns 1, or 0 for another `kind` or an unknown `what`.
pub fn moving_item_bind(m: &mut Mem, this: u32, args: [u32; 7], owner: &mut dyn BindOwner) -> u32 {
    if args[0] != 2 {
        return 0;
    }
    let what = args[1];
    m.set_u32(this + 0x4b8, what);
    if !matches!(what, 1..=6) {
        m.set_u32(this + 0x4b8, 0);
        return 0;
    }
    let (a3, a4, vec, extra) = (args[3], args[4], args[5], args[6]);
    match what {
        2 => {
            let v = m.u32(extra);
            m.set_u32(this + 0x4bc, v);
        }
        3 => {
            let mag = owner.magnitude(m, this + 0x1c4) as f64;
            if mag >= 5.0 {
                let esi = m.u32(extra) & 0xffff;
                if esi > 2 {
                    owner.error();
                }
                let pos = [m.u32(this + 0x218), m.u32(this + 0x21c), m.u32(this + 0x220)];
                let key = m.u32(this + 0x320);
                match owner.find(m, key, pos, 0x587e00) {
                    None => owner.error(),
                    Some(h) => {
                        let fl = owner.flags(m, h);
                        let mask = match esi {
                            0 => 3,
                            1 => 0xc,
                            2 => 0x30,
                            _ => 0xff_ffff,
                        };
                        if esi <= 2 && fl & mask & 0xff == 0 {
                            owner.attach(m, h, esi, this);
                        }
                    }
                }
            }
        }
        _ => {
            if a3 != 0 {
                m.set_u32(this + 0x4c0, a3);
                m.set_u32(this + 0x4bc, a4);
                for i in 0..3 {
                    let w = m.u32(vec + 4 * i);
                    m.set_u32(this + 0x4c4 + 4 * i, w);
                }
                if what == 5 {
                    let w = m.u32(extra);
                    m.set_u32(this + 0x4d0, w);
                } else if what == 6 {
                    let mut at = 0;
                    for _ in 0..2 {
                        loop {
                            let b = m.rd8(extra + at);
                            m.wr8(this + 0x4d0 + at, b);
                            at += 1;
                            if b == 0 {
                                break;
                            }
                        }
                    }
                }
            }
        }
    }
    let f = m.u32(this + 0x37c);
    m.set_u32(this + 0x37c, f | 4);
    let t = owner.now();
    m.set_u32(this + 0x388, t.to_bits());
    1
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

    #[test]
    fn native_collide_dispatch() {
        struct Log(Vec<String>, u32);
        fn h(a: &[u32]) -> String {
            a.iter().map(|x| format!("{x:x}")).collect::<Vec<_>>().join(",")
        }
        impl CollideOwner for Log {
            fn query_road(&mut self, _: &mut Mem, this: u32, a: [u32; 4]) -> u32 {
                self.0.push(format!("q4|{this:x},{}", h(&a)));
                self.1
            }
            fn query_objects(&mut self, _: &mut Mem, this: u32, a: [u32; 4], mode: u32) -> u32 {
                self.0.push(format!("q2|{this:x},{},{mode:x}", h(&a)));
                self.1
            }
            fn query_other(&mut self, _: &mut Mem, this: u32, a: [u32; 4], mode: u32) -> u32 {
                self.0.push(format!("q0|{this:x},{},{mode:x}", h(&a)));
                self.1
            }
            fn after_other(&mut self, _: &mut Mem, this: u32) {
                self.0.push(format!("after|{this:x}"));
            }
            fn trace(&mut self, _: &mut Mem, _: u32, _: u32) {
                self.0.push("trace".into());
            }
        }
        let (this, d) = (0x2a010000u32, 0x2a012000u32);
        let mut n = 0;
        for (c, line) in include_str!("../../../tools/emu/golden_collide_dispatch.tsv").lines().enumerate() {
            let v: Vec<&str> = line.split('\t').collect();
            let mut m = Mem::new();
            m.set_u32(0x6f6620, v[1].parse().unwrap());
            m.set_u32(d + 0x74, v[2].parse().unwrap());
            let a: Vec<u32> = v[3].split(',').map(|x| u32::from_str_radix(x, 16).unwrap()).collect();
            let mut log = Log(vec![], u32::from_str_radix(v[4], 16).unwrap());
            let r = collide_dispatch(&mut m, this, a[0], a[1], a[2], a[3], v[0].parse().unwrap(), &mut log);
            assert_eq!(log.0.join(";"), v[5], "case {c}");
            assert_eq!(format!("{r:x}"), v[6], "case {c}: result");
            n += 1;
        }
        assert_eq!(n, 300);
    }
    #[test]
    fn native_moving_item_bind() {
        struct Log {
            ev: Vec<String>,
            mag: f32,
            now: f32,
            handle: u32,
            flags: u32,
        }
        impl BindOwner for Log {
            fn magnitude(&mut self, _: &mut Mem, _: u32) -> f32 {
                self.mag
            }
            fn find(&mut self, _: &mut Mem, key: u32, p: [u32; 3], cb: u32) -> Option<u32> {
                self.ev.push(format!("find|{key:x}|{:x},{:x},{:x}|{cb:x}", p[0], p[1], p[2]));
                (self.handle != 0xffff_ffff).then_some(self.handle)
            }
            fn flags(&mut self, _: &mut Mem, h: u32) -> u32 {
                self.ev.push(format!("flags|{h:x}"));
                self.flags
            }
            fn attach(&mut self, _: &mut Mem, h: u32, k: u32, t: u32) {
                self.ev.push(format!("attach|{h:x},{k:x},{t:x}"));
            }
            fn error(&mut self) {
                self.ev.push("err".into());
            }
            fn now(&mut self) -> f32 {
                self.now
            }
        }
        let this = 0x2a010000u32;
        let (p5, p6) = (0x2a012010u32, 0x2a012040u32);
        let mut n = 0;
        for (c, line) in include_str!("../../../tools/emu/golden_item_bind.tsv").lines().enumerate() {
            let v: Vec<&str> = line.split('\t').collect();
            let f = |s: &str| f32::from_bits(u32::from_le_bytes(unhex(s).try_into().unwrap()));
            let hx = |s: &str| u32::from_str_radix(s, 16).unwrap();
            let mut m = Mem::new();
            let pre: Vec<&str> = v[10].split('|').collect();
            m.write(this + 0x218, &unhex(pre[1]));
            m.write(this + 0x320, &unhex(pre[2]));
            m.write(this + 0x37c, &unhex(pre[3]));
            m.write(this + 0x4b0, &unhex(pre[4]));
            m.write(p5, &unhex(v[8]));
            m.write(p6, &unhex(v[9]));
            let before = m.read(this, 0x600);
            let mut log = Log { ev: vec![], mag: f(v[2]), now: f(v[3]), handle: hx(v[4]), flags: hx(v[5]) };
            let args = [v[0].parse().unwrap(), v[1].parse().unwrap(), 0, hx(v[6]), hx(v[7]), p5, p6];
            let ret = moving_item_bind(&mut m, this, args, &mut log);
            let after = m.read(this, 0x600);
            let diff: Vec<String> = (0..0x600)
                .step_by(4)
                .filter(|&i| after[i..i + 4] != before[i..i + 4])
                .map(|i| format!("{i:x}:{}", after[i..i + 4].iter().map(|b| format!("{b:02x}")).collect::<String>()))
                .collect();
            assert_eq!(diff.join(","), v[11], "case {c}");
            assert_eq!(format!("{ret:x}"), v[12], "case {c}: ret");
            assert_eq!(log.ev.join(";"), v[13], "case {c}: events");
            n += 1;
        }
        assert_eq!(n, 600);
    }
}

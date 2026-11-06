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

/// `0x583c30(flags)` (MovingItem virtual `+0xcc`, thiscall, `ret 4`): unless `[+0x598] > [+0x690]` (signed) or `flags & 4`, the
/// displacement `(+0x1a0..+0x1a8) - (+0x7e4..+0x7ec)` times the `f32` at `+0x18c` is stored at `+0x7c0` (x from the unrounded
/// difference, y/z from the `f32`-rounded ones); with `[+0x688] & 0x2000000` it is scaled by 0.1; then the virtuals
/// `+0xa4(V3, D)` (`V3` = the words `+0x5ec..`) and `+0x74()` are called (stand-in owner).
pub fn moving_item_reset_step(m: &mut Mem, this: u32, flags: u32, owner: &mut dyn ItemVirtuals) {
    if (m.i32(this + 0x598)) > m.i32(this + 0x690) || flags & 4 != 0 {
        return;
    }
    const SCRATCH: u32 = 0x7ffe_f000;
    let (d, v3) = (SCRATCH + 0xc00, SCRATCH + 0xc10);
    let f = |m: &Mem, a: u32| m.f32(a) as f64;
    let s = f(m, this + 0x18c);
    let a = f(m, this + 0x1a0) - f(m, this + 0x7e4);
    let b = (f(m, this + 0x1a4) - f(m, this + 0x7e8)) as f32 as f64;
    let c = (f(m, this + 0x1a8) - f(m, this + 0x7ec)) as f32 as f64;
    let (x, y, z) = ((a * s) as f32, (b * s) as f32, (c * s) as f32);
    for (i, v) in [x, y, z].iter().enumerate() {
        m.set_u32(this + 0x7c0 + 4 * i as u32, v.to_bits());
        m.set_u32(d + 4 * i as u32, v.to_bits());
        let w = m.u32(this + 0x5ec + 4 * i as u32);
        m.set_u32(v3 + 4 * i as u32, w);
    }
    if m.u32(this + 0x688) & 0x200_0000 != 0 {
        crate::matrix::vec_scale_in_place(m, d, SCRATCH + 0xc30, f32::from_bits(0x3dcc_cccd));
    }
    owner.virt(m, this, 0xa4, &[v3, d]);
    owner.virt(m, this, 0x74, &[]);
}

/// `0x4e2fb4(mat, o1..o6)` (cdecl): `0x5de950` on the matrix at `mat`; the six doubles (position, three angles) are stored as `f32`
/// through the output pointers.
pub fn matrix_pose_f32(m: &mut Mem, mat: u32, outs: [u32; 6]) {
    let t: [f32; 12] = core::array::from_fn(|i| m.f32(mat + 4 * i as u32));
    let d = 0x7ffe_f000u32 + 0xd00;
    crate::chassis::fn_005de950(m, t, d, d + 8, d + 16, d + 24, d + 32, d + 40);
    for (i, o) in outs.iter().enumerate() {
        let v = m.f64(d + 8 * i as u32) as f32;
        m.set_f32(*o, v);
    }
}

/// `0x5818d0(a, b, m2, out_pos, out_ang)` (cdecl): `out_pos = a.t - b.t` (`f32` per component) and the angles of `a * m2`
/// (`0x5aba40`, `0x4e2fb4`): `out_ang = (-p17, p18, p16)` with `p16..p18` the angles of `0x5de950`.
pub fn pose_delta(m: &mut Mem, a: u32, b: u32, m2: u32, out_pos: u32, out_ang: u32) {
    for k in 0..3u32 {
        let v = (m.f32(a + 0x24 + 4 * k) as f64 - m.f32(b + 0x24 + 4 * k) as f64) as f32;
        m.set_f32(out_pos + 4 * k, v);
    }
    let (tmp, junk) = (0x7ffe_f000u32 + 0xe00, 0x7ffe_f000u32 + 0xe40);
    crate::matrix::mat_mul(m, tmp, a, m2);
    matrix_pose_f32(m, tmp, [junk, junk + 4, junk + 8, out_ang + 8, out_ang, out_ang + 4]);
    let x = m.f32(out_ang);
    m.set_f32(out_ang, -x);
}

/// What `0x582c30` calls outside itself (stand-ins, not ported): the world-object lookup `0x57c9e0`, the room lookup `0x4e5ba7`,
/// the contact routines `0x57e2e0` / `0x57e540` / `0x580a00` / `0x5808c0` (`which` is the address), `0x581990`, `0x588090`, the
/// range report `0x4e2821(vec, text, limit_bits)` and the error log `0x4e260c`. The virtuals `+0x1c/+0x90/+0x94/+0xa0/+0xdc` go
/// through `ItemVirtuals`.
pub trait UpdateOwner: ItemVirtuals {
    fn range(&mut self, m: &mut Mem, vec: u32, text: u32, limit: u32);
    fn world_object(&mut self, m: &mut Mem, id: u32) -> u32;
    fn room(&mut self, m: &mut Mem, a: u32, b: u32, c: u32) -> u32;
    fn contact(&mut self, m: &mut Mem, which: u32, a: u32, b: u32, c: u32);
    fn contact_1990(&mut self, m: &mut Mem, a: u32, b: u32, c: u32, d: u32);
    fn r588090(&mut self, m: &mut Mem, this: u32) -> u32;
    fn error(&mut self);
}

/// `0x582c30(flags, dt)` (MovingItem virtual `+0xcc`, thiscall, `ret 8`): the per-step pose / velocity update. Saves the
/// previous frame into the history ring, advances the pose by `dt` (`0x5abc90` look-at roll and translation from either the
/// stored velocities or the pose delta), sweeps the new pose (`0x5820c0`), pushes the body out along the hit normal, refreshes
/// the room, the wheel velocities and the contact bookkeeping (see the body for the order).
pub fn moving_item_update(m: &mut Mem, this: u32, flags: u32, dt: f32, o: &mut dyn UpdateOwner) {
    use crate::matrix::*;
    const L: u32 = 0x7ffe_f000 + 0x3000;
    let (a_v, b_v, t1, m1, m2, m3, min_t, tarr) = (L, L + 0x10, L + 0x20, L + 0x60, L + 0x90, L + 0xc0, L + 0xf0, L + 0x100);
    let f = |m: &Mem, a: u32| m.f32(a) as f64;
    let copy = |m: &mut Mem, dst: u32, src: u32, n: u32| {
        for i in 0..n {
            let w = m.u32(src + 4 * i);
            m.set_u32(dst + 4 * i, w);
        }
    };
    let k = m.i32(this + 0x598);
    let kk = k as u32;
    copy(m, this + 0x7f8 + 12u32.wrapping_mul(kk), this + 0x1ac, 3);
    copy(m, this + 0xd0u32.wrapping_mul(kk) + 0x838, this + 0x5b8, 0x34);
    o.range(m, this + 0x5ec, 0x67d788, 0x461c_4000);
    o.range(m, this + 0x5f8, 0x67d76c, 0x4000_0000);
    if m.u32(this + 0x688) != 0 {
        o.virt(m, this, 0xa0, &[this + 0x5b8]);
    }
    if m.u32(this + 0x404) == 0 {
        if f(m, this + 0x3d8) > 0.7 && m.u32(this + 0x37c) & 4 == 0 {
            m.set_u32(this + 0x68c, 1);
        }
        if m.u32(this + 0x404) == 0 && m.u32(this + 0x3f4) != 0 && m.u32(m.u32(this + 0x3f4)) == 1 && m.u32(this + 0x37c) & 4 == 0 {
            copy(m, this + 0x498, this + 0x3d0, 3);
            copy(m, this + 0x48c, this + 0x3c4, 3);
        }
    }
    let fr = 1.0 - dt as f64;
    let l64 = fr as f32;
    m.set_f32(this + 0x5b0, dt);
    let v = (fr * f(m, this + 0x5ac)) as f32;
    m.set_f32(this + 0x5ac, v);
    copy(m, this + 0x460, this + 0x3d0, 3);
    copy(m, this + 0x390, this + 0x5b8, 0x34);
    let v = m.u32(this + 0x37c) | 1;
    m.set_u32(this + 0x37c, v);
    let now = crate::globals::time(m);
    m.set_f32(this + 0x388, now);
    let v = m.u32(this + 0xd88) | 1;
    m.set_u32(this + 0xd88, v);
    m.set_u32(this + 0x69c, 0);
    copy(m, a_v, this + 0xc4c, 3);
    copy(m, b_v, this + 0xc58, 3);
    let saved = m.u32(this + 0x5a4);
    let dtf = dt as f64;
    let b3 = |m: &Mem, a: u32| [m.f32(a), m.f32(a + 4), m.f32(a + 8)];
    if m.u32(this + 0x68c) != 0 {
        let len = vec_len(m, this + 0xc58) as f32;
        m.set_f32(this + 0x5a4, len);
        let a = b3(m, this + 0xc4c);
        mat_translation(m, t1, (dtf * a[0] as f64) as f32, (dtf * a[1] as f64) as f32, (dtf * a[2] as f64) as f32);
        let th = (dtf * f(m, this + 0x5a4)) as f32;
        let dir = b3(m, this + 0xc58);
        mat_look(m, m1, [0.0; 3], dir, th);
        mat_mul(m, m2, m1, this + 0x760);
    } else {
        pose_delta(m, this + 0x6a0, this + 0x224, this + 0x284, a_v, b_v);
        let lb = vec_len(m, b_v) as f32;
        let a = b3(m, a_v);
        mat_translation(m, t1, (dtf * a[0] as f64) as f32, (dtf * a[1] as f64) as f32, (dtf * a[2] as f64) as f32);
        let th = (dtf * lb as f64) as f32;
        let dir = b3(m, b_v);
        mat_look(m, m1, [0.0; 3], dir, th);
        mat_mul(m, m2, m1, this + 0x224);
    }
    mat_mul(m, m3, m2, t1);
    copy(m, this + 0x700, m3, 12);
    let obj = o.world_object(m, m.u32(this + 0x404));
    let hits = crate::body_ray::sweep_hits(m, this, this + 0x700, min_t, tarr);
    o.range(m, this + 0x724, 0x67d74c, 0x461c_4000);
    if hits != 0 {
        let mt = f(m, min_t);
        let mut v1 = if m.u32(this + 0x68c) != 0 { (0.001 - mt) as f32 } else { (0.01 - mt) as f32 };
        let (d, vv, uu, ww) = (L + 0x200, L + 0x210, L + 0x220, L + 0x230);
        copy(m, d, this + 0x3d0, 3);
        o.virt(m, this, 0xdc, &[vv]);
        let (d0, d1, d2) = (f(m, d), f(m, d + 4), f(m, d + 8));
        let (v0, vy, vz) = (f(m, vv), f(m, vv + 4), f(m, vv + 8));
        let dot = (d0 * v0 + d1 * vy) + d2 * vz;
        let c = [d2 * vy - d1 * vz, d0 * vz - d2 * v0, d1 * v0 - d0 * vy];
        let e = [(c[1] * vz - c[2] * vy) as f32, (c[2] * v0 - c[0] * vz) as f32, (c[0] * vy - c[1] * v0) as f32];
        copy(m, uu, d, 3);
        if !(dot >= -0.7) {
            v1 = 0.0;
        } else if dot < 0.0 {
            for i in 0..3 {
                m.set_f32(ww + 4 * i, e[i as usize]);
            }
            let inv = 1.0 / vec_len(m, ww);
            for i in 0..3 {
                m.set_f32(uu + 4 * i, (e[i as usize] as f64 * inv) as f32);
            }
            let p = inv * v1 as f64;
            v1 = if p > 0.5 { 0.5 } else { p as f32 };
        }
        for i in 0..3 {
            let x = (f(m, uu + 4 * i) * v1 as f64) as f32;
            m.set_f32(ww + 4 * i, x);
        }
        vec_add_in_place(m, this + 0x724, vv, ww);
        if crate::carv::fn_004e28b0(m, this + 0x724, 10000.0) != 0 {
            o.error();
        }
    }
    copy(m, this + 0xc4c, a_v, 3);
    copy(m, this + 0xc58, b_v, 3);
    m.set_u32(this + 0x5a4, saved);
    crate::math::mat_inverse(m, m1, this + 0x700);
    copy(m, this + 0x730, m1, 12);
    let r = o.room(m, m.u32(this + 0x7f4), this + 0x784, this + 0x724);
    m.set_u32(this + 0x7f0, r);
    if r == 0 {
        let w = m.u32(this + 0x7f4);
        m.set_u32(this + 0x7f0, w);
        o.virt(m, this, 0x1c, &[1]);
    }
    let (p1, p2, junk) = (L + 0x240, L + 0x250, L + 0x260);
    let fl = l64 as f64;
    for (p, s, w) in [(p1, 0x1a0u32, 0x1dcu32), (p2, 0x1ac, 0x1e8)] {
        for i in 0..2u32 {
            let q = (fl * f(m, this + w + 4 * i)) as f32 as f64;
            m.set_f32(p + 4 * i, (f(m, this + s + 4 * i) - q) as f32);
        }
        let z = f(m, this + s + 8) - fl * f(m, this + w + 8);
        m.set_f32(p + 8, z as f32);
    }
    vec_scale_in_place(m, this + 0x1dc, junk, dt);
    vec_scale_in_place(m, this + 0x1e8, junk, dt);
    o.virt(m, this, 0x90, &[p1, 0x67d670]);
    o.virt(m, this, 0x94, &[p2, 0x67d650]);
    if k > m.i32(this + 0x690) {
        return;
    }
    if m.i32(this + 0x688) > 0 {
        copy(m, this + 0xc64, this + 0x700, 12);
        copy(m, this + 0xc94, this + 0x730, 12);
    } else {
        copy(m, this + 0xc64, this + 0x224, 12);
        copy(m, this + 0xc94, this + 0x284, 12);
    }
    copy(m, this + 0x7e4, this + 0x1a0, 3);
    if flags & 4 != 0 {
        o.contact(m, 0x57e2e0, this, this + 0x390, this + 0x1f4);
        return;
    }
    if m.u32(this + 0x188) == 1 && m.u32(this + 0x404) == 0 {
        o.contact(m, 0x57e540, this, this + 0x390, this + 0x1f4);
        return;
    }
    if obj != 0 {
        if m.u32(this + 0x698) != 0 {
            let (a, b) = (m.u32(this + 0x688), m.u32(this + 0x5b0));
            o.contact_1990(m, this, this + 0x390, a, b);
        }
        let v = m.u32(this + 0xd88) | 8;
        m.set_u32(this + 0xd88, v);
        return;
    }
    if m.u32(this + 0x37c) & 4 != 0 {
        let v = m.u32(this + 0xd88) | 4;
        m.set_u32(this + 0xd88, v);
        if o.r588090(m, this) == 0 {
            o.contact(m, 0x580a00, this, this + 0x3c4, this + 0x3d0);
        }
        m.set_u32(this + 0x69c, 1);
        let v = m.u32(this + 0x37c) & !4;
        m.set_u32(this + 0x37c, v);
        return;
    }
    let v = m.u32(this + 0xd88) | 0x10;
    m.set_u32(this + 0xd88, v);
    // q: the difference vector (f32 per component); the normalised copy made by 0x5ab400 is never read.
    let q = [
        (f(m, this + 0x3c4) - f(m, this + 0xc88)) as f32 as f64,
        (f(m, this + 0x3c8) - f(m, this + 0xc8c)) as f32 as f64,
        (f(m, this + 0x3cc) - f(m, this + 0xc90)) as f32 as f64,
    ];
    let (qa, qb) = (L + 0x280, L + 0x290);
    for i in 0..3 {
        m.set_f32(qa + 4 * i, q[i as usize] as f32);
    }
    vec_unit(m, qb, qa);
    let g = |m: &Mem, o: u32| f(m, this + o);
    let (a1, a2, a3) = (g(m, 0x1ac), g(m, 0x1b0), g(m, 0x1b4));
    let e0 = (g(m, 0xc70) * a2 + g(m, 0xc7c) * a3) + a1 * g(m, 0xc64);
    let e1 = (g(m, 0xc74) * a2 + g(m, 0xc80) * a3) + g(m, 0xc68) * a1;
    let e2 = (g(m, 0xc78) * a2 + g(m, 0xc84) * a3) + g(m, 0xc6c) * a1;
    let cx = (q[2] * e1 - q[1] * e2) as f32 as f64;
    let cy = (e2 * q[0] - q[2] * e0) as f32 as f64;
    let cz = e0 * q[1] - e1 * q[0];
    let ux = (cx + g(m, 0x1a0)) as f32 as f64;
    let uy = (cy + g(m, 0x1a4)) as f32 as f64;
    let uz = cz + g(m, 0x1a8);
    let s = ((uz * q[2] + ux * q[0]) + uy * q[1]) as f32 as f64;
    let (c3, c4) = (this + 0x3c4, this + 0x3d0);
    let gate = flags & 8 == 0
        && m.u32(this + 0x688) & 0x400_0000 == 0
        && crate::globals::time_hi(m) - g(m, 0xc48) >= 1.0
        && g(m, 0x3d8) >= 0.6
        && m.u32(this + 0x58c) == 0
        && !(s > 0.0);
    if gate {
        o.contact(m, 0x5808c0, this, c3, c4);
        return;
    }
    o.contact(m, 0x580a00, this, c3, c4);
    let kc = m.i32(this + 0x598);
    let mut esi = kc;
    if kc > 0 {
        esi = kc - 1;
        loop {
            let id = m.u32(this + 0xd0u32.wrapping_mul(esi as u32) + 0x8ac);
            if o.world_object(m, id) == 0 {
                break;
            }
            esi -= 1;
            if esi < 0 {
                esi = m.i32(this + 0x598);
                break;
            }
        }
    }
    let kc = m.i32(this + 0x598);
    if esi == kc {
        return;
    }
    let (rk, re) = (this + 0xd0u32.wrapping_mul(kc as u32), this + 0xd0u32.wrapping_mul(esi as u32));
    let dq = L + 0x2a0;
    for i in 0..3 {
        let x = (f(m, rk + 0x86c + 4 * i) - f(m, re + 0x86c + 4 * i)) as f32;
        m.set_f32(dq + 4 * i, x);
    }
    let dv = L + 0x2b0;
    vec_unit(m, dv, dq);
    let (d0, d1, d2) = (f(m, dv), f(m, dv + 4), f(m, dv + 8));
    let r = [f(m, rk + 0x878), f(m, rk + 0x87c), f(m, rk + 0x880)];
    let cx2 = (d1 * r[2] - d2 * r[1]) as f32 as f64;
    let cy2 = (d2 * r[0] - d0 * r[2]) as f32 as f64;
    let cz2 = (d0 * r[1] - d1 * r[0]) as f32 as f64;
    let t1v = (a3 * cz2 + a2 * cy2) + a1 * cx2;
    let vk = this + 0x7f8 + 12u32.wrapping_mul(kc as u32);
    let t2 = (f(m, vk + 8) * cz2 + f(m, vk + 4) * cy2) + cx2 * f(m, vk);
    if !(t2 * t1v < 0.0) {
        return;
    }
    if !(t1v.abs() < 0.1) {
        return;
    }
    let n = [cy2 * a3 - cz2 * a2, cz2 * a1 - cx2 * a3, cx2 * a2 - cy2 * a1];
    let mv = [
        (cz2 * n[1] - cy2 * n[2]) as f32,
        (n[2] * cx2 - cz2 * n[0]) as f32,
        (cy2 * n[0] - n[1] * cx2) as f32,
    ];
    let k01 = f32::from_bits(0x3dcc_cccd) as f64;
    let a0 = r[0] * k01;
    let r1 = (r[1] * k01) as f32 as f64;
    let r2 = (r[2] * k01) as f32 as f64;
    let (b1, b2, b3v) = (g(m, 0x1a0), g(m, 0x1a4), g(m, 0x1a8));
    let p0 = b3v * r[1] - r[2] * b2;
    let p1v = r[2] * b1 - r[0] * b3v;
    let p2v = r[0] * b2 - b1 * r[1];
    let q0 = (p1v * r[2] - p2v * r[1]) as f32 as f64;
    let q1 = (p2v * r[0] - p0 * r[2]) as f32 as f64;
    let q2 = (p0 * r[1] - p1v * r[0]) as f32 as f64;
    let pp = [(q0 + a0) as f32, (r1 + q1) as f32, (r2 + q2) as f32];
    let (mvp, ppp) = (L + 0x2c0, L + 0x2d0);
    for i in 0..3usize {
        m.set_f32(mvp + 4 * i as u32, mv[i]);
        m.set_f32(ppp + 4 * i as u32, pp[i]);
    }
    o.virt(m, this, 0x94, &[mvp, 0x67d640]);
    o.virt(m, this, 0x90, &[ppp, 0x67d630]);
}

/// `0x58cf30` (thiscall): the first `[+0xdf8]` flags at `+0xf28` become 1.
pub fn moving_item_flags_reset(m: &mut Mem, this: u32) {
    for i in 0..m.i32(this + 0xdf8).max(0) as u32 {
        m.set_u32(this + 0xf28 + 4 * i, 1);
    }
}

/// What `0x58cf60` calls outside itself (stand-ins): the context constructor `0x5ef5c0`, the cast `0x592a20(this, ctx, out)`
/// (`out[0]` is the distance) and the room lookup `0x4e5ba7`; the virtuals `+0x1c` / `+0x8c` go through `ItemVirtuals`.
pub trait ModeOwner: ItemVirtuals {
    fn ctx_init(&mut self, m: &mut Mem, ctx: u32);
    fn cast(&mut self, m: &mut Mem, this: u32, ctx: u32, out: u32) -> u32;
    fn room(&mut self, m: &mut Mem, a: u32, b: u32, c: u32) -> u32;
}

/// `0x58cf60(mode)` (thiscall, `ret 4`): switches `[+0xe04]`. With `[+0xf90] != 0` the mode is 0; the same mode returns, mode 1
/// is stored. Otherwise up to three times: cast, move the pose `+0x1f4` by `ctx[0x40..] * (distance + 0.01)`, look the room up
/// (`+0x1c(1)` and the old room when it fails) and `+0x8c(room, pose)`. Three successful casts end with `+0x8c(old room, old
/// pose)` and leave the mode; fewer store the mode.
pub fn moving_item_set_mode(m: &mut Mem, this: u32, mode: u32, o: &mut dyn ModeOwner) {
    const L: u32 = 0x7ffe_f000 + 0x4000;
    let mode = if m.u32(this + 0xf90) != 0 { 0 } else { mode };
    if mode == m.u32(this + 0xe04) {
        return;
    }
    if mode == 1 {
        m.set_u32(this + 0xe04, 1);
        return;
    }
    let (ctx, out, orig, work, w, tmp) = (L, L + 0x100, L + 0x110, L + 0x140, L + 0x170, L + 0x180);
    o.ctx_init(m, ctx);
    for i in 0..12 {
        let v = m.u32(this + 0x1f4 + 4 * i);
        m.set_u32(orig + 4 * i, v);
    }
    let room0 = m.u32(this + 0x320);
    let mut n = 0;
    while n < 3 {
        if o.cast(m, this, ctx, out) == 0 {
            break;
        }
        let d = m.f32(out) as f64 + 0.01;
        for i in 0..12 {
            let v = m.u32(this + 0x1f4 + 4 * i);
            m.set_u32(work + 4 * i, v);
        }
        for i in 0..3 {
            let x = (m.f32(ctx + 0x40 + 4 * i) as f64 * d) as f32;
            m.set_f32(w + 4 * i, x);
        }
        crate::matrix::vec_add_in_place(m, work + 0x24, tmp, w);
        let mut room = o.room(m, m.u32(this + 0x320), this + 0x218, work + 0x24);
        if room == 0 {
            o.virt(m, this, 0x1c, &[1]);
            room = m.u32(this + 0x320);
        }
        o.virt(m, this, 0x8c, &[room, work]);
        n += 1;
    }
    if n == 3 {
        o.virt(m, this, 0x8c, &[room0, orig]);
    } else {
        m.set_u32(this + 0xe04, mode);
    }
}

/// What `0x58ae30` calls outside itself (stand-ins): `0x58a020(neighbour, matrix, set, other, flag)` and the swept record
/// `0x584c70(p, q, rad_bits, set, body, mult)` (see `body_contact_record_swept`).
pub trait NeighbourOwner {
    fn pair_query(&mut self, m: &mut Mem, a: u32, mat: u32, set: u32, b: u32, flag: u32) -> u32;
    fn swept_record(&mut self, m: &mut Mem, p: u32, q: u32, rad: u32, set: u32, body: u32, mult: u32) -> u32;
}

/// `0x58ae30(body, matrix, set)` (cdecl): the body against its (up to six) neighbours at `+0xdb4` (the list ends at the first
/// null; the distance test `L(+0xdcc+4i) > ((|dpos|_1 * dt) + r_n + r_b)^2` ends it too). For a neighbour of mode 1 or 2 the matrix
/// `A0 = inverse(body^-1 * matrix)` (made once, with the first neighbour) is applied to its pose and `0x58a020` (mode 2) or
/// `0x584c70` (mode 1, the neighbour is stored at `set+0x5fc+4*count` on a contact) adds contacts, whose `+0xd04` slots get `A0`.
/// Then `0x58a020(body, matrix, set, neighbour, 1)`. Returns the sum of the results.
pub fn body_neighbours(m: &mut Mem, body: u32, mat: u32, set: u32, o: &mut dyn NeighbourOwner) -> u32 {
    use crate::matrix::*;
    const L: u32 = 0x7ffe_f000 + 0x5000;
    let (a0, a1, a2, nm, m2, prod, binv) = (L, L + 0x30, L + 0x60, L + 0x90, L + 0xc0, L + 0xf0, L + 0x120);
    for a in [a0, a1, a2] {
        for i in 0..12 {
            let v = m.u32(crate::globals::MAT_IDENTITY + 4 * i);
            m.set_u32(a + 4 * i, v);
        }
    }
    let dt = crate::globals::dt(m) as f64;
    let f = |m: &Mem, a: u32| m.f32(a) as f64;
    let copy = |m: &mut Mem, dst: u32, src: u32, n: u32| {
        for i in 0..n {
            let w = m.u32(src + 4 * i);
            m.set_u32(dst + 4 * i, w);
        }
    };
    let mut sum = 0u32;
    for i in 0..6u32 {
        let n = m.u32(body + 0xdb4 + 4 * i);
        if n == 0 {
            break;
        }
        let (dx, dy, dz) = (f(m, n + 0xce4) - f(m, body + 0xce4), f(m, n + 0xce8) - f(m, body + 0xce8), f(m, n + 0xcec) - f(m, body + 0xcec));
        let rr = (((dz.abs() + dy.abs()) + dx.abs()) * dt + f(m, n + 0xcc8)) + f(m, body + 0xcc8);
        if f(m, body + 0xdcc + 4 * i) > rr * rr {
            break;
        }
        let mode = m.u32(n + 0x188);
        if mode == 1 || mode == 2 {
            if i == 0 {
                let src = if m.u32(body + 0x5a0) != 0 { body + 0x730 } else { body + 0x254 };
                copy(m, a2, src, 12);
                mat_mul(m, prod, a2, mat);
                copy(m, a1, prod, 12);
                crate::math::mat_inverse(m, binv, a1);
                copy(m, a0, binv, 12);
            }
            let src = if m.u32(n + 0x5a0) != 0 { n + 0x700 } else { n + 0x1f4 };
            copy(m, nm, src, 12);
            mat_mul(m, m2, nm, a0);
            let before = m.i32(set + 0xb00);
            if mode == 2 {
                sum = sum.wrapping_add(o.pair_query(m, n, m2, set, body, u32::MAX));
            } else {
                let rad = m.u32(n + 0xcc8);
                let r = o.swept_record(m, nm + 0x24, m2 + 0x24, rad, set, body, u32::MAX);
                if r != 0 {
                    sum = sum.wrapping_add(r);
                    let c = m.u32(set + 0xb00);
                    m.set_u32(set + 0x5fc + 4 * c, n);
                }
            }
            let after = m.i32(set + 0xb00);
            for j in before..after {
                copy(m, set + 0xd04 + 0x30 * j as u32, a0, 12);
            }
        }
        sum = sum.wrapping_add(o.pair_query(m, body, mat, set, n, 1));
    }
    sum
}

/// What `0x589640` calls outside itself (stand-ins): the context constructor `0x5ef5c0`, the room lookup `0x4e5ba7`, the casts
/// `0x4e67d1` / `0x4e6ac4(room, p0, p1, ctx)` (the hit record is written to `ctx`) and the virtual `+0x1c` (`ItemVirtuals`).
pub trait PointCastOwner: ItemVirtuals {
    fn ctx_init(&mut self, m: &mut Mem, ctx: u32);
    fn room(&mut self, m: &mut Mem, a: u32, b: u32, c: u32) -> u32;
    fn cast(&mut self, m: &mut Mem, which: u32, room: u32, p0: u32, p1: u32, ctx: u32) -> u32;
}

/// `0x589640(body, matrix, set, flags)` (cdecl): for each sample point of the body (`0x58d0f0`, or the table variant `0x58d120` with
/// `flags & 0x200`) a segment is cast: `0x200` — from the body position to the point in world space, otherwise from the world point to
/// the point transformed by `matrix` (the room is looked up from the body position, `+0x1c(1)` when that fails). A miss may
/// be turned into a record without a second cast by the tests of `flags & 0x10` (the plane `+0x1420/+0x142c` of the first record,
/// the limit `+0x1418`, `[+0x1444]` selecting absolute distances) and `flags & 0x20` (the side plane `+0x142c x +0x1438`
/// against `+0x141c`); otherwise `0x4e6ac4` casts. Each point gives a flag at `set+0xb04+4n` (hit & 1) and scale 1.0; a hit makes the
/// scale `|hit - p0| / |ctx[0x58] - p0|` (at most 1, sqrt; 0 when the denominator is below 1e-9), copies the hit point /
/// normal / distance (`set+0xa00+4n = n . (hit - p1)`) / point index, sets `[+0x1444+4i]` / `[+0x1494+4i]` and copies the context
/// to `body+0x1418+0xd0i`. Stops at 63 entries (the iterator is drained). Returns the number of hits.
pub fn body_point_contacts(m: &mut Mem, body: u32, mat: u32, set: u32, flags: u32, o: &mut dyn PointCastOwner) -> u32 {
    use crate::matrix::*;
    const L: u32 = 0x7ffe_f000 + 0x6000;
    let (p, p0, p1, mm, ctx, u1, u2, v3, w, junk) = (L, L + 0x10, L + 0x20, L + 0x30, L + 0x80, L + 0x170, L + 0x190, L + 0x1a0, L + 0x1b0, L + 0x1c0);
    let f = |m: &Mem, a: u32| m.f32(a) as f64;
    let copy = |m: &mut Mem, dst: u32, src: u32, n: u32| {
        for i in 0..n {
            let x = m.u32(src + 4 * i);
            m.set_u32(dst + 4 * i, x);
        }
    };
    o.ctx_init(m, ctx);
    let room0 = if m.u32(body + 0x5a0) != 0 { m.u32(body + 0x7f0) } else { m.u32(body + 0x320) };
    copy(m, mm, if m.u32(body + 0x5a0) != 0 { body + 0x700 } else { body + 0x1f4 }, 12);
    let mut added = 0u32;
    loop {
        let idx = if flags & 0x200 != 0 {
            let (i, n) = (m.i32(body + 0xdfc), m.i32(body + 0xdf4));
            if i >= n {
                m.set_u32(body + 0xdfc, 0);
                0
            } else {
                copy(m, p, body + 0xe08 + 12 * i as u32, 3);
                m.set_u32(body + 0xdfc, (i + 1) as u32);
                (i + 1) as u32
            }
        } else {
            crate::body_ray::point_iter_next(m, body, p)
        };
        if idx == 0 {
            return added;
        }
        let xf = |m: &Mem, t: u32| -> [f32; 3] {
            let (px, py, pz) = (f(m, p), f(m, p + 4), f(m, p + 8));
            let g = |i: u32| f(m, t + 4 * i);
            [
                (((g(6) * pz + g(3) * py) + g(0) * px) + g(9)) as f32,
                (((g(7) * pz + g(4) * py) + g(1) * px) + g(10)) as f32,
                (((g(8) * pz + g(5) * py) + g(2) * px) + g(11)) as f32,
            ]
        };
        let wv = xf(m, mm);
        let mut room;
        if flags & 0x200 != 0 {
            copy(m, p0, mm + 0x24, 3);
            for i in 0..3 {
                m.set_f32(p1 + 4 * i, wv[i as usize]);
            }
            room = room0;
        } else {
            for i in 0..3 {
                m.set_f32(p0 + 4 * i, wv[i as usize]);
            }
            let nv = xf(m, mat);
            for i in 0..3 {
                m.set_f32(p1 + 4 * i, nv[i as usize]);
            }
            room = o.room(m, room0, mm + 0x24, p0);
        }
        if room == 0 {
            room = room0;
            o.virt(m, body, 0x1c, &[1]);
        }
        let mut hit = o.cast(m, 0x4e67d1, room, p0, p1, ctx);
        if hit == 0 {
            let mut do_cast = true;
            if flags & 0x10 != 0 {
                let r = [f(m, body + 0x1420), f(m, body + 0x1424), f(m, body + 0x1428)];
                let n = [f(m, body + 0x142c), f(m, body + 0x1430), f(m, body + 0x1434)];
                let side = |m: &Mem, q: u32| (((f(m, q + 8) - r[2]) * n[2] + (f(m, q + 4) - r[1]) * n[1]) + (f(m, q) - r[0]) * n[0]);
                let flag = m.u32(body + 0x1444);
                let mut d0 = side(m, p0);
                let mut d1 = side(m, p1) as f32 as f64;
                if flag != 0 {
                    d0 = d0.abs();
                    d1 = d1.abs();
                }
                let t = f(m, body + 0x1418) - 0.5;
                let mut early = false;
                if !(d0 >= t) && !(d1 >= t) {
                    if flag == 0 {
                        early = d0 > 0.5 && d1 > 0.5;
                    } else {
                        early = d0 > 0.0 && d1 > 0.0;
                    }
                }
                if !early && flags & 0x20 != 0 {
                    let u = [f(m, body + 0x1438), f(m, body + 0x143c), f(m, body + 0x1440)];
                    let wd = [(u[1] * n[2] - u[2] * n[1]) as f32, (u[2] * n[0] - n[2] * u[0]) as f32, (n[1] * u[0] - u[1] * n[0]) as f32];
                    for i in 0..3 {
                        m.set_f32(w + 4 * i, wd[i as usize]);
                    }
                    let s = (wd[2] as f64 * f(m, 0x702638) + wd[1] as f64 * f(m, 0x702634)) + wd[0] as f64 * f(m, 0x702630);
                    if !(s >= 0.0) {
                        vec_scale_in_place(m, w, junk, -1.0);
                    }
                    let (x1, y1, z1) = (f(m, p1) - r[0], f(m, p1 + 4) - r[1], f(m, p1 + 8) - r[2]);
                    let dot2 = (x1 * f(m, w) + z1 * f(m, w + 8)) + y1 * f(m, w + 4);
                    early = dot2 > f(m, body + 0x141c);
                }
                do_cast = !early;
            }
            if do_cast {
                hit = o.cast(m, 0x4e6ac4, room, p0, p1, ctx);
            }
        }
        let cnt = m.i32(set + 0xb00);
        if cnt >= 0x3f {
            while crate::body_ray::point_iter_next(m, body, p) != 0 {}
            return added;
        }
        let c = cnt as u32;
        let flag = hit & 1;
        m.set_u32(set + 0xb04 + 4 * c, flag);
        m.set_u32(set + 0x800 + 4 * c, 0x3f80_0000);
        if flag == 0 {
            continue;
        }
        m.set_u32(set + 0x800 + 4 * c, 0);
        added += 1;
        copy(m, set + 12 * c, ctx + 0x34, 3);
        copy(m, set + 12 * (c + 0x40), ctx + 0x40, 3);
        m.set_u32(set + 0xc04 + 4 * c, idx - 1);
        let fl = m.u32(body + 0x1444 + 4 * idx) | 1;
        m.set_u32(body + 0x1444 + 4 * idx, fl);
        m.set_u32(body + 0x1494 + 4 * idx, fl);
        copy(m, body + 0xd0 * idx + 0x1418, ctx, 0x34);
        let w74 = m.u32(ctx + 0x74);
        m.set_u32(set + 0x600 + 4 * c, w74);
        // u1 = ctx[0x58..] - p0, u2 = hit - p0, v3 = hit - p1 (f32 each)
        for i in 0..3u32 {
            let a = (f(m, ctx + 0x58 + 4 * i) - f(m, p0 + 4 * i)) as f32;
            m.set_f32(u1 + 4 * i, a);
            let b = (f(m, ctx + 0x34 + 4 * i) - f(m, p0 + 4 * i)) as f32;
            m.set_f32(u2 + 4 * i, b);
            let d = (f(m, ctx + 0x34 + 4 * i) - f(m, p1 + 4 * i)) as f32;
            m.set_f32(v3 + 4 * i, d);
        }
        let dist = ((f(m, v3 + 8) * f(m, ctx + 0x48) + f(m, v3 + 4) * f(m, ctx + 0x44)) + f(m, v3) * f(m, ctx + 0x40)) as f32;
        m.set_f32(set + 0xa00 + 4 * c, dist);
        let a = vec_dot(m, u1, u1) as f32;
        let b = vec_dot(m, u2, u2);
        if !(a as f64 >= 1e-9) {
            m.set_u32(set + 0xb00, c + 1);
            continue;
        }
        let mut q = b / a as f64;
        if !(q <= 1.0) {
            q = 1.0;
        }
        m.set_f32(set + 0x800 + 4 * c, q.sqrt() as f32);
        m.set_u32(set + 0xb00, c + 1);
    }
}

/// `0x589eb0(p*, q*)` (thiscall on the body, `ret 8`): the segment `p-q` taken into body space (`0x57dec0` inverse pose; the plane
/// coordinates `x' = Mi0 x + Mi6 z + Mi3 y + Mi9`, `y' = Mi1 x + Mi7 z + Mi4 y + Mi10`) against the box `+0x13f4/+0x13f8` (x', +-0.01)
/// and `+0x1400/+0x13fc` (y', +-0.01). 0 when both ends lie outside one side, else 1.
pub fn segment_box_test(m: &mut Mem, body: u32, p: u32, q: u32) -> u32 {
    let mi = if m.u32(body + 0x5a0) != 0 { body + 0x730 } else { body + 0x254 };
    let f = |m: &Mem, a: u32| m.f32(a) as f64;
    let g = |m: &Mem, i: u32| f(m, mi + 4 * i);
    let (px, py, pz) = (f(m, p), f(m, p + 4), f(m, p + 8));
    let (qx, qy, qz) = (f(m, q), f(m, q + 4), f(m, q + 8));
    let pxs = (((g(m, 0) * px + g(m, 6) * pz) + g(m, 3) * py) + g(m, 9)) as f32 as f64;
    let pys = ((g(m, 1) * px + g(m, 7) * pz) + g(m, 4) * py) + g(m, 10);
    let qxs = (((g(m, 6) * qz + g(m, 3) * qy) + g(m, 0) * qx) + g(m, 9)) as f32 as f64;
    let qys = (((g(m, 7) * qz + g(m, 4) * qy) + g(m, 1) * qx) + g(m, 10)) as f32 as f64;
    let a = f(m, body + 0x13fc) + 0.01;
    let c = (f(m, body + 0x1400) - 0.01) as f32 as f64;
    let d = (f(m, body + 0x13f4) - 0.01) as f32 as f64;
    let e = (f(m, body + 0x13f8) + 0.01) as f32 as f64;
    if pys > a && qys > a {
        return 0;
    }
    if !(pys >= c) && !(qys >= c) {
        return 0;
    }
    if !(pxs >= d) && !(qxs >= d) {
        return 0;
    }
    if pxs > e && qxs > e {
        return 0;
    }
    1
}

/// `0x58d1f0(out)` (thiscall, `ret 4`): the next 28-byte record of the list at `+0xf94` (count `+0x140c`, index `+0x1410`) goes to
/// `out`; returns the 1-based index, or 0 (and the index is reset) at the end.
pub fn record_iter_next(m: &mut Mem, this: u32, out: u32) -> u32 {
    let (i, n) = (m.i32(this + 0x1410), m.i32(this + 0x140c));
    if i >= n {
        m.set_u32(this + 0x1410, 0);
        return 0;
    }
    for k in 0..7 {
        let w = m.u32(this + 0xf94 + 28 * i as u32 + 4 * k);
        m.set_u32(out + 4 * k, w);
    }
    m.set_u32(this + 0x1410, (i + 1) as u32);
    (i + 1) as u32
}

/// `0x58d250(out, index)` (thiscall, `ret 8`): the record `index` of the same list; 1, or 0 when the index is out of range.
pub fn record_get(m: &mut Mem, this: u32, out: u32, index: i32) -> u32 {
    if index >= m.i32(this + 0x140c) || index < 0 {
        return 0;
    }
    for k in 0..7 {
        let w = m.u32(this + 0xf94 + 28 * index as u32 + 4 * k);
        m.set_u32(out + 4 * k, w);
    }
    1
}

/// `0x58b4f0` (thiscall): zeroes the 28-byte record (`+0..0x1c`; vectors at `+0` and `+0x10` come from `VEC_ZERO`).
pub fn record_zero(m: &mut Mem, this: u32) {
    for k in 0..7 {
        m.set_u32(this + 4 * k, 0);
    }
}

/// `0x5917f0(p*, body, rec*, t)` (cdecl): the point `p` against the planes of the body (28-byte records `(n, w, q)` from `+0xf94`, listed by
/// `0x58d1f0`). `p` is taken into body space (inverse pose `Mi`); a plane with `n . (p - q) > t` rejects (result 0), otherwise the plane
/// with the largest distance above `-[+0xcd0]` is the best one (index; the point is remembered). The best record (`0x58d250`; the error
/// log `0x67df80` when it is missing) gives `e = n * t + q`; the remembered point is moved onto the plane through `e` (`0x4e6362`),
/// carried to world space by the body matrix into `rec+0x34`, `n` rotated into `rec+0x40`; `rec+4..0x34 = Mi`, `rec+0x74 = body`,
/// `rec+0x64 = rec+0x6c = 0`. Returns 1.
pub fn hull_cast(m: &mut Mem, pt: u32, body: u32, rec: u32, t: f32, err: &mut dyn FnMut()) -> u32 {
    use crate::matrix::*;
    const L: u32 = 0x7ffe_f000 + 0x7000;
    let (rt, mi, mw, pl, best, e_pt, c_tmp) = (L, L + 0x20, L + 0x50, L + 0x80, L + 0x90, L + 0xa0, L + 0xb0);
    let f = |m: &Mem, a: u32| m.f32(a) as f64;
    record_zero(m, rt);
    let src = if m.u32(body + 0x5a0) != 0 { body + 0x730 } else { body + 0x254 };
    for i in 0..12 {
        let w = m.u32(src + 4 * i);
        m.set_u32(mi + 4 * i, w);
    }
    let (px, py, pz) = (f(m, pt), f(m, pt + 4), f(m, pt + 8));
    let g = |m: &Mem, i: u32| f(m, mi + 4 * i);
    let pv = [
        (((g(m, 0) * px + g(m, 6) * pz) + g(m, 3) * py) + g(m, 9)) as f32,
        (((g(m, 1) * px + g(m, 7) * pz) + g(m, 4) * py) + g(m, 10)) as f32,
        (((g(m, 2) * px + g(m, 8) * pz) + g(m, 5) * py) + g(m, 11)) as f32,
    ];
    for i in 0..3 {
        m.set_f32(pl + 4 * i, pv[i as usize]);
        m.set_u32(best + 4 * i, 0);
    }
    let mut ok = true;
    let mut best_d = (-f(m, body + 0xcd0)) as f32;
    let mut best_i = 0u32;
    let tt = t as f64;
    loop {
        let i = record_iter_next(m, body, rt);
        if i == 0 {
            break;
        }
        let (a, b, c) = (f(m, pl) - f(m, rt + 0x10), f(m, pl + 4) - f(m, rt + 0x14), f(m, pl + 8) - f(m, rt + 0x18));
        let d = (b * f(m, rt + 4) + c * f(m, rt + 8)) + a * f(m, rt);
        if d > tt {
            ok = false;
            continue;
        }
        if (best_d as f64) < d {
            best_i = i - 1;
            best_d = d as f32;
            for k in 0..3 {
                let w = m.u32(pl + 4 * k);
                m.set_u32(best + 4 * k, w);
            }
        }
    }
    if !ok {
        return 0;
    }
    if record_get(m, body, rt, best_i as i32) == 0 {
        err();
    }
    let n = [f(m, rt), f(m, rt + 4), f(m, rt + 8)];
    let q = [f(m, rt + 0x10), f(m, rt + 0x14), f(m, rt + 0x18)];
    let vy = (n[1] * tt) as f32 as f64;
    let vz = (n[2] * tt) as f32 as f64;
    let e3 = [((n[0] * tt) + q[0]) as f32, (vy + q[1]) as f32, (vz + q[2]) as f32];
    for i in 0..3 {
        m.set_f32(e_pt + 4 * i, e3[i as usize]);
    }
    point_to_plane(m, c_tmp, e_pt, rt, best);
    for i in 0..3 {
        let w = m.u32(c_tmp + 4 * i);
        m.set_u32(rec + 0x34 + 4 * i, w);
        let w = m.u32(rt + 4 * i);
        m.set_u32(rec + 0x40 + 4 * i, w);
    }
    let mat = if m.u32(body + 0x5a0) != 0 { body + 0x700 } else { body + 0x1f4 };
    for i in 0..12 {
        let w = m.u32(mat + 4 * i);
        m.set_u32(mw + 4 * i, w);
    }
    let h = |m: &Mem, i: u32| f(m, mw + 4 * i);
    let (cx, cy, cz) = (f(m, rec + 0x34), f(m, rec + 0x38), f(m, rec + 0x3c));
    let wv = [
        (((h(m, 0) * cx + h(m, 6) * cz) + h(m, 3) * cy) + h(m, 9)) as f32,
        (((h(m, 1) * cx + h(m, 7) * cz) + h(m, 4) * cy) + h(m, 10)) as f32,
        (((h(m, 2) * cx + h(m, 8) * cz) + h(m, 5) * cy) + h(m, 11)) as f32,
    ];
    for i in 0..3 {
        m.set_f32(rec + 0x34 + 4 * i, wv[i as usize]);
    }
    rotate(m, c_tmp, rec + 0x40, mw);
    for i in 0..3 {
        let w = m.u32(c_tmp + 4 * i);
        m.set_u32(rec + 0x40 + 4 * i, w);
    }
    m.set_u32(rec + 0x74, body);
    m.set_u32(rec + 0x64, 0);
    m.set_u32(rec + 0x6c, 0);
    for i in 0..12 {
        let w = m.u32(mi + 4 * i);
        m.set_u32(rec + 4 + 4 * i, w);
    }
    1
}

/// `0x591b50(p*, q*, body, rec*, t)` (cdecl): the segment `p-q` against the planes of the body. When `p` hits (`hull_cast`) the hit point
/// becomes `p` and the result is 1 only if `(q - p) . rec.normal < 0`; otherwise `q` is cast, and on a hit the point moves to where the
/// line `p-q` meets the plane (`line_plane`) and the result is 1.
pub fn segment_hull_cast(m: &mut Mem, p: u32, q: u32, body: u32, rec: u32, t: f32, err: &mut dyn FnMut()) -> u32 {
    let hit = hull_cast(m, p, body, rec, t, err);
    if hit != 0 {
        for i in 0..3 {
            let w = m.u32(p + 4 * i);
            m.set_u32(rec + 0x34 + 4 * i, w);
        }
        let f = |m: &Mem, a: u32| m.f32(a) as f64;
        let (dx, dy, dz) = (f(m, q) - f(m, p), f(m, q + 4) - f(m, p + 4), f(m, q + 8) - f(m, p + 8));
        let d = (dz * f(m, rec + 0x48) + dy * f(m, rec + 0x44)) + dx * f(m, rec + 0x40);
        return if !(d >= 0.0) { hit } else { 0 };
    }
    if hull_cast(m, q, body, rec, t, err) == 0 {
        return 0;
    }
    let tmp = 0x7ffe_f000 + 0x7100;
    crate::matrix::line_plane(m, tmp, rec + 0x34, rec + 0x40, p, q);
    for i in 0..3 {
        let w = m.u32(tmp + 4 * i);
        m.set_u32(rec + 0x34 + 4 * i, w);
    }
    1
}

/// What `0x58a020` calls outside itself (stand-ins): the context constructor `0x5ef5c0`, the object cast
/// `0x4e52fc(obj, p0, p1, ctx)` and the error log `0x4e260c` (its message is built by `sprintf` and only the log reads it).
pub trait PairOwner {
    fn ctx_init(&mut self, m: &mut Mem, ctx: u32);
    fn object_cast(&mut self, m: &mut Mem, obj: u32, p0: u32, p1: u32, ctx: u32) -> u32;
    fn error(&mut self);
}

/// `0x58a020(body, matrix*, set, other, sign)` (cdecl): the sample points of `body` against the other body. Each point goes to world
/// space (`Pw`, the body matrix) and through `matrix` (`P1`). For `other` of mode 2 a segment outside its box (`0x589eb0`) is skipped.
/// The segment is taken into the local space of `other` (`inverse(other[+0x2b4] * other matrix)`) and cast by `0x4e52fc` at
/// `[other+0x140]`; a hit (bit 0) moves hit point and normal back by the same transform (range checked, log on failure) and fills the
/// owner (`+0x74`: `other` for `sign > 0`, else `body`). A miss against a mode 2 `other` tries the plane list (`0x591b50`, 0.01).
/// Per point: flag `set+0xb04 = (hit & 1) * sign`, scales `set+0x800` / `set+0x900 = 1.0`; with a flag the scales become 0 and then
/// the hit point (`set+12n`), the sign-scaled normal (`set+12(n+0x40)`), the owner (`+0x600`), the point index (`+0xc04`), the
/// distance (`+0xa00`) and the ratios `|hit - pw| / |ctx[0x58] - pw|` (`+0x900`, at most 1) and `(|hit - pw| - 0.01) / |ctx[0x58] - pw|`
/// (`+0x800`, 0 when not positive) are written; for `sign > 0` `[+0x1444+4i]` gets bit 2 (also `+0x1494`) and the context is copied
/// to `body+0x1418+0xd0i`. Stops at 63 entries (the iterator is drained). Returns the number of flagged points.
pub fn body_pair_query(m: &mut Mem, a: u32, mat: u32, set: u32, other: u32, sign: i32, o: &mut dyn PairOwner) -> u32 {
    use crate::matrix::*;
    const L: u32 = 0x7ffe_f000 + 0x8000;
    let (p, pw, p1, oc, mo, zm, ri, pa, pb, ht, nt, ctx, tmp, u2) =
        (L, L + 0x10, L + 0x20, L + 0x60, L + 0x90, L + 0xc0, L + 0xf0, L + 0x120, L + 0x130, L + 0x140, L + 0x150, L + 0x200, L + 0x300, L + 0x310);
    let f = |m: &Mem, a: u32| m.f32(a) as f64;
    let copy = |m: &mut Mem, dst: u32, src: u32, n: u32| {
        for i in 0..n {
            let w = m.u32(src + 4 * i);
            m.set_u32(dst + 4 * i, w);
        }
    };
    o.ctx_init(m, ctx);
    let a_mat = if m.u32(a + 0x5a0) != 0 { a + 0x700 } else { a + 0x1f4 };
    let mut counter = 0u32;
    let mut idx = crate::body_ray::point_iter_next(m, a, p);
    if idx == 0 {
        return 0;
    }
    loop {
        let cnt = m.i32(set + 0xb00);
        if cnt >= 0x3f {
            while crate::body_ray::point_iter_next(m, a, p) != 0 {}
            return counter;
        }
        let (px, py, pz) = (f(m, p), f(m, p + 4), f(m, p + 8));
        let g = |m: &Mem, i: u32| f(m, a_mat + 4 * i);
        let w3 = [
            (((g(m, 6) * pz + g(m, 3) * py) + g(m, 0) * px) + g(m, 9)) as f32,
            (((g(m, 7) * pz + g(m, 4) * py) + g(m, 1) * px) + g(m, 10)) as f32,
            (((g(m, 8) * pz + g(m, 5) * py) + g(m, 2) * px) + g(m, 11)) as f32,
        ];
        let b = |m: &Mem, i: u32| f(m, mat + 4 * i);
        let q3 = [
            (((pz * b(m, 6) + py * b(m, 3)) + px * b(m, 0)) + b(m, 9)) as f32,
            (((px * b(m, 1) + pz * b(m, 7)) + py * b(m, 4)) + b(m, 10)) as f32,
            (((px * b(m, 2) + pz * b(m, 8)) + py * b(m, 5)) + b(m, 11)) as f32,
        ];
        for i in 0..3 {
            m.set_f32(pw + 4 * i, w3[i as usize]);
            m.set_f32(p1 + 4 * i, q3[i as usize]);
        }
        let mut hitraw = 0u32;
        let omode2 = m.u32(other + 0x188) == 2;
        let skip = omode2 && segment_box_test(m, other, pw, p1) == 0;
        if !skip {
            copy(m, oc, other + 0x2b4, 12);
            copy(m, mo, if m.u32(other + 0x5a0) != 0 { other + 0x700 } else { other + 0x1f4 }, 12);
            mat_mul(m, zm, oc, mo);
            crate::math::mat_inverse(m, ri, zm);
            let apply = |m: &mut Mem, dst: u32, src: u32| {
                let (x, y, z) = (f(m, src), f(m, src + 4), f(m, src + 8));
                let r = |i: u32| f(m, ri + 4 * i);
                let v = [
                    (((r(6) * z + r(3) * y) + r(0) * x) + r(9)) as f32,
                    (((r(7) * z + r(4) * y) + r(1) * x) + r(10)) as f32,
                    (((r(8) * z + r(5) * y) + r(2) * x) + r(11)) as f32,
                ];
                for i in 0..3 {
                    m.set_f32(dst + 4 * i, v[i as usize]);
                }
            };
            apply(m, pa, pw);
            apply(m, pb, p1);
            let obj = m.u32(other + 0x140);
            hitraw = o.object_cast(m, obj, pa, pb, ctx);
            if hitraw & 1 != 0 {
                let (hx, hy, hz) = (f(m, ctx + 0x34), f(m, ctx + 0x38), f(m, ctx + 0x3c));
                let (nx, ny, nz) = (f(m, ctx + 0x40), f(m, ctx + 0x44), f(m, ctx + 0x48));
                let r = |i: u32| f(m, zm + 4 * i);
                let hv = [
                    (((r(6) * hz + r(3) * hy) + r(0) * hx) + r(9)) as f32,
                    (((r(7) * hz + r(4) * hy) + r(1) * hx) + r(10)) as f32,
                    (((r(8) * hz + r(5) * hy) + r(2) * hx) + r(11)) as f32,
                ];
                let nv = [
                    ((r(3) * ny + r(6) * nz) + r(0) * nx) as f32,
                    ((r(1) * nx + r(4) * ny) + r(7) * nz) as f32,
                    ((r(2) * nx + r(5) * ny) + r(8) * nz) as f32,
                ];
                for i in 0..3 {
                    m.set_f32(ht + 4 * i, hv[i as usize]);
                    m.set_f32(nt + 4 * i, nv[i as usize]);
                }
                if crate::carv::fn_004e28b0(m, ht, 10000.0) != 0 || crate::carv::fn_004e28b0(m, ctx + 0x40, 2.0) != 0 {
                    o.error();
                }
                copy(m, ctx + 0x34, ht, 3);
                copy(m, ctx + 0x40, nt, 3);
                m.set_u32(ctx + 0x74, if sign > 0 { other } else { a });
            } else if omode2 && segment_hull_cast(m, pw, p1, other, ctx, f32::from_bits(0x3c23_d70a), &mut || o.error()) != 0 {
                hitraw = 1;
            }
        }
        let c = cnt as u32;
        let flag = (hitraw & 1) as i32 * sign;
        m.set_u32(set + 0xb04 + 4 * c, flag as u32);
        m.set_u32(set + 0x800 + 4 * c, 0x3f80_0000);
        m.set_u32(set + 0x900 + 4 * c, 0x3f80_0000);
        if flag != 0 {
            counter += 1;
            m.set_u32(set + 0x800 + 4 * c, 0);
            m.set_u32(set + 0x900 + 4 * c, 0);
            let sg = sign as f64;
            copy(m, set + 12 * c, ctx + 0x34, 3);
            for i in 0..3 {
                let v = (sg * f(m, ctx + 0x40 + 4 * i)) as f32;
                m.set_f32(set + 12 * (c + 0x40) + 4 * i, v);
            }
            if sign > 0 {
                let w = m.u32(ctx + 0x74);
                m.set_u32(set + 0x600 + 4 * c, w);
                let fl = m.u32(a + 0x1444 + 4 * idx) | 2;
                m.set_u32(a + 0x1444 + 4 * idx, fl);
                m.set_u32(a + 0x1494 + 4 * idx, fl);
                copy(m, a + 0xd0 * idx + 0x1418, ctx, 0x34);
            } else {
                m.set_u32(set + 0x600 + 4 * c, a);
            }
            m.set_u32(set + 0xc04 + 4 * c, idx - 1);
            let (qa, qb, qc) = (f(m, ctx + 0x58) - f(m, pw), f(m, ctx + 0x5c) - f(m, pw + 4), f(m, ctx + 0x60) - f(m, pw + 8));
            for i in 0..3 {
                let v = (f(m, ctx + 0x34 + 4 * i) - f(m, pw + 4 * i)) as f32;
                m.set_f32(u2 + 4 * i, v);
            }
            let (hx, hy) = (f(m, ctx + 0x34) - f(m, p1), f(m, ctx + 0x38) - f(m, p1 + 4));
            let hz = (f(m, ctx + 0x3c) - f(m, p1 + 8)) as f32 as f64;
            let dist = ((hy * f(m, ctx + 0x44) + hz * f(m, ctx + 0x48)) + hx * f(m, ctx + 0x40)) as f32;
            m.set_f32(set + 0xa00 + 4 * c, dist);
            let lq = ((qa * qa + qb * qb) + qc * qc) as f32;
            let _ = tmp;
            let (ux, uy, uz) = (f(m, u2), f(m, u2 + 4), f(m, u2 + 8));
            let s = (ux * ux + uy * uy) + uz * uz;
            if lq as f64 >= 1e-7 {
                let mut q = s / lq as f64;
                if !(q <= 1.0) {
                    q = 1.0;
                }
                let rr = q.sqrt() as f32;
                m.set_f32(set + 0x800 + 4 * c, rr);
                m.set_f32(set + 0x900 + 4 * c, rr);
                let wv = s.sqrt() - 0.01;
                if !(wv <= 0.0) {
                    m.set_f32(set + 0x800 + 4 * c, (wv / (lq as f64).sqrt()) as f32);
                } else {
                    m.set_u32(set + 0x800 + 4 * c, 0);
                }
            }
            m.set_u32(set + 0xb00, c + 1);
        }
        idx = crate::body_ray::point_iter_next(m, a, p);
        if idx == 0 {
            return counter;
        }
    }
}

/// What `0x58b120` calls outside itself (stand-ins): the wrapper `0x589e70` (`0x589640` with `flags & !0x200`, receives the raw
/// flags), the neighbours `0x58ae30` and the road query `0x585150`, all `(body, matrix, set)`; each appends contacts to the set.
pub trait QueryOwner {
    fn point_contacts(&mut self, m: &mut Mem, body: u32, mat: u32, set: u32, flags: u32) -> u32;
    fn neighbours(&mut self, m: &mut Mem, body: u32, mat: u32, set: u32) -> u32;
    fn collect(&mut self, m: &mut Mem, body: u32, mat: u32, set: u32) -> u32;
}

/// `0x58b120(body, matrix, scale*, dist*, rec, flags)` (cdecl): the body against everything. A local contact set (constructor
/// `0x587c80`) is filled by `0x589e70`, `0x58ae30` and `0x585150`; all three zero gives 0. Otherwise the contact with the smallest scale
/// (flag set, strictly below 100) is chosen: `*scale *= its scale`, `*dist` = its distance (`+0xa00`). Without neighbour result its hit
/// point is the weighted blend (`(1.01 - scale)`, only contacts whose normal is within 0.9999 of the chosen one) added to the chosen
/// point and the normal is the unit chosen normal; with it both are copied. `rec+0x64` / `rec+0x74` get the `+0x700` / owner slots,
/// a flag of -1 copies the contact's matrix to `rec+4`. The result is a mask: `0x4000000` / `0x2000000` from `[body+0xf38+4*index]`
/// bits 0 / 1 (mode `[+0xe04] == 0`), `0x8000000` for flag -1, 3 / 8 / 4 for the results of the three calls.
pub fn body_query_other(m: &mut Mem, body: u32, mat: u32, scale: u32, dist: u32, rec: u32, flags: u32, o: &mut dyn QueryOwner) -> u32 {
    use crate::matrix::*;
    const L: u32 = 0x7ffe_f000 + 0xa000;
    let (acc, out, bp, bn, set) = (L, L + 0x10, L + 0x20, L + 0x30, L + 0x100);
    let f = |m: &Mem, a: u32| m.f32(a) as f64;
    crate::contact_set::contact_set_ctor(m, set);
    let r1 = o.point_contacts(m, body, mat, set, flags);
    let r2 = o.neighbours(m, body, mat, set);
    let r3 = o.collect(m, body, mat, set);
    if r1 == 0 && r2 == 0 && r3 == 0 {
        return 0;
    }
    let cnt = m.i32(set + 0xb00);
    let (mut b, mut best, mut entry) = (0u32, 100.0f64, 0u32);
    for i in 0..cnt.max(0) as u32 {
        if m.u32(set + 0xb04 + 4 * i) != 0 {
            let s = f(m, set + 0x800 + 4 * i);
            if best > s {
                b = i;
                best = s;
                entry = m.u32(set + 0xa00 + 4 * i);
            }
        }
    }
    for i in 0..3 {
        let (p, n) = (m.u32(set + 12 * b + 4 * i), m.u32(set + 0x300 + 12 * b + 4 * i));
        m.set_u32(bp + 4 * i, p);
        m.set_u32(bn + 4 * i, n);
    }
    let sb = f(m, set + 0x800 + 4 * b);
    let v = (sb * f(m, scale)) as f32;
    m.set_f32(scale, v);
    for i in 0..3 {
        let z = m.u32(0x702650 + 4 * i);
        m.set_u32(acc + 4 * i, z);
    }
    if r2 == 0 {
        let mut sum = 0.0f32;
        for i in (0..cnt.max(0) as u32).rev() {
            if m.u32(set + 0xb04 + 4 * i) == 0 {
                continue;
            }
            let g = |m: &Mem, k: u32| f(m, set + 0x300 + 12 * i + 4 * k);
            let dot = (g(m, 2) * f(m, bn + 8) + g(m, 0) * f(m, bn)) + g(m, 1) * f(m, bn + 4);
            if !(dot > 0.9999) {
                continue;
            }
            let p = |m: &Mem, k: u32| f(m, set + 12 * i + 4 * k);
            let dx = p(m, 0) - f(m, bp);
            let dy = (p(m, 1) - f(m, bp + 4)) as f32 as f64;
            let dz = (p(m, 2) - f(m, bp + 8)) as f32 as f64;
            let s = f(m, set + 0x800 + 4 * i);
            let w = (1.01f64 - s) as f32 as f64;
            let w3 = [(dx * w) as f32, (dy * w) as f32, (dz * w) as f32];
            for k in 0..3 {
                m.set_f32(L + 0x50 + 4 * k, w3[k as usize]);
            }
            vec_add_in_place(m, acc, out, L + 0x50);
            sum = ((1.01f64 - s) + sum as f64) as f32;
        }
        vec_div_in_place(m, acc, out, sum);
        vec_add_in_place(m, acc, out, bp);
        for k in 0..3 {
            let w = m.u32(acc + 4 * k);
            m.set_u32(rec + 0x34 + 4 * k, w);
        }
        vec_unit(m, out, bn);
        for k in 0..3 {
            let w = m.u32(out + 4 * k);
            m.set_u32(rec + 0x40 + 4 * k, w);
        }
    } else {
        for k in 0..3 {
            let (p, n) = (m.u32(bp + 4 * k), m.u32(bn + 4 * k));
            m.set_u32(rec + 0x34 + 4 * k, p);
            m.set_u32(rec + 0x40 + 4 * k, n);
        }
    }
    let (w64, w74) = (m.u32(set + 0x700 + 4 * b), m.u32(set + 0x600 + 4 * b));
    m.set_u32(rec + 0x64, w64);
    m.set_u32(rec + 0x74, w74);
    let fl = m.u32(set + 0xb04 + 4 * b);
    if fl == 0xffff_ffff {
        for k in 0..12 {
            let w = m.u32(set + 0xd04 + 0x30 * b + 4 * k);
            m.set_u32(rec + 4 + 4 * k, w);
        }
    }
    m.set_u32(dist, entry);
    let mut res = 0u32;
    if m.u32(body + 0xe04) == 0 {
        let idx = m.u32(set + 0xc04 + 4 * b);
        let c = m.u32(body.wrapping_add(idx.wrapping_mul(4)).wrapping_add(0xf38));
        if c & 1 != 0 {
            res = 0x400_0000;
        }
        if c & 2 != 0 {
            res |= 0x200_0000;
        }
    }
    if fl == 0xffff_ffff {
        res |= 0x800_0000;
    }
    if r1 != 0 {
        res |= 3;
    }
    if r2 != 0 {
        res |= 8;
    }
    if r3 != 0 {
        res |= 4;
    }
    res
}

/// What `0x5f9b00` calls outside itself (stand-in): the virtual `+0x34(flags, rec)` of the child object `[this+0x40]`.
pub trait CastVirt {
    fn virt34(&mut self, m: &mut Mem, obj: u32, flags: u32, rec: u32) -> u32;
}

/// `0x5f9b00(flags, rec)` (thiscall, the virtual `+0x3c` of the collision wrapper): 0 when `[+0x28] & 1`, `[+0x50] != 0`, `flags & 4 == 0`
/// or without child `[+0x40]`. When the node `[+0x44]` exists, its world matrix (`0x5f25c0`) is taken into the local space of the record:
/// result 0: `rec+4 = inverse(M)`, result 1: `rec+4 = rec+4 * inverse(M)` (other: unchanged). Then the child's virtual `+0x34(flags, rec)`:
/// 0 restores `rec+4` and returns 0; with `[this+0x48] == [rec+0x78]` (non-null) it restores `rec+4`, `rec+0x4c` and `rec+0x58` and returns 0;
/// otherwise `[rec+0x74] = [this+0x48]` and the virtual's result is returned.
pub fn cast_wrapper_virt3c(m: &mut Mem, this: u32, flags: u32, rec: u32, o: &mut dyn CastVirt) -> u32 {
    const L: u32 = 0x7ffe_f000 + 0xb000;
    let (mm, inv, tmp, saved) = (L, L + 0x40, L + 0x80, L + 0xc0);
    if m.u32(this + 0x28) & 1 != 0 || m.u32(this + 0x50) != 0 || flags & 4 == 0 {
        return 0;
    }
    let child = m.u32(this + 0x40);
    for i in 0..12 {
        let w = m.u32(rec + 4 + 4 * i);
        m.set_u32(saved + 4 * i, w);
    }
    if child == 0 {
        return 0;
    }
    let (v1, v2) = ([m.u32(rec + 0x4c), m.u32(rec + 0x50), m.u32(rec + 0x54)], [m.u32(rec + 0x58), m.u32(rec + 0x5c), m.u32(rec + 0x60)]);
    for i in 0..12 {
        m.set_u32(mm + 4 * i, m.u32(0x7025f0 + 4 * i));
    }
    let node = m.u32(this + 0x44);
    if node != 0 {
        match crate::mesh_query::node_world_matrix(m, node, mm) {
            0 => {
                crate::math::mat_inverse(m, inv, mm);
                for i in 0..12 {
                    let w = m.u32(inv + 4 * i);
                    m.set_u32(rec + 4 + 4 * i, w);
                }
            }
            1 => {
                crate::math::mat_inverse(m, inv, mm);
                crate::matrix::mat_mul(m, tmp, rec + 4, inv);
                for i in 0..12 {
                    let w = m.u32(tmp + 4 * i);
                    m.set_u32(rec + 4 + 4 * i, w);
                }
            }
            _ => {}
        }
    }
    let res = o.virt34(m, child, flags, rec);
    let restore = |m: &mut Mem| {
        for i in 0..12 {
            let w = m.u32(saved + 4 * i);
            m.set_u32(rec + 4 + 4 * i, w);
        }
    };
    if res == 0 {
        restore(m);
        return 0;
    }
    let tag = m.u32(this + 0x48);
    if tag != 0 && tag == m.u32(rec + 0x78) {
        restore(m);
        for i in 0..3 {
            m.set_u32(rec + 0x4c + 4 * i, v1[i as usize]);
            m.set_u32(rec + 0x58 + 4 * i, v2[i as usize]);
        }
        return 0;
    }
    m.set_u32(rec + 0x74, tag);
    res
}

/// What `0x5f9d20` calls outside itself (stand-ins): the transform stack push `0x5ae580(stack, matrix, type)` and pop `0x5ae8a0(stack)` and
/// the virtual `+0x38(a0, mat, mat2, stack)` of the child object `[this+0x40]`.
pub trait CastVirt2 {
    fn push(&mut self, m: &mut Mem, stack: u32, mat: u32, kind: u32);
    fn pop(&mut self, m: &mut Mem, stack: u32);
    fn virt38(&mut self, m: &mut Mem, obj: u32, a0: u32, mat: u32, mat2: u32, stack: u32) -> u32;
}

/// `0x5f9d20(a0, mat, mat2, stack)` (thiscall, `ret 0x10`): 0 when `[this+0x28] & 1` (`0x5ef820`) or without child `[+0x40]`. The matrix `mat`
/// (12 words) is saved. With a node `[+0x44]` its world matrix `M` (`0x5f25c0`, kind = its result) is pushed on the stack object (`0x5ae580`) and
/// `mat = M * mat2` (`0x5aba40`). The child's virtual `+0x38(a0, mat, mat2, stack)` is called, with a node the stack is popped (`0x5ae8a0`),
/// `mat` is restored and the virtual's result returned.
pub fn cast_wrapper_virt3c_b(m: &mut Mem, this: u32, a0: u32, mat: u32, mat2: u32, stack: u32, o: &mut dyn CastVirt2) -> u32 {
    const L: u32 = 0x7ffe_f000 + 0xb400;
    let (mm, tmp, saved) = (L, L + 0x40, L + 0x80);
    if m.u32(this + 0x28) & 1 != 0 {
        return 0;
    }
    let child = m.u32(this + 0x40);
    if child == 0 {
        return 0;
    }
    for i in 0..12 {
        let w = m.u32(mat + 4 * i);
        m.set_u32(saved + 4 * i, w);
        m.set_u32(mm + 4 * i, m.u32(0x7025f0 + 4 * i));
    }
    let node = m.u32(this + 0x44);
    if node != 0 {
        let kind = crate::mesh_query::node_world_matrix(m, node, mm);
        o.push(m, stack, mm, kind);
        crate::matrix::mat_mul(m, tmp, mm, mat2);
        for i in 0..12 {
            let w = m.u32(tmp + 4 * i);
            m.set_u32(mat + 4 * i, w);
        }
    }
    let res = o.virt38(m, child, a0, mat, mat2, stack);
    if node != 0 {
        o.pop(m, stack);
    }
    for i in 0..12 {
        let w = m.u32(saved + 4 * i);
        m.set_u32(mat + 4 * i, w);
    }
    res
}

/// What `0x592a20` calls outside itself (stand-in): the wrapper `0x589e90(body, set, flags)` (`0x589640` with the identity matrix and
/// `flags | 0x200`); `body_point_contacts` is its port.
pub trait GroundOwner {
    fn point_contacts(&mut self, m: &mut Mem, body: u32, set: u32, flags: u32) -> u32;
}

/// `0x592a20(body, rec, out*)` (cdecl): the sample-point casts of the body into a local contact set (`0x589e90(body, set, 0x200)`); no hit
/// gives 0. Otherwise the flagged contact with the largest distance (`+0xa00`, strictly above 0, first of equals) fills the record: point
/// `rec+0x34`, normal `rec+0x40`, owner `rec+0x74`, `rec+0x64` from `+0x700`, a flag of -1 copies the contact's matrix to `rec+4`;
/// `*out` = that distance; returns 1.
pub fn body_ground_hit(m: &mut Mem, body: u32, rec: u32, out: u32, o: &mut dyn GroundOwner) -> u32 {
    const L: u32 = 0x7ffe_f000 + 0xc000;
    let set = L;
    crate::contact_set::contact_set_ctor(m, set);
    if o.point_contacts(m, body, set, 0x200) == 0 {
        return 0;
    }
    let cnt = m.i32(set + 0xb00);
    let (mut best, mut idx) = (0.0f64, 0u32);
    for i in 0..cnt.max(0) as u32 {
        if m.u32(set + 0xb04 + 4 * i) != 0 {
            let d = m.f32(set + 0xa00 + 4 * i) as f64;
            if best < d {
                best = d;
                idx = i;
            }
        }
    }
    for k in 0..3 {
        let (p, n) = (m.u32(set + 12 * idx + 4 * k), m.u32(set + 0x300 + 12 * idx + 4 * k));
        m.set_u32(rec + 0x34 + 4 * k, p);
        m.set_u32(rec + 0x40 + 4 * k, n);
    }
    let (w74, w64) = (m.u32(set + 0x600 + 4 * idx), m.u32(set + 0x700 + 4 * idx));
    m.set_u32(rec + 0x74, w74);
    m.set_u32(rec + 0x64, w64);
    if m.u32(set + 0xb04 + 4 * idx) == 0xffff_ffff {
        for k in 0..12 {
            let w = m.u32(set + 0xd04 + 0x30 * idx + 4 * k);
            m.set_u32(rec + 4 + 4 * k, w);
        }
    }
    m.set_f32(out, best as f32);
    1
}

/// What `0x57e2e0` calls outside itself (stand-ins): the range report `0x4e2821(vec, text, limit_bits)` and the virtual `+0x90(vec, text)`.
pub trait ContactPlaceOwner: ItemVirtuals {
    fn range(&mut self, m: &mut Mem, vec: u32, text: u32, limit: u32);
}

/// `0x57e2e0(body, rec, out*)` (cdecl): a contact at the point `rec+0x34` with the normal `rec+0x40` and the depth `rec[0]` (offset `+0.001`)
/// is turned into a pose for the body. The point `P = depth' * n + point` is taken, `d = body position (+0xc64 matrix) - P`, `c = n x d`,
/// `e = c x n`, `Q = P + e` and the body matrix with the translation `Q` is written to `*out`. With `b` = the vector at `+0xce4` and `k` =
/// `[+0x590]`: `F = ((b x n) x n) + k * (|b . n| * n)`; `F` is range-checked (`0x4e2821`, text `0x67d1b8`, 150.0) and handed to the
/// virtual `+0x90(F, text)`. The x87 intermediates stay unrounded as in the original (`P.z`, `d`, `c.z` are kept in extended registers).
pub fn contact_place(m: &mut Mem, this: u32, rec: u32, out: u32, o: &mut dyn ContactPlaceOwner) {
    use crate::matrix::*;
    const L: u32 = 0x7ffe_f000 + 0xc400;
    let (mm, h, u, w, hk, fv) = (L, L + 0x40, L + 0x50, L + 0x60, L + 0x70, L + 0x80);
    let f = |m: &Mem, a: u32| m.f32(a) as f64;
    let r32 = |v: f64| v as f32 as f64;
    for i in 0..12 {
        let x = m.u32(this + 0xc64 + 4 * i);
        m.set_u32(mm + 4 * i, x);
    }
    let n = [f(m, rec + 0x40), f(m, rec + 0x44), f(m, rec + 0x48)];
    let s = f(m, rec) + 0.001;
    let (tx, ty, tz) = (r32(s * n[0]), r32(s * n[1]), s * n[2]);
    let (px, py) = (r32(tx + f(m, rec + 0x34)), r32(ty + f(m, rec + 0x38)));
    let pz = tz + f(m, rec + 0x3c);
    let (dx, dy, dz) = (f(m, mm + 36) - px, f(m, mm + 40) - py, f(m, mm + 44) - pz);
    let cx = r32(dz * n[1] - dy * n[2]);
    let cy = r32(dx * n[2] - dz * n[0]);
    let cz = dy * n[0] - dx * n[1];
    let ex = r32(cy * n[2] - cz * n[1]);
    let ey = r32(cz * n[0] - cx * n[2]);
    let ez = cx * n[1] - cy * n[0];
    let (qx, qy, qz) = ((ex + px) as f32, (py + ey) as f32, (pz + ez) as f32);
    m.set_f32(mm + 36, qx);
    m.set_f32(mm + 40, qy);
    m.set_f32(mm + 44, qz);
    for i in 0..12 {
        let x = m.u32(mm + 4 * i);
        m.set_u32(out + 4 * i, x);
    }
    let b = [f(m, this + 0xce4), f(m, this + 0xce8), f(m, this + 0xcec)];
    let a = (((b[2] * n[2]) + b[1] * n[1]) + b[0] * n[0]).abs();
    crate::math::vec_set(m, h, (a * n[0]) as f32, (a * n[1]) as f32, (a * n[2]) as f32);
    crate::math::vec_set(m, u, (n[1] * b[2] - b[1] * n[2]) as f32, (b[0] * n[2] - n[0] * b[2]) as f32, (b[1] * n[0] - n[1] * b[0]) as f32);
    vec_cross(m, w, u, rec + 0x40);
    let k = m.f32(this + 0x590);
    vec_scale(m, hk, h, k);
    vec_add(m, fv, w, hk);
    o.range(m, fv, 0x67d1b8, 0x4316_0000);
    o.virt(m, this, 0x90, &[fv, 0x67d1b8]);
}

/// What `0x4e7b4d` calls outside itself (stand-ins): the formatter `0x4e7a19(buf, 300, text, A2, A24, A28)` that builds a diagnostic message
/// (it only writes its own buffer) and the log `0x4e260c(text)` (`text == 0`: a message built by `sprintf` in the frame).
pub trait ContactSolveLog {
    fn format(&mut self, m: &mut Mem, a2: u32, a24: u32, a28: u32);
    fn log(&mut self, text: u32);
}

/// `0x4e794f(v)`: limits the length of the vector: above 35 it is scaled to 20 (`(35 - 5) * 0.5 + 5`) (`0x4bdd60`), between 5 and 35 to
/// `(L - 5) * 0.5 + 5` (scaled by that and divided by `L`, `0x4bdd60` / `0x4e10d0`); up to 5 unchanged.
pub fn vec_soft_limit(m: &mut Mem, v: u32) {
    const L: u32 = 0x7ffe_f000 + 0xd000 + 0x2000;
    let (hi, lo, half) = (35.0f32, 5.0f32, 0.5f32);
    let len = crate::math::vec_len(m, v) as f32;
    let mid = ((hi as f64 - lo as f64) * half as f64 + lo as f64) as f32;
    let copy = |m: &mut Mem, dst: u32, src: u32| {
        for i in 0..3 {
            let w = m.u32(src + 4 * i);
            m.set_u32(dst + 4 * i, w);
        }
    };
    if len > hi {
        crate::matrix::vec_scale(m, L, v, (mid as f64 / len as f64) as f32);
        copy(m, v, L);
    } else if len > lo {
        let s = ((len as f64 - lo as f64) * half as f64 + lo as f64) as f32;
        crate::matrix::vec_scale(m, L, v, s);
        crate::matrix::vec_div_scalar(m, L + 0x10, L, len);
        copy(m, v, L + 0x10);
    }
}

/// `0x4e7b4d(A0, A1, A2, A3, f18, f1c, f20, A24, A28, A2c, out30, out34)` (cdecl, frame `0x1270`): a contact solver between two bodies. The
/// frame is kept in memory exactly like the original's (every temporary is an `f32` slot, the x87 intermediates between stores are `f64`).
pub fn contact_solve(m: &mut Mem, a: [u32; 4], f18: f32, f1c: f32, f20: f32, a24: u32, a28: u32, a2c: u32, o30: u32, o34: u32, lg: &mut dyn ContactSolveLog) {
    use crate::matrix::*;
    const B: u32 = 0x7ffe_f000 + 0xd000 + 0x1400;
    let p = |o: i32| (B as i32 + o) as u32;
    let f = |m: &Mem, o: i32| m.f32((B as i32 + o) as u32) as f64;
    let st = |m: &mut Mem, o: i32, v: f64| m.set_f32((B as i32 + o) as u32, v as f32);
    let copy3 = |m: &mut Mem, dst: u32, src: u32| {
        for i in 0..3 {
            let w = m.u32(src + 4 * i);
            m.set_u32(dst + 4 * i, w);
        }
    };
    let (a0, a1, a2, a3) = (a[0], a[1], a[2], a[3]);
    let (f18, f1c, f20) = (f18 as f64, f1c as f64, f20 as f64);
    rotate(m, p(-0x44), a28, a2);
    vec_sub(m, p(-0x120), a0, a2 + 0x24);
    let dot = vec_dot(m, p(-0x120), p(-0x120));
    st(m, -0x130, dot);
    st(m, -0x12c, dot.sqrt());
    let l = f(m, -0x12c) as f32;
    vec_div_scalar(m, p(-0xd4), p(-0x120), l);
    rotate(m, p(-0x188), p(-0xd4), a3);
    rotate(m, p(-0x15c), a1, a3);
    vec_cross(m, p(-0x114), a1, p(-0xd4));
    rotate(m, p(-0x1c), a24, a3);
    copy3(m, p(-0xc8), 0x702650);
    vec_cross(m, p(-0x1160), p(-0x15c), p(-0x1c));
    vec_cross(m, p(-0x104), p(-0x1160), p(-0x15c));
    vec_cross(m, p(-0x116c), a1, p(-0xd4));
    vec_unit(m, p(-0x9c), p(-0x116c));
    vec_cross(m, p(-0x1178), p(-0x15c), p(-0x188));
    vec_unit(m, p(-0x150), p(-0x1178));
    rotate(m, p(-0x17c), p(-0x9c), a3);
    // 0x4e30f4: component-wise product
    for i in 0..3 {
        let v = m.f32(a28 + 4 * i) as f64 * m.f32(a2c + 4 * i) as f64;
        m.set_f32(p(-0x10) + 4 * i, v as f32);
    }
    let d = vec_dot(m, p(-0x10), p(-0x17c));
    st(m, -0x1260, d);
    vec_scale(m, p(-0x7c), p(-0x17c), d as f32);
    vec_sub(m, p(-0x144), p(-0x10), p(-0x7c));
    copy3(m, p(-0x5c), p(-0x144));
    copy3(m, p(-0xbc), 0x702650);
    copy3(m, o30, a24);
    copy3(m, o34, a28);
    vec_cross(m, p(-0x1184), a28, p(-0x188));
    vec_scale(m, p(-0x194), p(-0x1184), l);
    rotate(m, p(-0x1190), p(-0x194), a2);
    copy3(m, p(-0x194), p(-0x1190));
    vec_add(m, p(-0x119c), a24, p(-0x194));
    copy3(m, p(-0x194), p(-0x119c));
    let d2 = vec_dot(m, p(-0x194), a1);
    st(m, -0x198, d2);
    if f(m, -0x198) > 0.0 {
        return;
    }
    let cl = vec_len(m, p(-0x114));
    st(m, -0xec, cl);
    let thr = f(m, -0x12c) * 0.02;
    if thr > f(m, -0xec) {
        // short cross product: a straight contact
        let d3 = vec_dot(m, a24, a1);
        st(m, -0x1d4, d3);
        st(m, -0x1a8, -f1c * f(m, -0x1d4));
        vec_cross(m, p(-0x11a8), a1, a24);
        vec_cross(m, p(-0x1a4), p(-0x11a8), a1);
        vec_scale(m, p(-0x1d0), a1, f(m, -0x1a8) as f32);
        vec_add(m, p(-0x11b4), p(-0x1d0), p(-0x1a4));
        copy3(m, o30, p(-0x11b4));
        let d4 = vec_dot(m, a28, p(-0x188));
        st(m, -0x1264, d4);
        vec_scale(m, p(-0x1b8), p(-0x188), d4 as f32);
        vec_sub(m, p(-0x1c4), a28, p(-0x1b8));
        st(m, -0x1ac, (f(m, -0x1a8) - f(m, -0x1d4)) * f18);
        if f(m, -0x1ac) > 0.0 {
            let len = vec_len(m, p(-0x5c));
            st(m, -0x1e4, len);
            st(m, -0x1e8, f(m, -0x1ac) * f20);
            if f(m, -0x1e8) > f(m, -0x1e4) {
                st(m, -0x1e4, 0.0);
            } else {
                let v = (f(m, -0x1e4) - f(m, -0x1e8)) / f(m, -0x1e4);
                st(m, -0x1e4, v);
            }
            vec_scale_in_place(m, p(-0x5c), p(-0x11c0), f(m, -0x1e4) as f32);
            copy3(m, p(-0xbc), p(-0x7c));
            vec_add(m, p(-0x1e0), p(-0xbc), p(-0x5c));
            let v = [m.f32(p(-0x1e0)), m.f32(p(-0x1e0) + 4), m.f32(p(-0x1e0) + 8)];
            let r = crate::math::vec_div(m, p(-0x11cc), v, a2c);
            copy3(m, o34, r);
        } else {
            st(m, -0x1ac, f(m, -0x1ac) - 0.1);
        }
        return;
    }
    // the general case
    let c1 = vec_dot(m, p(-0x1c), p(-0x15c));
    st(m, -0x170, c1);
    let c2 = vec_dot(m, p(-0x7c), p(-0x17c));
    st(m, -0x70, c2);
    vec_cross(m, p(-0x11d8), p(-0x188), p(-0x15c));
    let c3 = vec_len(m, p(-0x11d8));
    st(m, -0x134, c3);
    st(m, -0x38, f18 * f(m, -0x12c) * f(m, -0x134));
    st(m, -0x6c, f18 * 0.5);
    st(m, -0x50, f(m, -0x170) * f(m, -0x38) + f(m, -0x70));
    let r = crate::math::vec_div(m, p(-0x11e4), [m.f32(p(-0x17c)), m.f32(p(-0x17c) + 4), m.f32(p(-0x17c) + 8)], a2c);
    let d5 = vec_dot(m, p(-0x17c), r);
    st(m, -0x88, d5 * 0.5);
    if !(f(m, -0x88) > 0.0) {
        lg.format(m, a2, a24, a28);
        copy3(m, o30, a24);
        copy3(m, o34, a28);
        return;
    }
    st(m, -0x138, f(m, -0x170) * f(m, -0x170) * f(m, -0x6c));
    st(m, -0x4c, f(m, -0x70) * f(m, -0x70) * f(m, -0x88));
    st(m, -0xe8, f(m, -0x138) + f(m, -0x4c));
    m.set_f32(p(-0x128), f1c as f32);
    st(m, -0x68, f(m, -0x38) * f(m, -0x50) * f(m, -0x88));
    st(m, -0x84, f(m, -0x88) * f(m, -0x38) * f(m, -0x38) + f(m, -0x6c));
    st(m, -0xa8, f(m, -0x6c) * f(m, -0x88) * f(m, -0x50) * f(m, -0x50));
    let den = f(m, -0x84) * f(m, -0xe8);
    if den != 0.0 && !den.is_nan() {
        st(m, -0x128, f(m, -0xa8) / den);
    }
    let c128 = f(m, -0x128);
    if !(c128 > 1.01) && !(c128 < 0.0 || c128.is_nan()) {
    } else {
        lg.log(0);
        m.set_u32(p(-0x128), 0x3f80_0000);
        lg.log(0x673f88);
    }
    // [ebp-0x124] is a constant zero: the square root below is of 0
    m.set_u32(p(-0x124), 0);
    st(m, -0x90, (f(m, -0x124)).sqrt());
    st(m, -0x48, (f(m, -0x68) + f(m, -0x90)) / f(m, -0x84));
    st(m, -0x64, (f(m, -0x68) - f(m, -0x90)) / f(m, -0x84));
    st(m, -0xe4, f(m, -0x50) - f(m, -0x48) * f(m, -0x38));
    st(m, -0xf8, f(m, -0x50) - f(m, -0x64) * f(m, -0x38));
    let c170d = f(m, -0x170);
    let r = crate::math::vec_div(m, p(-0x11f0), [m.f32(p(-0x17c)), m.f32(p(-0x17c) + 4), m.f32(p(-0x17c) + 8)], a2c);
    vec_scale(m, p(-0x11fc), r, f(m, -0x70) as f32);
    vec_cross(m, p(-0x1208), p(-0x11fc), p(-0x188));
    vec_scale(m, p(-0x1214), p(-0x1208), f(m, -0x12c) as f32);
    let d6 = vec_dot(m, p(-0x1214), p(-0x15c));
    st(m, -0x8c, d6 + c170d);
    st(m, -0xa4, (f(m, -0xe4) * f(m, -0x88)) * 2.0 * f(m, -0x12c) + f(m, -0x48));
    st(m, -0xb0, f(m, -0x64) - (f(m, -0xf8) * f(m, -0x88)) * 2.0 * f(m, -0x12c));
    st(m, -0xf4, f(m, -0x48) * f(m, -0x48) * f(m, -0x6c) + f(m, -0xe4) * f(m, -0xe4) * f(m, -0x88));
    st(m, -0x108, f(m, -0x64) * f(m, -0x64) * f(m, -0x6c) + f(m, -0xf8) * f(m, -0xf8) * f(m, -0x88));
    let c48 = m.u32(p(-0x48));
    m.set_u32(p(-4), c48);
    st(m, -0xac, f(m, -0x50) - f(m, -4) * f(m, -0x38));
    st(m, -0xa0, (f(m, -4) - f(m, -0x170)) * f18);
    if f(m, -0xa0) > 0.0 {
        let len = vec_len(m, p(-0x5c));
        st(m, -0x1128, len);
        st(m, -0x112c, f(m, -0xa0) * f20);
        if !(f(m, -0x112c) >= f(m, -0x1128)) {
            if f(m, -0x1128) != 0.0 && !f(m, -0x1128).is_nan() {
                let v = (f(m, -0x1128) - f(m, -0x112c)) / f(m, -0x1128);
                st(m, -0x1128, v);
            }
        } else {
            st(m, -0x1128, 0.0);
        }
        vec_scale_in_place(m, p(-0x5c), p(-0x1220), f(m, -0x1128) as f32);
    }
    vec_scale(m, p(-0x122c), p(-0x15c), f(m, -4) as f32);
    copy3(m, p(-0xc8), p(-0x122c));
    vec_scale(m, p(-0x1238), p(-0x17c), f(m, -0xac) as f32);
    copy3(m, p(-0xbc), p(-0x1238));
    vec_add(m, p(-0x34), p(-0xc8), p(-0x104));
    vec_add(m, p(-0x28), p(-0xbc), p(-0x5c));
    rotate(m, p(-0x16c), p(-0x34), a2);
    crate::math::vec_div(m, p(-0xe0), [m.f32(p(-0x28)), m.f32(p(-0x28) + 4), m.f32(p(-0x28) + 8)], a2c);
    let t1 = vec_dot(m, p(-0xc8), p(-0xc8));
    st(m, -0x160, t1 * f(m, -0x6c));
    let t2 = vec_dot(m, p(-0xbc), p(-0xbc));
    st(m, -0x60, t2 * f(m, -0x88));
    st(m, -0xf0, f(m, -0x160) + f(m, -0x60));
    m.set_u32(p(-0x80), 0x3f80_0000);
    if f(m, -0xe8) > 0.0 {
        st(m, -0x80, f(m, -0xf0) / f(m, -0xe8));
        if f(m, -0x80) > 1.01 {
            lg.log(0x673fac);
        }
    }
    vec_cross(m, p(-0x1244), p(-0xe0), p(-0x188));
    vec_scale(m, p(-0x1138), p(-0x1244), f(m, -0x12c) as f32);
    rotate(m, p(-0x1150), p(-0x1138), a2);
    vec_add(m, p(-0x1144), p(-0x16c), p(-0x1150));
    let d7 = vec_dot(m, p(-0x1144), a1);
    st(m, -0x1154, d7);
    if f(m, -0x1154) < 0.0 || f(m, -0x1154).is_nan() {
        st(m, -0x1270, f(m, -0x1154) - 0.01);
        vec_scale(m, p(-0x1250), a1, f(m, -0x1270) as f32);
        vec_sub_in_place(m, p(-0x16c), p(-0x125c), p(-0x1250));
        st(m, -0x1154, f(m, -0x1154) * 1.01);
    }
    vec_soft_limit(m, p(-0xe0));
    copy3(m, o30, p(-0x16c));
    copy3(m, o34, p(-0xe0));
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
    #[test]
    fn native_moving_item_reset_step() {
        struct Log(Vec<String>);
        impl ItemVirtuals for Log {
            fn virt(&mut self, m: &mut Mem, _: u32, slot: u32, args: &[u32]) {
                let hx = |a: u32| m.read(a, 12).iter().map(|b| format!("{b:02x}")).collect::<String>();
                self.0.push(if slot == 0xa4 { format!("a4|{}|{}", hx(args[0]), hx(args[1])) } else { "74".to_string() });
            }
        }
        let this = 0x2a010000u32;
        let mut n = 0;
        for (c, line) in include_str!("../../../tools/emu/golden_item_reset_step.tsv").lines().enumerate() {
            let v: Vec<&str> = line.split('\t').collect();
            let mut m = Mem::new();
            for it in v[1].split(',') {
                let (o, x) = it.split_once(':').unwrap();
                m.write(this + u32::from_str_radix(o, 16).unwrap(), &unhex(x));
            }
            let before = m.read(this, 0x1000);
            let mut log = Log(vec![]);
            moving_item_reset_step(&mut m, this, v[0].parse().unwrap(), &mut log);
            let after = m.read(this, 0x1000);
            let diff: Vec<String> = (0..0x1000)
                .step_by(4)
                .filter(|&i| after[i..i + 4] != before[i..i + 4])
                .map(|i| format!("{i:x}:{}", after[i..i + 4].iter().map(|b| format!("{b:02x}")).collect::<String>()))
                .collect();
            assert_eq!(diff.join(","), v[2], "case {c}");
            assert_eq!(log.0.join(";"), v[3], "case {c}: events");
            n += 1;
        }
        assert_eq!(n, 300);
    }
    #[test]
    fn native_pose_delta() {
        let base = 0x2a010000u32;
        let at = |i: u32| base + 0x1000 * i;
        let (a, b, m2, op, oa) = (at(0), at(1), at(2), at(3), at(4));
        let mut n = 0;
        for (c, line) in include_str!("../../../tools/emu/golden_pose_delta.tsv").lines().enumerate() {
            let v: Vec<&str> = line.split('\t').collect();
            let mut m = Mem::new();
            crate::globals::install_constants(&mut m);
            m.write(a, &unhex(v[0]));
            m.write(b, &unhex(v[1]));
            m.write(m2, &unhex(v[2]));
            pose_delta(&mut m, a, b, m2, op, oa);
            let hex = |p: u32| m.read(p, 12).iter().map(|x| format!("{x:02x}")).collect::<String>();
            assert_eq!(hex(op), v[3], "case {c}: pos");
            assert_eq!(hex(oa), v[4], "case {c}: angles");
            n += 1;
        }
        assert_eq!(n, 300);
    }

    struct UpOwner {
        ev: Vec<String>,
        obj: u32,
        r5ba7: u32,
        r8090: u32,
        vdc: Vec<u8>,
    }
    fn hx(m: &Mem, a: u32, n: usize) -> String {
        m.read(a, n).iter().map(|x| format!("{x:02x}")).collect()
    }
    impl ItemVirtuals for UpOwner {
        fn virt(&mut self, m: &mut Mem, _this: u32, slot: u32, args: &[u32]) {
            let e = match slot {
                0x1c => format!("v1c|{:x}", args[0]),
                0x90 => format!("v90|{}|{:x}", hx(m, args[0], 12), args[1]),
                0x94 => format!("v94|{}|{:x}", hx(m, args[0], 12), args[1]),
                0xa0 => format!("va0|{}", hx(m, args[0], 0xd0)),
                0xdc => {
                    m.write(args[0], &self.vdc);
                    "vdc".to_string()
                }
                _ => panic!("slot {slot:x}"),
            };
            self.ev.push(e);
        }
    }
    impl UpdateOwner for UpOwner {
        fn range(&mut self, m: &mut Mem, vec: u32, text: u32, limit: u32) {
            self.ev.push(format!("rng|{}|{:x}|{:x}", hx(m, vec, 12), text, limit));
        }
        fn world_object(&mut self, _m: &mut Mem, id: u32) -> u32 {
            self.ev.push(format!("c9e0|{id:x}"));
            self.obj
        }
        fn room(&mut self, _m: &mut Mem, a: u32, b: u32, c: u32) -> u32 {
            self.ev.push(format!("5ba7|{a:x},{b:x},{c:x}"));
            self.r5ba7
        }
        fn contact(&mut self, _m: &mut Mem, which: u32, a: u32, b: u32, c: u32) {
            let n = match which {
                0x57e2e0 => "e2e0",
                0x57e540 => "e540",
                0x580a00 => "0a00",
                _ => "08c0",
            };
            self.ev.push(format!("{n}|{a:x},{b:x},{c:x}"));
        }
        fn contact_1990(&mut self, _m: &mut Mem, a: u32, b: u32, c: u32, d: u32) {
            self.ev.push(format!("1990|{a:x},{b:x},{c:x},{d:x}"));
        }
        fn r588090(&mut self, _m: &mut Mem, this: u32) -> u32 {
            self.ev.push(format!("8090|{this:x}"));
            self.r8090
        }
        fn error(&mut self) {
            self.ev.push("err".to_string());
        }
    }
    #[test]
    fn native_moving_item_update() {
        let (this, clk) = (0x2a010000u32, 0x2a016000u32);
        let mut n = 0;
        let mut bad = 0;
        for (c, line) in include_str!("../../../tools/emu/golden_item_update.tsv").lines().enumerate() {
            let v: Vec<&str> = line.split('\t').collect();
            let mut m = Mem::new();
            crate::globals::install_constants(&mut m);
            m.set_u32(crate::globals::CLOCK_PTR, clk);
            m.write(crate::globals::FIXED_STEP_MS, &[0, 0]);
            for it in v[2].split(',').filter(|s| !s.is_empty()) {
                let (o, x) = it.split_once(':').unwrap();
                m.write(this + u32::from_str_radix(o, 16).unwrap(), &unhex(x));
            }
            m.write(clk + 0xb98, &unhex(v[7]));
            let before = m.read(this, 0x1000);
            let flags: u32 = v[0].parse().unwrap();
            let dt = f32::from_bits(u32::from_le_bytes(unhex(v[1]).try_into().unwrap()));
            let mut o = UpOwner {
                ev: vec![],
                obj: u32::from_str_radix(v[3], 16).unwrap(),
                r5ba7: u32::from_str_radix(v[4], 16).unwrap(),
                r8090: v[5].parse().unwrap(),
                vdc: unhex(v[6]),
            };
            moving_item_update(&mut m, this, flags, dt, &mut o);
            let after = m.read(this, 0x1000);
            let diff: String = (0..0x1000usize)
                .step_by(4)
                .filter(|&i| after[i..i + 4] != before[i..i + 4])
                .map(|i| format!("{:x}:{}", i, after[i..i + 4].iter().map(|x| format!("{x:02x}")).collect::<String>()))
                .collect::<Vec<_>>()
                .join(",");
            let ev = o.ev.join(";");
            if diff != v[8] || ev != v[9] {
                bad += 1;
                if bad <= 3 {
                    let (gd, ed): (Vec<&str>, Vec<&str>) = (v[8].split(',').collect(), diff.split(',').collect());
                    let dif: Vec<&&str> = gd.iter().filter(|x| !ed.contains(x)).collect();
                    let ext: Vec<&&str> = ed.iter().filter(|x| !gd.contains(x)).collect();
                    eprintln!("case {c} flags {flags}: missing {dif:?}\n  extra {ext:?}\n  ev ours {ev}\n  ev gold {}", v[9]);
                }
            }
            n += 1;
        }
        assert_eq!(bad, 0, "{bad} of {n} differ");
        assert_eq!(n, 500);
    }

    struct ModeTest {
        ev: Vec<String>,
        ctx: Vec<u8>,
        casts: Vec<(u32, f32)>,
        n: usize,
        room: u32,
    }
    impl ItemVirtuals for ModeTest {
        fn virt(&mut self, m: &mut Mem, _this: u32, slot: u32, args: &[u32]) {
            self.ev.push(match slot {
                0x1c => format!("v1c|{:x}", args[0]),
                0x8c => format!("v8c|{:x}|{}", args[0], hx(m, args[1], 48)),
                _ => panic!(),
            });
        }
    }
    impl ModeOwner for ModeTest {
        fn ctx_init(&mut self, m: &mut Mem, ctx: u32) {
            m.write(ctx, &self.ctx);
        }
        fn cast(&mut self, m: &mut Mem, this: u32, _ctx: u32, out: u32) -> u32 {
            let (h, d) = self.casts[self.n.min(self.casts.len() - 1)];
            self.n += 1;
            self.ev.push(format!("cast|{this:x}"));
            m.set_f32(out, d);
            h
        }
        fn room(&mut self, m: &mut Mem, a: u32, b: u32, c: u32) -> u32 {
            self.ev.push(format!("5ba7|{a:x},{b:x},{}", hx(m, c, 12)));
            self.room
        }
    }
    #[test]
    fn native_moving_item_mode() {
        let this = 0x2a010000u32;
        let mut n = 0;
        for (c, line) in include_str!("../../../tools/emu/golden_mode_set.tsv").lines().enumerate() {
            let v: Vec<&str> = line.split('\t').collect();
            let mut m = Mem::new();
            crate::globals::install_constants(&mut m);
            for it in v[2].split(',').filter(|s| !s.is_empty()) {
                let (o, x) = it.split_once(':').unwrap();
                m.write(this + u32::from_str_radix(o, 16).unwrap(), &unhex(x));
            }
            let before = m.read(this, 0x1000);
            let mut o = ModeTest {
                ev: vec![],
                ctx: unhex(v[3]),
                casts: v[4].split(';').map(|s| {
                    let (h, d) = s.split_once(',').unwrap();
                    (h.parse().unwrap(), f32::from_bits(u32::from_le_bytes(unhex(d).try_into().unwrap())))
                }).collect(),
                n: 0,
                room: u32::from_str_radix(v[5], 16).unwrap(),
            };
            if v[0] == "0" {
                moving_item_flags_reset(&mut m, this);
            } else {
                moving_item_set_mode(&mut m, this, v[1].parse().unwrap(), &mut o);
            }
            let after = m.read(this, 0x1000);
            let diff: String = (0..0x1000usize)
                .step_by(4)
                .filter(|&i| after[i..i + 4] != before[i..i + 4])
                .map(|i| format!("{:x}:{}", i, after[i..i + 4].iter().map(|x| format!("{x:02x}")).collect::<String>()))
                .collect::<Vec<_>>()
                .join(",");
            assert_eq!(diff, v[6], "case {c}: memory");
            assert_eq!(o.ev.join(";"), v[7], "case {c}: events");
            n += 1;
        }
        assert_eq!(n, 400);
    }

    struct NbTest {
        ev: Vec<String>,
        a020: Vec<u32>,
        c70: Vec<u32>,
    }
    impl NeighbourOwner for NbTest {
        fn pair_query(&mut self, m: &mut Mem, a: u32, mat: u32, set: u32, b: u32, flag: u32) -> u32 {
            self.ev.push(format!("a020|{a:x}|{}|{set:x}|{b:x}|{flag:x}", hx(m, mat, 48)));
            if self.a020.is_empty() { 0 } else { self.a020.remove(0) }
        }
        fn swept_record(&mut self, m: &mut Mem, p: u32, q: u32, rad: u32, set: u32, body: u32, mult: u32) -> u32 {
            self.ev.push(format!("c70|{}|{}|{rad:x}|{set:x}|{body:x}|{mult:x}", hx(m, p, 12), hx(m, q, 12)));
            let k = if self.c70.is_empty() { 0 } else { self.c70.remove(0) };
            let c = m.u32(set + 0xb00);
            m.set_u32(set + 0xb00, c + k);
            k
        }
    }
    #[test]
    fn native_body_neighbours() {
        let (body, set, arg1, clk) = (0x2a010000u32, 0x2a020000u32, 0x2a028000u32, 0x2a02a000u32);
        let mut n = 0;
        for (c, line) in include_str!("../../../tools/emu/golden_neighbours.tsv").lines().enumerate() {
            let v: Vec<&str> = line.split('\t').collect();
            let mut m = Mem::new();
            crate::globals::install_constants(&mut m);
            m.set_u32(crate::globals::CLOCK_PTR, clk);
            m.write(crate::globals::FIXED_STEP_MS, &[0, 0]);
            for it in v[0].split(',').filter(|s| !s.is_empty()) {
                let (o, x) = it.split_once(':').unwrap();
                m.write(u32::from_str_radix(o, 16).unwrap(), &unhex(x));
            }
            let before = m.read(set, 0x1000);
            let nums = |t: &str| t.split(',').filter(|s| !s.is_empty()).map(|x| x.parse().unwrap()).collect::<Vec<u32>>();
            let mut o = NbTest { ev: vec![], a020: nums(v[4]), c70: nums(v[5]) };
            let res = body_neighbours(&mut m, body, arg1, set, &mut o);
            let after = m.read(set, 0x1000);
            let diff: String = (0..0x1000usize)
                .step_by(4)
                .filter(|&i| after[i..i + 4] != before[i..i + 4])
                .map(|i| format!("{:x}:{}", i, after[i..i + 4].iter().map(|x| format!("{x:02x}")).collect::<String>()))
                .collect::<Vec<_>>()
                .join(",");
            assert_eq!(res.to_string(), v[1], "case {c}: result");
            assert_eq!(diff, v[2], "case {c}: set");
            assert_eq!(o.ev.join(";"), v[3], "case {c}: events");
            n += 1;
        }
        assert_eq!(n, 400);
    }

    struct PcTest {
        ev: Vec<String>,
        blobs: Vec<Vec<u8>>,
        hits: Vec<u32>,
        room: u32,
        k: usize,
    }
    impl ItemVirtuals for PcTest {
        fn virt(&mut self, _m: &mut Mem, _this: u32, slot: u32, args: &[u32]) {
            assert_eq!(slot, 0x1c);
            self.ev.push(format!("v1c|{:x}", args[0]));
        }
    }
    impl PointCastOwner for PcTest {
        fn ctx_init(&mut self, m: &mut Mem, ctx: u32) {
            self.k = 0;
            m.write(ctx, &self.blobs[0]);
        }
        fn room(&mut self, m: &mut Mem, a: u32, b: u32, c: u32) -> u32 {
            self.ev.push(format!("5ba7|{a:x}|{}|{}", hx(m, b, 12), hx(m, c, 12)));
            self.room
        }
        fn cast(&mut self, m: &mut Mem, which: u32, room: u32, p0: u32, p1: u32, ctx: u32) -> u32 {
            let name = if which == 0x4e67d1 { "c67d1" } else { "c6ac4" };
            self.ev.push(format!("{name}|{room:x}|{}|{}", hx(m, p0, 12), hx(m, p1, 12)));
            self.k += 1;
            let b = self.blobs[(self.k - 1).min(self.blobs.len() - 1)].clone();
            m.write(ctx, &b);
            if self.hits.is_empty() { 0 } else { self.hits.remove(0) }
        }
    }
    #[test]
    fn native_body_point_contacts() {
        let (body, set, arg1) = (0x2a010000u32, 0x2a020000u32, 0x2a028000u32);
        let _ = arg1;
        let mut n = 0;
        for (c, line) in include_str!("../../../tools/emu/golden_point_contacts.tsv").lines().enumerate() {
            let v: Vec<&str> = line.split('\t').collect();
            let mut m = Mem::new();
            crate::globals::install_constants(&mut m);
            for it in v[0].split(',').filter(|s| !s.is_empty()) {
                let (o, x) = it.split_once(':').unwrap();
                m.write(u32::from_str_radix(o, 16).unwrap(), &unhex(x));
            }
            let (b0, s0) = (m.read(body, 0x3000), m.read(set, 0x1000));
            let mut o = PcTest {
                ev: vec![],
                blobs: v[2].split(';').map(unhex).collect(),
                hits: v[3].split(',').map(|x| x.parse().unwrap()).collect(),
                room: u32::from_str_radix(v[4], 16).unwrap(),
                k: 0,
            };
            let res = body_point_contacts(&mut m, body, arg1, set, v[1].parse().unwrap(), &mut o);
            let d = |a: &[u8], b: &[u8]| -> String {
                (0..a.len())
                    .step_by(4)
                    .filter(|&i| a[i..i + 4] != b[i..i + 4])
                    .map(|i| format!("{:x}:{}", i, b[i..i + 4].iter().map(|x| format!("{x:02x}")).collect::<String>()))
                    .collect::<Vec<_>>()
                    .join(",")
            };
            assert_eq!(res.to_string(), v[5], "case {c}: result");
            assert_eq!(d(&b0, &m.read(body, 0x3000)), v[6], "case {c}: body");
            assert_eq!(d(&s0, &m.read(set, 0x1000)), v[7], "case {c}: set");
            assert_eq!(o.ev.join(";"), v[8], "case {c}: events");
            n += 1;
        }
        assert_eq!(n, 500);
    }

    #[test]
    fn native_segment_box_test() {
        let (body, pp, qq) = (0x2a010000u32, 0x2a01a000u32, 0x2a01b000u32);
        let mut n = 0;
        for (c, line) in include_str!("../../../tools/emu/golden_box_reject.tsv").lines().enumerate() {
            let v: Vec<&str> = line.split('\t').collect();
            let mut m = Mem::new();
            for it in v[0].split(',').filter(|s| !s.is_empty()) {
                let (o, x) = it.split_once(':').unwrap();
                m.write(u32::from_str_radix(o, 16).unwrap(), &unhex(x));
            }
            assert_eq!(segment_box_test(&mut m, body, pp, qq).to_string(), v[1], "case {c}");
            n += 1;
        }
        assert_eq!(n, 600);
    }

    #[test]
    fn native_record_leaves() {
        let (this, out) = (0x2a010000u32, 0x2a014000u32);
        let mut n = 0;
        for (c, line) in include_str!("../../../tools/emu/golden_small_leaves.tsv").lines().enumerate() {
            let v: Vec<&str> = line.split('\t').collect();
            let mut m = Mem::new();
            crate::globals::install_constants(&mut m);
            for it in v[1].split(',').filter(|s| !s.is_empty()) {
                let (o, x) = it.split_once(':').unwrap();
                m.write(this + u32::from_str_radix(o, 16).unwrap(), &unhex(x));
            }
            m.write(out, &unhex(v[3]));
            let before = m.read(this, 0x1800);
            let res = match v[0] {
                "0" => record_iter_next(&mut m, this, out),
                "1" => record_get(&mut m, this, out, v[2].parse().unwrap()),
                _ => {
                    record_zero(&mut m, out);
                    v[4].parse().unwrap()
                }
            };
            let after = m.read(this, 0x1800);
            let diff: String = (0..0x1800usize)
                .step_by(4)
                .filter(|&i| before[i..i + 4] != after[i..i + 4])
                .map(|i| format!("{:x}:{}", i, after[i..i + 4].iter().map(|x| format!("{x:02x}")).collect::<String>()))
                .collect::<Vec<_>>()
                .join(",");
            assert_eq!(res.to_string(), v[4], "case {c}: result");
            assert_eq!(diff, v[5], "case {c}: this");
            assert_eq!(hx(&m, out, 0x1c), v[6], "case {c}: out");
            n += 1;
        }
        assert_eq!(n, 300);
    }

    #[test]
    fn native_hull_cast() {
        let (body, pp, rec) = (0x2a010000u32, 0x2a01a000u32, 0x2a01b000u32);
        let mut n = 0;
        for (c, line) in include_str!("../../../tools/emu/golden_hull_cast.tsv").lines().enumerate() {
            let v: Vec<&str> = line.split('\t').collect();
            let mut m = Mem::new();
            crate::globals::install_constants(&mut m);
            for it in v[0].split(',').filter(|s| !s.is_empty()) {
                let (o, x) = it.split_once(':').unwrap();
                m.write(u32::from_str_radix(o, 16).unwrap(), &unhex(x));
            }
            let t = f32::from_bits(u32::from_le_bytes(unhex(v[1]).try_into().unwrap()));
            let (b0, r0) = (m.read(body, 0x1800), m.read(rec, 0x100));
            let mut errs = vec![];
            let res = hull_cast(&mut m, pp, body, rec, t, &mut || errs.push("err|67df80"));
            let d = |a: &[u8], b: &[u8]| -> String {
                (0..a.len())
                    .step_by(4)
                    .filter(|&i| a[i..i + 4] != b[i..i + 4])
                    .map(|i| format!("{:x}:{}", i, b[i..i + 4].iter().map(|x| format!("{x:02x}")).collect::<String>()))
                    .collect::<Vec<_>>()
                    .join(",")
            };
            assert_eq!(res.to_string(), v[2], "case {c}: result");
            assert_eq!(d(&r0, &m.read(rec, 0x100)), v[3], "case {c}: rec");
            assert_eq!(d(&b0, &m.read(body, 0x1800)), v[4], "case {c}: body");
            assert_eq!(errs.join(";"), v[5], "case {c}: events");
            n += 1;
        }
        assert_eq!(n, 600);
    }

    #[test]
    fn native_segment_hull_cast() {
        let (body, pp, qq, rec) = (0x2a010000u32, 0x2a01a000u32, 0x2a01a100u32, 0x2a01b000u32);
        let mut n = 0;
        for (c, line) in include_str!("../../../tools/emu/golden_segment_hull.tsv").lines().enumerate() {
            let v: Vec<&str> = line.split('\t').collect();
            let mut m = Mem::new();
            crate::globals::install_constants(&mut m);
            for it in v[0].split(',').filter(|s| !s.is_empty()) {
                let (o, x) = it.split_once(':').unwrap();
                m.write(u32::from_str_radix(o, 16).unwrap(), &unhex(x));
            }
            let t = f32::from_bits(u32::from_le_bytes(unhex(v[1]).try_into().unwrap()));
            let (b0, r0) = (m.read(body, 0x1800), m.read(rec, 0x100));
            let mut errs = vec![];
            let res = segment_hull_cast(&mut m, pp, qq, body, rec, t, &mut || errs.push("err|67df80"));
            let d = |a: &[u8], b: &[u8]| -> String {
                (0..a.len())
                    .step_by(4)
                    .filter(|&i| a[i..i + 4] != b[i..i + 4])
                    .map(|i| format!("{:x}:{}", i, b[i..i + 4].iter().map(|x| format!("{x:02x}")).collect::<String>()))
                    .collect::<Vec<_>>()
                    .join(",")
            };
            assert_eq!(res.to_string(), v[2], "case {c}: result");
            assert_eq!(d(&r0, &m.read(rec, 0x100)), v[3], "case {c}: rec");
            assert_eq!(d(&b0, &m.read(body, 0x1800)), v[4], "case {c}: body");
            assert_eq!(errs.join(";"), v[5], "case {c}: events");
            n += 1;
        }
        assert_eq!(n, 600);
    }

    struct PqTest {
        ev: Vec<String>,
        blobs: Vec<Vec<u8>>,
        hits: Vec<u32>,
        k: usize,
    }
    impl PairOwner for PqTest {
        fn ctx_init(&mut self, m: &mut Mem, ctx: u32) {
            self.k = 0;
            m.write(ctx, &self.blobs[0]);
        }
        fn object_cast(&mut self, m: &mut Mem, obj: u32, p0: u32, p1: u32, ctx: u32) -> u32 {
            self.ev.push(format!("cast|{obj:x}|{}|{}", hx(m, p0, 12), hx(m, p1, 12)));
            self.k += 1;
            let b = self.blobs[self.k.min(self.blobs.len() - 1)].clone();
            m.write(ctx, &b);
            if self.hits.is_empty() { 0 } else { self.hits.remove(0) }
        }
        fn error(&mut self) {
            self.ev.push("err".to_string());
        }
    }
    #[test]
    fn native_body_pair_query() {
        let (a, o_, set, b1) = (0x2a010000u32, 0x2a014000u32, 0x2a018000u32, 0x2a01a000u32);
        let mut n = 0;
        for (c, line) in include_str!("../../../tools/emu/golden_pair_query.tsv").lines().enumerate() {
            let v: Vec<&str> = line.split('\t').collect();
            let mut m = Mem::new();
            crate::globals::install_constants(&mut m);
            for it in v[0].split(',').filter(|s| !s.is_empty()) {
                let (o, x) = it.split_once(':').unwrap();
                m.write(u32::from_str_radix(o, 16).unwrap(), &unhex(x));
            }
            let (a0, o0, s0) = (m.read(a, 0x2600), m.read(o_, 0x1800), m.read(set, 0x1000));
            let mut ow = PqTest {
                ev: vec![],
                blobs: v[2].split(';').map(unhex).collect(),
                hits: v[3].split(',').map(|x| x.parse().unwrap()).collect(),
                k: 0,
            };
            let res = body_pair_query(&mut m, a, b1, set, o_, v[1].parse().unwrap(), &mut ow);
            let d = |x: &[u8], y: &[u8]| -> String {
                (0..x.len())
                    .step_by(4)
                    .filter(|&i| x[i..i + 4] != y[i..i + 4])
                    .map(|i| format!("{:x}:{}", i, y[i..i + 4].iter().map(|z| format!("{z:02x}")).collect::<String>()))
                    .collect::<Vec<_>>()
                    .join(",")
            };
            assert_eq!(res.to_string(), v[4], "case {c}: result");
            assert_eq!(d(&a0, &m.read(a, 0x2600)), v[5], "case {c}: body");
            assert_eq!(d(&o0, &m.read(o_, 0x1800)), v[6], "case {c}: other");
            assert_eq!(d(&s0, &m.read(set, 0x1000)), v[7], "case {c}: set");
            assert_eq!(ow.ev.join(";"), v[8], "case {c}: events");
            n += 1;
        }
        assert_eq!(n, 500);
    }

    struct QoTest {
        ev: Vec<String>,
        script: Vec<(u32, Vec<(u32, Vec<u8>)>)>,
    }
    impl QoTest {
        fn run(&mut self, m: &mut Mem, name: &str, body: u32, mat: u32, set: u32, fl: Option<u32>) -> u32 {
            self.ev.push(format!("{name}|{body:x}|{mat:x}|{}", fl.map(|x| format!("{x:x}")).unwrap_or_default()));
            let (ret, patch) = self.script.remove(0);
            for (off, b) in patch {
                m.write(set + off, &b);
            }
            ret
        }
    }
    impl QueryOwner for QoTest {
        fn point_contacts(&mut self, m: &mut Mem, body: u32, mat: u32, set: u32, flags: u32) -> u32 {
            self.run(m, "pc", body, mat, set, Some(flags))
        }
        fn neighbours(&mut self, m: &mut Mem, body: u32, mat: u32, set: u32) -> u32 {
            self.run(m, "nb", body, mat, set, None)
        }
        fn collect(&mut self, m: &mut Mem, body: u32, mat: u32, set: u32) -> u32 {
            self.run(m, "col", body, mat, set, None)
        }
    }
    #[test]
    fn native_body_query_other() {
        let (body, mat, cell, rec) = (0x2a010000u32, 0x2a01a000u32, 0x2a01b000u32, 0x2a01c000u32);
        let mut n = 0;
        for (c, line) in include_str!("../../../tools/emu/golden_query_other.tsv").lines().enumerate() {
            let v: Vec<&str> = line.split('\t').collect();
            let mut m = Mem::new();
            for it in v[0].split(',').filter(|s| !s.is_empty()) {
                let (o, x) = it.split_once(':').unwrap();
                m.write(u32::from_str_radix(o, 16).unwrap(), &unhex(x));
            }
            let (b0, r0, c0) = (m.read(body, 0x1000), m.read(rec, 0x100), m.read(cell, 0x20));
            let script = v[2]
                .split(';')
                .map(|s| {
                    let (a, p) = s.split_once('|').unwrap();
                    let patch = p
                        .split(',')
                        .filter(|x| !x.is_empty())
                        .map(|x| {
                            let (o, h) = x.split_once(':').unwrap();
                            (u32::from_str_radix(o, 16).unwrap(), unhex(h))
                        })
                        .collect();
                    (a.parse().unwrap(), patch)
                })
                .collect();
            let mut ow = QoTest { ev: vec![], script };
            let res = body_query_other(&mut m, body, mat, cell, cell + 0x10, rec, v[1].parse().unwrap(), &mut ow);
            let d = |a: &[u8], b: &[u8]| -> String {
                (0..a.len())
                    .step_by(4)
                    .filter(|&i| a[i..i + 4] != b[i..i + 4])
                    .map(|i| format!("{:x}:{}", i, b[i..i + 4].iter().map(|x| format!("{x:02x}")).collect::<String>()))
                    .collect::<Vec<_>>()
                    .join(",")
            };
            assert_eq!(res.to_string(), v[3], "case {c}: result");
            assert_eq!(d(&b0, &m.read(body, 0x1000)), v[4], "case {c}: body");
            assert_eq!(d(&r0, &m.read(rec, 0x100)), v[5], "case {c}: rec");
            assert_eq!(d(&c0, &m.read(cell, 0x20)), v[6], "case {c}: cells");
            assert_eq!(ow.ev.join(";"), v[7], "case {c}: events");
            n += 1;
        }
        assert_eq!(n, 500);
    }

    struct CvTest {
        ev: Vec<String>,
        ret: u32,
    }
    impl CastVirt for CvTest {
        fn virt34(&mut self, m: &mut Mem, obj: u32, flags: u32, rec: u32) -> u32 {
            self.ev.push(format!("v34|{obj:x}|{flags:x}|{}|{}", hx(m, rec + 4, 48), hx(m, rec + 0x4c, 24)));
            self.ret
        }
    }
    #[test]
    fn native_cast_wrapper_virt3c() {
        let base = 0x2a010000u32;
        let (this, node, rec) = (base, base + 0x3000, base + 0x5000);
        let mut n = 0;
        for (c, line) in include_str!("../../../tools/emu/golden_cast_virt.tsv").lines().enumerate() {
            let v: Vec<&str> = line.split('\t').collect();
            let mut m = Mem::new();
            for it in v[0].split(',').filter(|s| !s.is_empty()) {
                let (o, x) = it.split_once(':').unwrap();
                m.write(u32::from_str_radix(o, 16).unwrap(), &unhex(x));
            }
            for i in 0..12 {
                m.set_f32(0x7025f0 + 4 * i, crate::matrix::IDENTITY[i as usize]);
            }
            let s0: Vec<Vec<u8>> = [rec, this, node].iter().map(|&a| m.read(a, 0x100)).collect();
            let mut ow = CvTest { ev: vec![], ret: v[2].parse().unwrap() };
            let res = cast_wrapper_virt3c(&mut m, this, v[1].parse().unwrap(), rec, &mut ow);
            let d = |a: &[u8], b: &[u8]| -> String {
                (0..a.len())
                    .step_by(4)
                    .filter(|&i| a[i..i + 4] != b[i..i + 4])
                    .map(|i| format!("{:x}:{}", i, b[i..i + 4].iter().map(|x| format!("{x:02x}")).collect::<String>()))
                    .collect::<Vec<_>>()
                    .join(",")
            };
            assert_eq!(res.to_string(), v[3], "case {c}: result");
            for (k, &a) in [rec, this, node].iter().enumerate() {
                assert_eq!(d(&s0[k], &m.read(a, 0x100)), v[4 + k], "case {c}: block {k}");
            }
            assert_eq!(ow.ev.join(";"), v[7], "case {c}: events");
            n += 1;
        }
        assert_eq!(n, 500);
    }

    struct Cv2Test {
        ev: Vec<String>,
        ret: u32,
    }
    impl CastVirt2 for Cv2Test {
        fn push(&mut self, m: &mut Mem, stack: u32, mat: u32, kind: u32) {
            self.ev.push(format!("push|{stack:x}|{}|{kind:x}", hx(m, mat, 48)));
        }
        fn pop(&mut self, _m: &mut Mem, stack: u32) {
            self.ev.push(format!("pop|{stack:x}"));
        }
        fn virt38(&mut self, m: &mut Mem, obj: u32, a0: u32, mat: u32, mat2: u32, stack: u32) -> u32 {
            self.ev.push(format!("v38|{obj:x}|{a0:x}|{}|{}|{stack:x}", hx(m, mat, 48), hx(m, mat2, 48)));
            self.ret
        }
    }
    #[test]
    fn native_cast_wrapper_virt3c_b() {
        let base = 0x2a010000u32;
        let (this, node, mat, mat2, stk) = (base, base + 0x3000, base + 0x5000, base + 0x6000, base + 0x7000);
        let mut n = 0;
        for (c, line) in include_str!("../../../tools/emu/golden_cast_virt2.tsv").lines().enumerate() {
            let v: Vec<&str> = line.split('\t').collect();
            let mut m = Mem::new();
            for it in v[0].split(',').filter(|s| !s.is_empty()) {
                let (o, x) = it.split_once(':').unwrap();
                m.write(u32::from_str_radix(o, 16).unwrap(), &unhex(x));
            }
            for i in 0..12 {
                m.set_f32(0x7025f0 + 4 * i, crate::matrix::IDENTITY[i as usize]);
            }
            let s0: Vec<Vec<u8>> = [(mat, 0x40), (this, 0x100), (node, 0x100)].iter().map(|&(a, n)| m.read(a, n)).collect();
            let mut ow = Cv2Test { ev: vec![], ret: v[2].parse().unwrap() };
            let res = cast_wrapper_virt3c_b(&mut m, this, v[1].parse().unwrap(), mat, mat2, stk, &mut ow);
            let d = |a: &[u8], b: &[u8]| -> String {
                (0..a.len())
                    .step_by(4)
                    .filter(|&i| a[i..i + 4] != b[i..i + 4])
                    .map(|i| format!("{:x}:{}", i, b[i..i + 4].iter().map(|x| format!("{x:02x}")).collect::<String>()))
                    .collect::<Vec<_>>()
                    .join(",")
            };
            assert_eq!(res.to_string(), v[3], "case {c}: result");
            for (k, &(a, n)) in [(mat, 0x40), (this, 0x100), (node, 0x100)].iter().enumerate() {
                assert_eq!(d(&s0[k], &m.read(a, n)), v[4 + k], "case {c}: block {k}");
            }
            assert_eq!(ow.ev.join(";"), v[7], "case {c}: events");
            n += 1;
        }
        assert_eq!(n, 500);
    }

    struct GqTest {
        ev: Vec<String>,
        script: Vec<(u32, Vec<(u32, Vec<u8>)>)>,
    }
    impl GroundOwner for GqTest {
        fn point_contacts(&mut self, m: &mut Mem, body: u32, set: u32, flags: u32) -> u32 {
            self.ev.push(format!("pc|{body:x}|{flags:x}"));
            let (ret, patch) = self.script.remove(0);
            for (off, b) in patch {
                m.write(set + off, &b);
            }
            ret
        }
    }
    #[test]
    fn native_body_ground_hit() {
        let (body, cell, rec) = (0x2a010000u32, 0x2a01b000u32, 0x2a01c000u32);
        let mut n = 0;
        for (c, line) in include_str!("../../../tools/emu/golden_ground_query.tsv").lines().enumerate() {
            let v: Vec<&str> = line.split('\t').collect();
            let mut m = Mem::new();
            for it in v[0].split(',').filter(|s| !s.is_empty()) {
                let (o, x) = it.split_once(':').unwrap();
                m.write(u32::from_str_radix(o, 16).unwrap(), &unhex(x));
            }
            let (b0, r0, c0) = (m.read(body, 0x1000), m.read(rec, 0x100), m.read(cell, 0x20));
            let script = v[1]
                .split(';')
                .map(|s| {
                    let (a, p) = s.split_once('|').unwrap();
                    let patch = p
                        .split(',')
                        .filter(|x| !x.is_empty())
                        .map(|x| {
                            let (o, h) = x.split_once(':').unwrap();
                            (u32::from_str_radix(o, 16).unwrap(), unhex(h))
                        })
                        .collect();
                    (a.parse().unwrap(), patch)
                })
                .collect();
            let mut ow = GqTest { ev: vec![], script };
            let res = body_ground_hit(&mut m, body, rec, cell, &mut ow);
            let d = |a: &[u8], b: &[u8]| -> String {
                (0..a.len())
                    .step_by(4)
                    .filter(|&i| a[i..i + 4] != b[i..i + 4])
                    .map(|i| format!("{:x}:{}", i, b[i..i + 4].iter().map(|x| format!("{x:02x}")).collect::<String>()))
                    .collect::<Vec<_>>()
                    .join(",")
            };
            assert_eq!(res.to_string(), v[2], "case {c}: result");
            assert_eq!(d(&b0, &m.read(body, 0x1000)), v[3], "case {c}: body");
            assert_eq!(d(&r0, &m.read(rec, 0x100)), v[4], "case {c}: rec");
            assert_eq!(d(&c0, &m.read(cell, 0x20)), v[5], "case {c}: cells");
            assert_eq!(ow.ev.join(";"), v[6], "case {c}: events");
            n += 1;
        }
        assert_eq!(n, 500);
    }

    struct CpTest {
        ev: Vec<String>,
    }
    impl ItemVirtuals for CpTest {
        fn virt(&mut self, m: &mut Mem, _this: u32, slot: u32, args: &[u32]) {
            assert_eq!(slot, 0x90);
            self.ev.push(format!("v90|{}|{:x}", hx(m, args[0], 12), args[1]));
        }
    }
    impl ContactPlaceOwner for CpTest {
        fn range(&mut self, m: &mut Mem, vec: u32, text: u32, limit: u32) {
            self.ev.push(format!("rng|{}|{text:x}|{limit:x}", hx(m, vec, 12)));
        }
    }
    #[test]
    fn native_contact_place() {
        let (body, rec, out) = (0x2a010000u32, 0x2a014000u32, 0x2a018000u32);
        let mut n = 0;
        for (c, line) in include_str!("../../../tools/emu/golden_contact_e2e0.tsv").lines().enumerate() {
            let v: Vec<&str> = line.split('\t').collect();
            let mut m = Mem::new();
            for it in v[0].split(',').filter(|s| !s.is_empty()) {
                let (o, x) = it.split_once(':').unwrap();
                m.write(u32::from_str_radix(o, 16).unwrap(), &unhex(x));
            }
            let (b0, r0, o0) = (m.read(body, 0x1000), m.read(rec, 0x100), m.read(out, 0x30));
            let mut ow = CpTest { ev: vec![] };
            contact_place(&mut m, body, rec, out, &mut ow);
            let d = |a: &[u8], b: &[u8]| -> String {
                (0..a.len())
                    .step_by(4)
                    .filter(|&i| a[i..i + 4] != b[i..i + 4])
                    .map(|i| format!("{:x}:{}", i, b[i..i + 4].iter().map(|x| format!("{x:02x}")).collect::<String>()))
                    .collect::<Vec<_>>()
                    .join(",")
            };
            assert_eq!(d(&b0, &m.read(body, 0x1000)), v[1], "case {c}: body");
            assert_eq!(d(&r0, &m.read(rec, 0x100)), v[2], "case {c}: rec");
            assert_eq!(d(&o0, &m.read(out, 0x30)), v[3], "case {c}: out");
            assert_eq!(ow.ev.join(";"), v[4], "case {c}: events");
            n += 1;
        }
        assert_eq!(n, 500);
    }

    struct CsTest {
        ev: Vec<String>,
    }
    impl ContactSolveLog for CsTest {
        fn format(&mut self, m: &mut Mem, a2: u32, a24: u32, a28: u32) {
            self.ev.push(format!("fmt|673ddc|{}|{}|{}", hx(m, a2, 48), hx(m, a24, 12), hx(m, a28, 12)));
        }
        fn log(&mut self, text: u32) {
            self.ev.push(format!("log|{text:x}"));
        }
    }
    #[test]
    fn native_contact_solve() {
        let (inp, out) = (0x2a010000u32, 0x2a011000u32);
        let (a0, a1, a2, a3, a24, a28, a2c, o30, o34) = (inp, inp + 0x10, inp + 0x20, inp + 0x60, inp + 0xa0, inp + 0xb0, inp + 0xc0, out, out + 0x10);
        let mut n = 0;
        for (c, line) in include_str!("../../../tools/emu/golden_contact_solve.tsv").lines().enumerate() {
            let v: Vec<&str> = line.split('\t').collect();
            let mut m = Mem::new();
            for it in v[0].split(',').filter(|s| !s.is_empty()) {
                let (o, x) = it.split_once(':').unwrap();
                m.write(u32::from_str_radix(o, 16).unwrap(), &unhex(x));
            }
            m.set_u32(0x702650, 0);
            m.set_u32(0x702654, 0);
            m.set_u32(0x702658, 0);
            let fl = |s: &str| f32::from_bits(u32::from_str_radix(&s.as_bytes().chunks(2).rev().map(|b| std::str::from_utf8(b).unwrap()).collect::<String>(), 16).unwrap());
            let i0 = m.read(inp, 0x200);
            let mut ow = CsTest { ev: vec![] };
            contact_solve(&mut m, [a0, a1, a2, a3], fl(v[1]), fl(v[2]), fl(v[3]), a24, a28, a2c, o30, o34, &mut ow);
            let i1 = m.read(inp, 0x200);
            let d: String = (0..0x200)
                .step_by(4)
                .filter(|&i| i0[i..i + 4] != i1[i..i + 4])
                .map(|i| format!("{:x}:{}", i, i1[i..i + 4].iter().map(|x| format!("{x:02x}")).collect::<String>()))
                .collect::<Vec<_>>()
                .join(",");
            assert_eq!(d, v[4], "case {c}: inputs");
            assert_eq!(hx(&m, out, 0x20), v[5], "case {c}: outputs");
            assert_eq!(ow.ev.join(";"), v[6], "case {c}: events");
            n += 1;
        }
        assert_eq!(n, 800);
    }
}

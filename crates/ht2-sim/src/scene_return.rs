//! `0x554670` (`__fastcall`, ecx = vehicle scene): the dispatcher of "Vehicle::returnToLast". It hands the
//! saved pose `+0x234` (and the vectors `+0x1d4`, `+0x1e0`) to the scene's body object (`+0x10`), to the
//! optional object `[+0x5464]` and, in the network mode `[+0x2aac] = 1`, to the car object `[+0x5460]`.
//! The callees are engine objects that are not ported yet (`ReturnOwner::call`, one entry per original
//! address); the order of the calls and their arguments are the port.
use crate::mem::Mem;

pub trait ReturnOwner {
    /// `call(address, this, args)` of the original callee: `0x581230`, `0x5812a0`, `0x5816d0`, `0x561d10`,
    /// `0x4f0fc0`, `0x4f1dd0`, `0x4f1e20`.
    fn call(&mut self, m: &mut Mem, address: u32, this: u32, args: &[u32]);
}

pub fn scene_return_to_last(m: &mut Mem, s: u32, owner: &mut dyn ReturnOwner) {
    let (pose, body) = (s + 0x234, s + 0x10);
    let id = m.u32(s + 0x334);
    owner.call(m, 0x581230, body, &[id, pose]);
    owner.call(m, 0x5812a0, body, &[s + 0x1d4, 0x67a3c4]);
    owner.call(m, 0x5816d0, body, &[s + 0x1e0, 0]);
    let extra = m.u32(s + 0x5464);
    if extra != 0 {
        let id = m.u32(s + 0x334);
        owner.call(m, 0x561d10, extra, &[id, pose]);
    }
    let car = m.u32(s + 0x5460);
    if car != 0 && m.u32(s + 0x2aac) == 1 {
        let id = m.u32(s + 0x334);
        owner.call(m, 0x4f0fc0, car, &[id, pose]);
        let car = m.u32(s + 0x5460);
        owner.call(m, 0x4f1dd0, car, &[s + 0x1d4]);
        let car = m.u32(s + 0x5460);
        owner.call(m, 0x4f1e20, car, &[s + 0x1e0]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Trace(Vec<String>);
    impl ReturnOwner for Trace {
        fn call(&mut self, _m: &mut Mem, address: u32, this: u32, args: &[u32]) {
            let a: Vec<String> = args.iter().map(|x| format!("{x:x}")).collect();
            self.0.push(format!("{address:x}:{this:x}:{}", a.join(",")));
        }
    }
    #[test]
    fn native_scene_return() {
        let hex = |s: &str| u32::from_str_radix(s, 16).unwrap();
        let scene = 0x2a010000u32;
        let mut n = 0;
        for (c, line) in include_str!("../../../tools/emu/golden_scene_return.tsv").lines().enumerate() {
            let v: Vec<&str> = line.split('\t').collect();
            let mut m = Mem::new();
            for it in v[0].split(',') {
                let (o, x) = it.split_once(':').unwrap();
                m.set_u32(scene + hex(o), hex(x));
            }
            let mut t = Trace(vec![]);
            scene_return_to_last(&mut m, scene, &mut t);
            assert_eq!(t.0.join(","), v[1], "case {c}");
            n += 1;
        }
        assert_eq!(n, 200);
    }
}

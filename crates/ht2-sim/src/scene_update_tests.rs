//! Whole `0x54ff80` against the original (`tools/emu/gen_scene_update.py`): the owners that are reached record into
//! one shared event list, the others (ray cast, effects, lights, road) panic because the gates of the cases keep them closed.
use crate::instruments::{ExhaustEmit, SceneEffectEvents, SceneFlash, SceneLightOwner, WheelEffect};
use crate::maneuver::ManeuverRoad;
use crate::mem::Mem;
use crate::scene_net::NetOwner;
use crate::scene_probe::ProbeOwner;
use crate::scene_return::ReturnOwner;
use crate::scene_update::{vehicle_scene_update, SceneOwners};
use crate::vehicle_reset::ResetOwner;
use std::cell::RefCell;
use std::rc::Rc;

fn unhex(s: &str) -> Vec<u8> {
    (0..s.len()).step_by(2).map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap()).collect()
}
fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

#[derive(Clone, Default)]
struct Shared(Rc<RefCell<Vec<String>>>);
impl NetOwner for Shared {
    fn send(&mut self, _m: &mut Mem, channel: i32, payload: &[u8]) {
        self.0.borrow_mut().push(format!("n|{channel}|{}", hex(payload)));
    }
}
impl ResetOwner for Shared {
    fn virt(&mut self, _m: &mut Mem, this: u32, slot: u32) {
        self.0.borrow_mut().push(format!("v|{slot:x}|{this:x}"));
    }
    fn call(&mut self, _m: &mut Mem, address: u32, this: u32, _args: &[u32]) {
        assert_eq!(address, 0x5d32f0);
        self.0.borrow_mut().push(format!("c|5d32f0|{this:x}|"));
    }
    fn log(&mut self, _m: &mut Mem, _text: &str) {
        panic!("debug log");
    }
}
impl ReturnOwner for Shared {
    fn call(&mut self, _m: &mut Mem, address: u32, this: u32, args: &[u32]) {
        let a: Vec<String> = args.iter().map(|x| format!("{x:x}")).collect();
        self.0.borrow_mut().push(format!("r|{address:x}|{this:x}|{}", a.join(",")));
    }
}

struct Never;
impl ProbeOwner for Never {
    fn cast(&mut self, _: &mut Mem, _: u32, _: u32, _: u32) -> u32 {
        panic!("ray cast")
    }
    fn set_case(&mut self, _: &mut Mem, _: u32, _: i32) {
        panic!("set_case")
    }
}
impl SceneEffectEvents for Never {
    fn rand(&mut self, _: &mut Mem) -> i32 {
        panic!("effects")
    }
    fn scene_effect(&mut self, _: &mut Mem, _: [f32; 3], _: [f32; 3]) {
        panic!("effects")
    }
    fn wheel(&mut self, _: &mut Mem, _: WheelEffect) {
        panic!("effects")
    }
    fn exhaust_high(&mut self, _: &mut Mem) {
        panic!("effects")
    }
    fn exhaust(&mut self, _: &mut Mem, _: ExhaustEmit) {
        panic!("effects")
    }
    fn flash(&mut self, _: &mut Mem, _: SceneFlash) {
        panic!("effects")
    }
}
impl SceneLightOwner for Never {
    fn allocate(&mut self, _: &mut Mem, _: u32) -> u32 {
        panic!("lights")
    }
    fn register(&mut self, _: &mut Mem, _: u32, _: u32) {
        panic!("lights")
    }
}
impl ManeuverRoad for Never {
    fn zone(&mut self, _: &mut Mem, _: u32, _: u32, _: u32, _: u32, _: u32) -> u32 {
        panic!("road")
    }
    fn opposite(&mut self, _: &mut Mem, _: u32, _: [u32; 8]) -> u32 {
        panic!("road")
    }
    fn params(&mut self, _: &mut Mem, _: [u32; 8], _: u32) {
        panic!("road")
    }
    fn coordinates(&mut self, _: &mut Mem, _: u32, _: [u32; 8]) {
        panic!("road")
    }
    fn tangent(&mut self, _: &mut Mem, _: u32, _: [u32; 8]) -> u32 {
        panic!("road")
    }
    fn one_way(&mut self, _: &mut Mem, _: u32) -> u32 {
        panic!("road")
    }
}

#[test]
fn native_scene_update() {
    let h = |s: &str| u32::from_str_radix(s, 16).unwrap();
    let regions: [(&str, u32, usize); 4] =
        [("S", 0x2a010000, 0x5600), ("N", 0x2a020000, 0x3c00), ("O", 0x2a030000, 0x1000), ("T", 0x2a050000, 0x100)];
    let (mut with_events, mut cases) = (0, 0);
    for (c, line) in include_str!("../../../tools/emu/golden_scene_update.tsv").lines().enumerate() {
        let v: Vec<&str> = line.split('\t').collect();
        let mut m = Mem::new();
        crate::globals::install_constants(&mut m);
        let put = |m: &mut Mem, base: u32, spec: &str| {
            for it in spec.split(',').filter(|s| !s.is_empty()) {
                let (o, x) = it.split_once(':').unwrap();
                m.write(base + h(o), &unhex(x));
            }
        };
        put(&mut m, 0x2a010000, v[0]);
        put(&mut m, 0x2a020000, v[1]);
        put(&mut m, 0x2a030000, v[2]);
        put(&mut m, 0x2a100000, v[3]);
        put(&mut m, 0x2a110000, v[4]);
        put(&mut m, 0x2a050000, v[5]);
        let mut level = 0f64;
        for it in v[6].split(',').filter(|s| !s.is_empty()) {
            let (a, x) = it.split_once(':').unwrap();
            match a {
                "555500" => level = f32::from_le_bytes(unhex(x).try_into().unwrap()) as f64,
                "ar" => m.write(0x2a120000 + 0x58, &unhex(x)),
                "ac" => m.write(0x6ef2a0, &unhex(x)),
                _ => m.write(h(a), &unhex(x)),
            }
        }
        let before: Vec<Vec<u8>> = regions.iter().map(|&(_, a, n)| m.read(a, n)).collect();
        let shared = Shared::default();
        let (mut n1, mut n2, mut n3) = (shared.clone(), shared.clone(), shared.clone());
        let log = shared.0.clone();
        let mut imp = |m: &mut Mem, p: u32, w: u32| {
            log.borrow_mut().push(format!("i|{}|{}", hex(&m.read(p, 12)), hex(&m.read(w, 12))));
        };
        let mut lv = |_: &mut Mem, _: u32| level;
        let (mut probe, mut fx, mut light, mut road) = (Never, Never, Never, Never);
        let mut o = SceneOwners {
            impact: &mut imp,
            net: &mut n1,
            level: &mut lv,
            probe: &mut probe,
            reset: &mut n2,
            back: &mut n3,
            effects: &mut fx,
            light: &mut light,
            road: &mut road,
        };
        vehicle_scene_update(&mut m, 0x2a010000, 0x2a0f0000, &mut o);
        let events = shared.0.borrow().join(";");
        assert_eq!(events, v[7], "case {c}: events");
        let mut changes: Vec<String> = vec![];
        for (k, &(name, a, n)) in regions.iter().enumerate() {
            let after = m.read(a, n);
            for i in (0..n).step_by(4) {
                if after[i..i + 4] != before[k][i..i + 4] {
                    changes.push(format!("{name}:{i:x}:{}", hex(&after[i..i + 4])));
                }
            }
        }
        assert_eq!(changes.join(","), v[8], "case {c}: changes");
        if !events.is_empty() {
            with_events += 1;
        }
        cases += 1;
    }
    assert!(cases >= 20, "{cases}");
    assert!(with_events > 10);
}

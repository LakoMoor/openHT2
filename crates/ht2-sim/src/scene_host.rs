//! Host for `vehicle_scene_update` (`0x54ff80`): owns the flat memory the scene lives in and supplies every external
//! owner. What is ported goes through the ported code; what is not is a **stand-in**, listed here:
//!
//! * ray cast `[owner->vtable + 0x34]` (kind 7): answered by a `RayWorld` (the game's scene graph, `ht2-game/src/world.rs`)
//!   directly on the query record in the host memory;
//! * `SetCaseSwitch` (`0x4e480c`), the reset virtuals `[vtable+0x4c/+0xcc]`, `0x5d32f0`, the calls of `0x554670`
//!   (`0x581230` runs ported, only its virtuals `+0x88/+0x90/+0x94/+0x3c` are recorded; `0x4f0fc0`, ...), the sparks manager and the network impact announcement are recorded in `SceneHost::events`
//!   and have no effect on the game yet;
//! * effects (`0x551970`), lights and the maneuver chooser are switched off (their gates are closed by the caller or the
//!   owners do nothing), net messages are collected in `SceneHost::net`.
use crate::instruments::{ExhaustEmit, SceneEffectEvents, SceneFlash, SceneLightOwner, WheelEffect};
use crate::maneuver::ManeuverRoad;
use crate::mem::Mem;
use crate::scene_level::scene_level;
use crate::scene_net::NetOwner;
use crate::scene_probe::ProbeOwner;
use crate::scene_return::ReturnOwner;
use crate::scene_update::{vehicle_scene_update, SceneOwners};
use crate::vehicle_reset::ResetOwner;

/// The scene graph as the ray cast sees it. The query record (`q`, layout of `0x54cb90`: matrix at `+4`, hit point
/// `+0x34`, normal `+0x40`, segment `+0x4c`/`+0x58`, hit surface record pointer `+0x64`) lives in the host memory,
/// so a world that keeps its volumes in the same memory (`ht2-game/src/world.rs`) answers it with its own query.
/// Returns the original's flags (bit 0 = hit); the hit is left in the node's frame (`0x4e5698` brings it back).
pub trait RayWorld {
    fn cast(&mut self, m: &mut Mem, kind: u32, q: u32) -> u32;
}

/// No world: every ray misses.
pub struct EmptyWorld;
impl RayWorld for EmptyWorld {
    fn cast(&mut self, _: &mut Mem, _: u32, _: u32) -> u32 {
        0
    }
}

pub struct SceneHost {
    pub mem: Mem,
    pub scene: u32,
    /// Net messages `(channel, payload)` the scene sent this far.
    pub net: Vec<(i32, Vec<u8>)>,
    /// Stand-in calls: `"v|slot|this"`, `"c|addr|this"`, `"r|addr|this|args"`, `"case|object|index"`, `"impact"`.
    pub events: Vec<String>,
}

#[derive(Default)]
struct Shared {
    net: Vec<(i32, Vec<u8>)>,
    events: Vec<String>,
}
/// One value implements every owner trait; copies of it share the log and the world through `RefCell`s.
#[derive(Clone, Copy)]
struct Owners<'a, 'w> {
    world: &'a std::cell::RefCell<&'w mut dyn RayWorld>,
    shared: &'a std::cell::RefCell<Shared>,
}
impl NetOwner for Owners<'_, '_> {
    fn send(&mut self, _: &mut Mem, channel: i32, payload: &[u8]) {
        self.shared.borrow_mut().net.push((channel, payload.to_vec()));
    }
}
impl ResetOwner for Owners<'_, '_> {
    fn virt(&mut self, _: &mut Mem, this: u32, slot: u32) {
        self.shared.borrow_mut().events.push(format!("v|{slot:x}|{this:x}"));
    }
    fn call(&mut self, _: &mut Mem, address: u32, this: u32, _: &[u32]) {
        self.shared.borrow_mut().events.push(format!("c|{address:x}|{this:x}"));
    }
    fn log(&mut self, _: &mut Mem, _: &str) {}
}
impl ReturnOwner for Owners<'_, '_> {
    fn call(&mut self, m: &mut Mem, address: u32, this: u32, args: &[u32]) {
        if address == 0x581230 && args.len() == 2 {
            // ported: the clamp runs, its virtuals are stand-ins recorded as "v|slot|this"
            struct Rec<'s>(&'s std::cell::RefCell<Shared>);
            impl crate::moving_item::ItemVirtuals for Rec<'_> {
                fn virt(&mut self, _: &mut Mem, this: u32, slot: u32, _: &[u32]) {
                    self.0.borrow_mut().events.push(format!("v|{slot:x}|{this:x}"));
                }
            }
            crate::moving_item::moving_item_place(m, this, args[0], args[1], &mut Rec(self.shared));
            return;
        }
        let a: Vec<String> = args.iter().map(|x| format!("{x:x}")).collect();
        self.shared.borrow_mut().events.push(format!("r|{address:x}|{this:x}|{}", a.join(",")));
    }
}
impl ProbeOwner for Owners<'_, '_> {
    fn cast(&mut self, m: &mut Mem, _owner: u32, kind: u32, q: u32) -> u32 {
        self.world.borrow_mut().cast(m, kind, q)
    }
    fn set_case(&mut self, _: &mut Mem, object: u32, index: i32) {
        self.shared.borrow_mut().events.push(format!("case|{object:x}|{index}"));
    }
}
struct NoEffects;
impl SceneEffectEvents for NoEffects {
    fn rand(&mut self, _: &mut Mem) -> i32 {
        0
    }
    fn scene_effect(&mut self, _: &mut Mem, _: [f32; 3], _: [f32; 3]) {}
    fn wheel(&mut self, _: &mut Mem, _: WheelEffect) {}
    fn exhaust_high(&mut self, _: &mut Mem) {}
    fn exhaust(&mut self, _: &mut Mem, _: ExhaustEmit) {}
    fn flash(&mut self, _: &mut Mem, _: SceneFlash) {}
}
impl SceneLightOwner for NoEffects {
    fn allocate(&mut self, m: &mut Mem, size: u32) -> u32 {
        m.alloc(size)
    }
    fn register(&mut self, _: &mut Mem, _: u32, _: u32) {}
}
impl ManeuverRoad for NoEffects {
    fn zone(&mut self, _: &mut Mem, _: u32, _: u32, _: u32, _: u32, _: u32) -> u32 {
        0
    }
    fn opposite(&mut self, _: &mut Mem, out: u32, _: [u32; 8]) -> u32 {
        out
    }
    fn params(&mut self, _: &mut Mem, _: [u32; 8], _: u32) {}
    fn coordinates(&mut self, _: &mut Mem, _: u32, _: [u32; 8]) {}
    fn tangent(&mut self, _: &mut Mem, out: u32, _: [u32; 8]) -> u32 {
        out
    }
    fn one_way(&mut self, _: &mut Mem, _: u32) -> u32 {
        0
    }
}

const SCENE: u32 = 0x1000_0000;
const FRAME: u32 = 0x1200_0000;

impl SceneHost {
    pub fn new() -> Self {
        let mut mem = Mem::new();
        crate::globals::install_constants(&mut mem);
        SceneHost { mem, scene: SCENE, net: vec![], events: vec![] }
    }

    /// One `0x54ff80` step. `impact_strength` of the original goes through `impact_effect`; its network announcement and
    /// the sparks are stand-ins (recorded as `"impact"`).
    pub fn step(&mut self, world: &mut dyn RayWorld) {
        let shared = std::cell::RefCell::new(Shared { net: std::mem::take(&mut self.net), events: std::mem::take(&mut self.events) });
        {
            let world = std::cell::RefCell::new(world);
            let h = Owners { world: &world, shared: &shared };
            let (mut n, mut p, mut r, mut b) = (h, h, h, h);
            let mut imp = |m: &mut Mem, pt: u32, w: u32| {
                let mut sent = false;
                crate::instruments::impact_effect(m, pt, w, true, |_, _, _| sent = true, |_, _, _, _| {});
                shared.borrow_mut().events.push(format!("impact|{}", sent as u8));
            };
            let mut lv = |m: &mut Mem, s: u32| scene_level(m, s);
            let (mut fx, mut light, mut road) = (NoEffects, NoEffects, NoEffects);
            let mut o = SceneOwners {
                impact: &mut imp,
                net: &mut n,
                level: &mut lv,
                probe: &mut p,
                reset: &mut r,
                back: &mut b,
                effects: &mut fx,
                light: &mut light,
                road: &mut road,
            };
            vehicle_scene_update(&mut self.mem, self.scene, FRAME, &mut o);
        }
        let sh = shared.into_inner();
        self.net = sh.net;
        self.events = sh.events;
    }
}

impl Default for SceneHost {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_steps_an_empty_scene() {
        let mut h = SceneHost::new();
        h.step(&mut EmptyWorld);
        // nothing is switched on in a zeroed scene: no net traffic, +0xdf4 cleared
        assert!(h.net.is_empty());
        assert_eq!(h.mem.u32(h.scene + 0xdf4), 0);
    }

    struct Floor;
    impl RayWorld for Floor {
        fn cast(&mut self, m: &mut Mem, kind: u32, q: u32) -> u32 {
            assert_eq!(kind, 7);
            let node = 0x1100_0000;
            for i in 0..12u32 {
                m.set_f32(q + 4 + 4 * i, if i % 4 == 0 { 1.0 } else { 0.0 });
            }
            m.set_f32(q + 0x44, 1.0);
            m.set_u32(node, 0x4d);
            m.set_i32(node + 4, 5);
            m.set_u32(q + 0x64, node);
            1
        }
    }

    #[test]
    fn host_probe_reports_ray_hits() {
        let mut h = SceneHost::new();
        let s = h.scene;
        h.mem.set_u32(s + 0x2aa8, 0x1300_0000); // a ray cast owner
        h.mem.set_u32(s + 0x51a4, 1);
        // names the probe looks up: K_HFLAG_005 / K_FLAG_005 in the global table
        let (table, node1, node2, str1, str2) = (0x1400_0000u32, 0x1400_1000u32, 0x1400_1100u32, 0x1400_2000u32, 0x1400_2100u32);
        h.mem.set_u32(0x71e828, table);
        for (node, st, name, bucket) in [(node1, str1, "K_HFLAG_005", 0u32), (node2, str2, "K_FLAG_005", 0)] {
            let _ = bucket;
            for (i, b) in name.bytes().chain(std::iter::once(0)).enumerate() {
                h.mem.wr8(st + i as u32, b);
            }
            h.mem.set_u32(node + 0xc, st);
            h.mem.set_u32(node + 0x10, node + 1);
            let b = name.bytes().map(|x| x as u32).sum::<u32>() % 500;
            let head = h.mem.u32(table + 4 * b);
            if head == 0 {
                h.mem.set_u32(table + 4 * b, node);
            } else {
                h.mem.set_u32(head + 8, node);
            }
        }
        h.step(&mut Floor);
        assert!(h.events.iter().any(|e| e.starts_with("case|")), "{:?}", h.events);
    }
}

//! `0x54ff80` (`__fastcall`, ecx = vehicle scene): the per-frame update of the scene, assembled from the ported
//! parts in the original's order. The engine objects that are not ported are external owners (`SceneOwners`).
//!
//! | step | original | here |
//! |---|---|---|
//! | state flags, gauges | `0x54ff80..0x54fff3`, `0x5553e0` | inline, `gauges::scene_gauges` (message `0x546240`) |
//! | impact trigger | `0x54fff3..0x5501d7` | `scene_impact_trigger` (`0x54ef90` external) |
//! | body lean | `0x5501d7..0x550452` | `scene_body_lean` |
//! | flag bits | `0x550452..0x55054a` | `update_scene_flag_bits` |
//! | ray probe, requests, reset, return | `0x54cb90`, `0x54d0b0`, `0x580dd0`, `0x554670` | `scene_probe`, `scene_requests`, `vehicle_reset` (object `scene+0x10`), `scene_return_to_last` |
//! | wheel dust levels, effects | `0x55059f..0x5506cd`, `0x551970` | `scene_wheel_levels`, `update_scene_effects` |
//! | lights | `0x5506d4..0x551866` | `scene_light_rays` |
//! | maneuver, light timeout, input packet | `0x54f040`, `0x55186d`, `0x541e80` | `maneuver`, `scene_light_timeout`, `net_veh_in_put` |
use crate::gauges::scene_gauges;
use crate::instruments::{
    ftol_61d830, net_veh_in_put, scene_body_lean, scene_impact_trigger, scene_light_rays, scene_light_timeout,
    scene_wheel_levels, update_scene_effects, update_scene_flag_bits, SceneEffectEvents, SceneLightOwner,
};
use crate::maneuver::{maneuver, ManeuverRoad};
use crate::mem::Mem;
use crate::scene_net::{scene_effect_message, scene_gauge_message, NetOwner};
use crate::scene_probe::{scene_probe, ProbeOwner};
use crate::scene_requests::scene_requests;
use crate::scene_return::{scene_return_to_last, ReturnOwner};
use crate::vehicle_reset::{vehicle_reset, ResetOwner};

pub struct SceneOwners<'a, E: SceneEffectEvents, L: SceneLightOwner> {
    /// `0x54ef90(0, point, direction)`: the impact (`0x4ba130` with the network announcement).
    pub impact: &'a mut dyn FnMut(&mut Mem, u32, u32),
    /// The net stream behind `0x546240` / `0x546360`.
    pub net: &'a mut dyn NetOwner,
    /// `0x555500`: the level the gauge message carries.
    pub level: &'a mut dyn FnMut(&mut Mem, u32) -> f64,
    pub probe: &'a mut dyn ProbeOwner,
    pub reset: &'a mut dyn ResetOwner,
    pub back: &'a mut dyn ReturnOwner,
    pub effects: &'a mut E,
    pub light: &'a mut L,
    pub road: &'a mut dyn ManeuverRoad,
}

/// `f` is a scratch area of `0xe00` bytes standing in for the stack frames of the function and its callees.
pub fn vehicle_scene_update<E: SceneEffectEvents, L: SceneLightOwner>(m: &mut Mem, s: u32, f: u32, o: &mut SceneOwners<E, L>) {
    let on = m.u32(s + 0x390) != 0 || m.u32(s + 0x38c) != 0;
    m.set_u32(s + 0x390, on as u32);
    let v = m.u32(s + 0x38c);
    m.set_u32(s + 0x394, v);
    if m.u32(s + 0x38c) != 0 && m.u32(s + 0x5464) != 0 {
        let p = m.u32(s + 0x5464);
        m.set_u32(p + 0xda0, 1);
    }
    scene_gauges(m, s, &mut |m: &mut Mem, sc: u32| scene_gauge_message(m, sc, &mut *o.level, &mut *o.net));
    scene_impact_trigger(m, s, f, &mut *o.impact);
    scene_body_lean(m, s, f + 0x100);
    let dl = m.rd8(s + 0xf98);
    update_scene_flag_bits(m, s, dl);
    if m.rd8(s + 0x51a4) & 1 != 0 {
        scene_probe(m, s, f + 0x400, &mut *o.probe);
    }
    scene_requests(m, s, &mut |m: &mut Mem, sc: u32| scene_effect_message(m, sc, &mut *o.net));
    let clock = m.f64(m.u32(0x6f33ac) + 0x58);
    let k = ftol_61d830(clock) as i32;
    m.set_u32(s + 0x51b4, if k < 0x17 && k > 4 { 0 } else { 1 });
    vehicle_reset(m, s + 0x10, f + 0x600, &mut *o.reset);
    if m.u32(0x6796b0) & 8 != 0 {
        scene_return_to_last(m, s, &mut *o.back);
    }
    scene_wheel_levels(m, s);
    update_scene_effects(m, s, f + 0x200, &mut *o.effects);
    scene_light_rays(m, s, f + 0x800, &mut *o.light);
    maneuver(m, s, f + 0xc00, &mut *o.road);
    scene_light_timeout(m, s);
    net_veh_in_put(m, s + 0x2764);
    m.set_u32(s + 0xdf4, 0);
}

//! Particle effects of the player's truck: emission, ticking and camera-facing billboards.
//! STAND-IN: the vehicle layer `0x54xxxx` (the fields the original reads to decide when to emit)
//! is not ported, so emission is driven by throttle and speed of the ported vehicle instead.
use crate::particles::ParticleWorld;
use glam::{Mat4, Vec3, Vec4};
use ht2_formats::res::{Res, Section};
use ht2_render::render::{ObjectId, Renderer, TexturesId};
use ht2_render::scene::{load_material_quads, Scene};
use ht2_sim::mem::Mem;
use std::collections::{HashMap, HashSet};
use std::path::Path;

const PREFIXES: [&str; 4] = ["msmoke", "mvapor", "burn", "dust"];
/// Pool capacity per class (STAND-IN, see `particles.rs`).
const CAPACITY: u32 = 64;

pub struct Effects {
    mem: Mem,
    world: ParticleWorld,
    scene: Scene,
    textures: TexturesId,
    /// Quad objects per material index, grown on demand.
    objects: Vec<Vec<ObjectId>>,
    index: HashMap<String, usize>,
    used: Vec<usize>,
    /// Address of the stand-in vehicle scene object inside `mem`.
    scene_at: u32,
    previous_speed: f32,
}

fn effect_material_names(res_path: &Path) -> Result<Vec<String>, String> {
    let data = std::fs::read(res_path).map_err(|e| format!("{}: {e}", res_path.display()))?;
    let res = Res::parse(&data).map_err(|e| e.to_string())?;
    let mut names = Vec::new();
    for s in &res.sections {
        if let Section::Tokens { kind: "MATERIALS", tokens } = s {
            for t in tokens {
                if let Some(n) = t.split_whitespace().next() {
                    if PREFIXES.iter().any(|p| n.starts_with(p)) {
                        names.push(n.to_owned());
                    }
                }
            }
        }
    }
    names.sort();
    names.dedup();
    Ok(names)
}

/// Sinks of `0x551970` wired to the particle manager. A wheel effect with an owner pointer
/// (`WheelEffectPoint::Linked`) needs the second vehicle object of the original and is skipped (STAND-IN).
struct Events<'a> {
    world: &'a mut ParticleWorld,
}

impl ht2_sim::instruments::SceneEffectEvents for Events<'_> {
    fn rand(&mut self, m: &mut Mem) -> i32 {
        ht2_sim::globals::rand(m)
    }
    fn scene_effect(&mut self, m: &mut Mem, point: [f32; 3], direction: [f32; 3]) {
        self.world.emit_scene_effect(m, point, direction);
    }
    fn wheel(&mut self, m: &mut Mem, e: ht2_sim::instruments::WheelEffect) {
        if let ht2_sim::instruments::WheelEffectPoint::World(p) = e.point {
            self.world.emit_dust(m, p, e.direction, f32::from_bits(e.intensity));
        }
    }
    fn exhaust_high(&mut self, _: &mut Mem) {}
    fn exhaust(&mut self, m: &mut Mem, e: ht2_sim::instruments::ExhaustEmit) {
        self.world.emit_exhaust(m, e.position, e.velocity, e.p);
    }
    fn flash(&mut self, m: &mut Mem, f: ht2_sim::instruments::SceneFlash) {
        self.world.emit_flash(m, f.point, f32::from_bits(f.size));
    }
}

/// Build the constant part of the stand-in vehicle scene read by `0x551970`.
/// STAND-INS (the owners are the unported layer `0x54xxxx` and the vehicle tech record `0x697xxx`):
/// axis vectors `0x702620/30/40` = (0,1,0)/(0,0,1)/(1,0,0) (inferred from use: spread, up, sideways),
/// matrix `0x2c4` = identity, wheel points and the exhaust offset `(1.1,-3,2.6)`, the exhaust box,
/// flash points = two exhaust stacks (tech record `0x697a10`/`0x697a1c`, axis factor `0x6978cc` = 0), options object `[0x6cec6c]` all zero (`+0x514 = 0`: the scheduling gate lets every frame through), count scale 0.5, effect mode 4 (`p = 0.4`, no soot; modes 1 and 2 give `p >= 0.76`: soot and the `0x549e60` call, probably a damage state), `x` = 1.2*(1-throttle), ground level -100 (water branch and the low-vehicle branch off; `0x697fdc` is both the exhaust y and that test).
fn fill_scene(m: &mut Mem) -> u32 {
    let scene = m.alloc(0x14000);
    let cfg = m.alloc(0x20);
    let set = |m: &mut Mem, at: u32, v: &[f32]| v.iter().enumerate().for_each(|(i, x)| m.set_f32(at + i as u32 * 4, *x));
    set(m, 0x702620, &[0.0, 1.0, 0.0]);
    set(m, 0x702630, &[0.0, 0.0, 1.0]);
    set(m, 0x702640, &[1.0, 0.0, 0.0]);
    set(m, 0x702650, &[0.0, 0.0, 0.0]);
    set(m, scene + 0x2c4, &[1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0]);
    m.set_u32(scene + 0x28b4, 0);
    m.set_u32(scene + 0x51a4, 0x100);
    let (options, clock) = (m.alloc(0x600), m.alloc(0x2000));
    m.set_u32(0x6cec6c, options);
    m.set_u32(0x6ef1fc, clock);
    m.set_u32(scene + 0x5454, cfg);
    m.set_u32(cfg + 0xc, 4);
    m.write(scene + 0x2821, &[0xf]);
    let wheels: [[f32; 3]; 4] = [[-1.0, 2.5, 0.2], [1.0, 2.5, 0.2], [-1.0, -2.5, 0.2], [1.0, -2.5, 0.2]];
    for (k, w) in wheels.iter().enumerate() {
        m.set_u32(scene + 0x4fb0 + k as u32 * 4, k as u32);
        set(m, scene + 0x282c + k as u32 * 12, w);
        m.set_f32(0x697a10 + k as u32 * 12, w[0]);
    }
    m.set_f32(0x6978cc, 0.0);
    for (o, v) in [(0x697a10, [-1.1, -3.0, 2.6]), (0x697a1c, [1.1, -3.0, 2.6])] {
        set(m, o, &v);
    }
    m.set_f32(0x6979f8, -100.0);
    m.set_f32(0x697db8, 0.5);
    m.set_f32(0x697dc0, 0.5);
    for (o, v) in [(0x697dc4, 5.0), (0x697dc8, 6.0), (0x697dcc, 6.0), (0x697dd0, -5.0), (0x697dd4, -6.0), (0x697dd8, 0.0)] {
        m.set_f32(o, v);
    }
    for (o, v) in [(0x697fd8, 1.1), (0x697fdc, -3.0), (0x697fe0, 2.6)] {
        m.set_f32(o, v);
    }
    scene
}

/// One fixed step of the vehicle scene effects: fill the stand-in scene from the vehicle and run the ported `0x551970`.
fn run_scene_effects(m: &mut Mem, world: &mut ParticleWorld, scene: u32, body: Mat4, speed: f32, throttle: f32, event: u32) {
    let c = body.to_cols_array();
    let cols = [c[0], c[1], c[2], c[4], c[5], c[6], c[8], c[9], c[10], c[12], c[13], c[14]];
    for (i, v) in cols.into_iter().enumerate() {
        m.set_f32(scene + 0x204 + i as u32 * 4, v);
    }
    let velocity = body.y_axis.truncate() * speed;
    for i in 0..3 {
        m.set_f32(scene + 0x1b0 + i as u32 * 4, velocity[i]);
    }
    let slip = if speed.abs() > 5.0 { (speed.abs() / 20.0).min(1.0) } else { 0.0 };
    for i in 0..4 {
        m.set_f32(scene + 0x27f0 + i * 4, slip);
    }
    let cfg = m.u32(scene + 0x5454);
    m.set_f32(cfg + 8, (1.0 - throttle.clamp(0.0, 1.0)) * 1.2);
    // The impact event of `0x4ff800` (`[owner+0x3c18]`, the field `0x548250` reads as `[param_3+0x1228]`) feeds the
    // flash flags. STAND-IN: the record flag `[rec+0x108] & 0x40` is taken as set.
    let phys = scene + 0x12000;
    m.set_u32(phys + 0x1228, event);
    ht2_sim::instruments::update_flash_flags(m, scene, phys, 0x40);
    let mut events = Events { world };
    ht2_sim::instruments::update_scene_effects(m, scene, scene + 0x11000, &mut events);
}

impl Effects {
    pub fn load(renderer: &mut Renderer, game_dir: &Path) -> Result<Self, String> {
        let res_path = game_dir.join("COMMON").join("COMMON.RES");
        let names = effect_material_names(&res_path)?;
        let scene = load_material_quads(&res_path, &names)?;
        let textures = renderer.upload_textures(&scene);
        let have: HashSet<&str> = names.iter().map(|s| s.as_str()).collect();
        let mut mem = Mem::new();
        let world = ParticleWorld::new(&mut mem, CAPACITY, |n| have.contains(n)).map_err(|e| e.to_string())?;
        let index = names.iter().cloned().enumerate().map(|(i, n)| (n, i)).collect();
        let n = names.len();
        let scene_at = fill_scene(&mut mem);
        Ok(Self { mem, world, scene, textures, objects: vec![Vec::new(); n], index, used: vec![0; n], scene_at, previous_speed: 0.0 })
    }

    /// Fixed-step emission for the truck at `body` (model matrix, +Y forward, +Z up) moving at `speed`.
    /// The original decides this in `0x551970` from the vehicle object; here its scene fields are filled
    /// from the ported vehicle and the ported branches (`scene_wheel_requests`, `scene_exhaust_branch`)
    /// run on them in the original order. STAND-INS are listed in `fill_scene`.
    pub fn emit(&mut self, body: Mat4, speed: f32, throttle: f32, event: u32) {
        // STAND-IN: the autopilot demo does not run the powertrain input, so accelerating counts as full throttle
        let throttle = if throttle > 0.0 || speed > self.previous_speed { throttle.max(if speed > self.previous_speed { 1.0 } else { 0.0 }) } else { 0.0 };
        self.previous_speed = speed;
        let Self { mem, world, scene_at, .. } = self;
        run_scene_effects(mem, world, *scene_at, body, speed, throttle, event);
    }

    pub fn tick(&mut self, dt: f64) {
        self.world.tick(&mut self.mem, dt);
    }

    pub fn dump(&self) {
        for q in self.world.quads(&self.mem) {println!("particle {q:?}");}
    }

    pub fn live(&self) -> usize {
        self.world.live_count(&self.mem)
    }

    /// Place a quad object for every live particle, facing the camera (`vp` is the combined view-projection).
    pub fn place(&mut self, renderer: &mut Renderer, vp: Mat4) {
        for list in &self.objects {
            for id in list {
                renderer.set_visible(*id, false);
            }
        }
        self.used.iter_mut().for_each(|u| *u = 0);
        let inv = vp.inverse();
        let at = |x: f32, y: f32| {
            let v = inv * Vec4::new(x, y, 0.5, 1.0);
            v.truncate() / v.w
        };
        let right = (at(1.0, 0.0) - at(0.0, 0.0)).normalize();
        let up = (at(0.0, 1.0) - at(0.0, 0.0)).normalize();
        let forward = right.cross(up);
        for q in self.world.quads(&self.mem) {
            let Some(&i) = self.index.get(&q.material) else { continue };
            if self.used[i] == self.objects[i].len() {
                let ids = renderer.add_parts(self.textures, &self.scene, &[std::slice::from_ref(&self.scene.batches[i])]);
                self.objects[i].push(ids[0]);
            }
            let id = self.objects[i][self.used[i]];
            self.used[i] += 1;
            let (w, h) = (q.width.abs().max(0.01), q.height.abs().max(0.01));
            let model = Mat4::from_cols(right.extend(0.0) * w, up.extend(0.0) * h, forward.extend(0.0), Vec4::new(q.position[0], q.position[1], q.position[2], 1.0));
            renderer.set_model(id, model);
            renderer.set_visible(id, true);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rig() -> (Mem, ParticleWorld, u32) {
        let mut m = Mem::new();
        let world = ParticleWorld::new(&mut m, 16, |n| ["msmoke", "mvapor", "burn", "dust"].iter().any(|p| n.starts_with(p)) && n.len() <= 8).unwrap();
        let scene = fill_scene(&mut m);
        (m, world, scene)
    }

    /// A zero event raises no flash; event 1 makes `0x551970` emit the two flashes (class `0x650ed8`) through
    /// `update_flash_flags` -> `scene_flash_requests` -> `emit_flash`.
    #[test]
    fn effect_event_makes_flashes() {
        let (mut m, mut w, scene) = rig();
        run_scene_effects(&mut m, &mut w, scene, Mat4::IDENTITY, 0.0, 0.0, 0);
        assert_eq!(w.count_class(&m, 0x650ed8), 0);
        run_scene_effects(&mut m, &mut w, scene, Mat4::IDENTITY, 0.0, 0.0, 1);
        assert_eq!(w.count_class(&m, 0x650ed8), 2);
    }
}

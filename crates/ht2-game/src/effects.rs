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
        Ok(Self { mem, world, scene, textures, objects: vec![Vec::new(); n], index, used: vec![0; n] })
    }

    /// Fixed-step emission for the truck at `body` (model matrix, +Y forward, +Z up).
    pub fn emit(&mut self, body: Mat4, speed: f32, throttle: f32) {
        let m = &mut self.mem;
        let at = |p: Vec3| body.transform_point3(p).to_array();
        let dir = body.transform_vector3(Vec3::new(0.0, -0.3, 0.6)).to_array();
        if throttle > 0.0 {
            self.world.emit_exhaust(m, at(Vec3::new(1.1, -3.0, 2.6)), dir, throttle.min(1.0));
        }
        if speed.abs() > 5.0 {
            let back = body.transform_vector3(Vec3::new(0.0, -speed.abs() * 0.05, 0.4)).to_array();
            for x in [-1.0, 1.0] {
                self.world.emit_dust(m, at(Vec3::new(x, -2.5, 0.2)), back, (speed.abs() / 20.0).min(1.0));
            }
        }
    }

    /// Camera for the dust particles (`vp` is the combined view-projection); STAND-IN for the camera layer.
    pub fn set_camera(&mut self, vp: Mat4) {
        let inv = vp.inverse();
        let at = |x: f32, y: f32, z: f32| {
            let v = inv * Vec4::new(x, y, z, 1.0);
            v.truncate() / v.w
        };
        let eye = at(0.0, 0.0, 0.0);
        let forward = (at(0.0, 0.0, 0.9) - eye).normalize_or_zero();
        let up = (at(0.0, 1.0, 0.5) - at(0.0, 0.0, 0.5)).normalize_or_zero();
        self.world.set_camera(&mut self.mem, [0.0; 3], forward.to_array(), up.to_array());
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

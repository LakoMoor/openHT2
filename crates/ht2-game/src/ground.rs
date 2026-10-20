//! Drivable surface built from the world triangles, with a 2D grid for fast height queries.

use glam::Vec3;
use ht2_render::Scene;

/// Materials that are never driven on.
const NOT_GROUND: [&str; 9] = ["water", "shadow", "glass", "sky", "fog", "sun", "light", "glow", "tree"];

pub struct Ground {
    /// The collision of the world's scene graph, used by the ported vehicle.
    pub scene: crate::world::Scene,
    /// The road network of the `.way` files, which the computer drivers follow.
    pub roads: std::rc::Rc<ht2_sim::road::RoadNet>,
    cell: f32,
    min: (f32, f32),
    dims: (usize, usize),
    cells: Vec<Vec<u32>>,
    tris: Vec<[Vec3; 3]>,
}

fn material_name(scene: &Scene, material: u32) -> String {
    scene.material_names.get(material as usize).map_or(String::new(), |s| s.to_lowercase())
}

/// Every triangle of every scene together with the name of its material (lower case).
fn road_triangles<'a>(scenes: &'a [Scene], name_part: &'a str) -> impl Iterator<Item = [Vec3; 3]> + 'a {
    scenes
        .iter()
        .flat_map(|s| s.batches.iter().map(move |b| (s, b)))
        .filter(move |(s, b)| material_name(s, b.material).contains(name_part))
        .flat_map(|(_, b)| b.verts.chunks_exact(3).map(|t| [Vec3::from(t[0].pos), Vec3::from(t[1].pos), Vec3::from(t[2].pos)]))
}

fn longest_edge(t: &[Vec3; 3]) -> Vec3 {
    [t[1] - t[0], t[2] - t[1], t[0] - t[2]].into_iter().max_by(|a, b| a.length().total_cmp(&b.length())).unwrap()
}

/// Heading of a direction: 0 faces +Y and positive turns left, so forward is (-sin h, cos h).
fn heading_of(dir: Vec3) -> f32 {
    (-dir.x).atan2(dir.y)
}

impl Ground {
    pub fn new(scenes: &[Scene]) -> Self {
        let mut tris = Vec::new();
        for (scene, b) in scenes.iter().flat_map(|s| s.batches.iter().map(move |b| (s, b))) {
            let name = material_name(scene, b.material);
            if NOT_GROUND.iter().any(|w| name.contains(w)) {
                continue;
            }
            for t in b.verts.chunks_exact(3) {
                let tri = [Vec3::from(t[0].pos), Vec3::from(t[1].pos), Vec3::from(t[2].pos)];
                let n = (tri[1] - tri[0]).cross(tri[2] - tri[0]);
                // Skip walls and degenerate triangles: only surfaces a wheel can stand on.
                if n.length_squared() > 1e-8 && n.normalize().z.abs() > 0.25 {
                    tris.push(tri);
                }
            }
        }
        let (mut lo, mut hi) = ((f32::MAX, f32::MAX), (f32::MIN, f32::MIN));
        for p in tris.iter().flatten() {
            lo = (lo.0.min(p.x), lo.1.min(p.y));
            hi = (hi.0.max(p.x), hi.1.max(p.y));
        }
        let cell = 16.0;
        let dims = (((hi.0 - lo.0) / cell) as usize + 1, ((hi.1 - lo.1) / cell) as usize + 1);
        let mut cells = vec![Vec::new(); dims.0 * dims.1];
        for (i, t) in tris.iter().enumerate() {
            let (x0, x1) = (t.iter().map(|p| p.x).fold(f32::MAX, f32::min), t.iter().map(|p| p.x).fold(f32::MIN, f32::max));
            let (y0, y1) = (t.iter().map(|p| p.y).fold(f32::MAX, f32::min), t.iter().map(|p| p.y).fold(f32::MIN, f32::max));
            for cy in ((y0 - lo.1) / cell) as usize..=((y1 - lo.1) / cell) as usize {
                for cx in ((x0 - lo.0) / cell) as usize..=((x1 - lo.0) / cell) as usize {
                    cells[cy * dims.0 + cx].push(i as u32);
                }
            }
        }
        Ground { scene: crate::world::Scene::new(&[]), roads: Default::default(), cell, min: lo, dims, cells, tris }
    }

    pub fn triangle_count(&self) -> usize {
        self.tris.len()
    }

    /// Highest surface under `(x, y)` that is not more than `step_up` above `z_ref`.
    pub fn height_at(&self, x: f32, y: f32, z_ref: f32, step_up: f32) -> Option<f32> {
        let (cx, cy) = (((x - self.min.0) / self.cell).floor(), ((y - self.min.1) / self.cell).floor());
        if cx < 0.0 || cy < 0.0 || cx as usize >= self.dims.0 || cy as usize >= self.dims.1 {
            return None;
        }
        let mut best: Option<f32> = None;
        for &i in &self.cells[cy as usize * self.dims.0 + cx as usize] {
            let [a, b, c] = self.tris[i as usize];
            let d = (b.y - c.y) * (a.x - c.x) + (c.x - b.x) * (a.y - c.y);
            if d.abs() < 1e-9 {
                continue;
            }
            let u = ((b.y - c.y) * (x - c.x) + (c.x - b.x) * (y - c.y)) / d;
            let v = ((c.y - a.y) * (x - c.x) + (a.x - c.x) * (y - c.y)) / d;
            let w = 1.0 - u - v;
            if u < 0.0 || v < 0.0 || w < 0.0 {
                continue;
            }
            let z = u * a.z + v * b.z + w * c.z;
            if z <= z_ref + step_up && best.is_none_or(|b| z > b) {
                best = Some(z);
            }
        }
        best
    }

    /// First crossing of the segment `a`..`b` with the surface: the point and the surface normal
    /// (facing the side `a` is on).
    pub fn segment_hit(&self, a: Vec3, b: Vec3) -> Option<(Vec3, Vec3)> {
        let (x0, x1) = (a.x.min(b.x), a.x.max(b.x));
        let (y0, y1) = (a.y.min(b.y), a.y.max(b.y));
        let cell = |v: f32, lo: f32, n: usize| (((v - lo) / self.cell).floor().max(0.0) as usize).min(n.saturating_sub(1));
        if x1 < self.min.0 || y1 < self.min.1 || self.dims.0 == 0 {
            return None;
        }
        let (cx0, cx1) = (cell(x0, self.min.0, self.dims.0), cell(x1, self.min.0, self.dims.0));
        let (cy0, cy1) = (cell(y0, self.min.1, self.dims.1), cell(y1, self.min.1, self.dims.1));
        if (cx1 - cx0 + 1) * (cy1 - cy0 + 1) > 4096 {
            return None;
        }
        let d = b - a;
        let mut best: Option<(f32, Vec3)> = None;
        for cy in cy0..=cy1 {
            for cx in cx0..=cx1 {
                for &i in &self.cells[cy * self.dims.0 + cx] {
                    let [p0, p1, p2] = self.tris[i as usize];
                    let (e1, e2) = (p1 - p0, p2 - p0);
                    let h = d.cross(e2);
                    let det = e1.dot(h);
                    if det.abs() < 1e-12 {
                        continue;
                    }
                    let s = a - p0;
                    let u = s.dot(h) / det;
                    if !(0.0..=1.0).contains(&u) {
                        continue;
                    }
                    let q = s.cross(e1);
                    let v = d.dot(q) / det;
                    if v < 0.0 || u + v > 1.0 {
                        continue;
                    }
                    let t = e2.dot(q) / det;
                    if (0.0..=1.0).contains(&t) && best.is_none_or(|(bt, _)| t < bt) {
                        let mut n = e1.cross(e2).normalize();
                        if n.dot(d) > 0.0 {
                            n = -n;
                        }
                        best = Some((t, n));
                    }
                }
            }
        }
        best.map(|(t, n)| (a + d * t, n))
    }

    /// Heading of the road triangle closest to `pos`, or 0 when there are no roads.
    pub fn road_heading_near(scenes: &[Scene], pos: Vec3) -> f32 {
        road_triangles(scenes, "road")
            .map(|t| {
                let c = (t[0] + t[1] + t[2]) / 3.0;
                ((c.x - pos.x).hypot(c.y - pos.y), t)
            })
            .min_by(|a, b| a.0.total_cmp(&b.0))
            .map_or(0.0, |(_, t)| heading_of(longest_edge(&t)))
    }

    /// A point in the middle of the road surface and its heading, as a fallback spawn position.
    pub fn spawn_on_road(scenes: &[Scene]) -> Option<(Vec3, f32)> {
        let tris: Vec<_> = road_triangles(scenes, "road").collect();
        let t = tris.get(tris.len() / 2)?;
        Some(((t[0] + t[1] + t[2]) / 3.0, heading_of(longest_edge(t))))
    }
}

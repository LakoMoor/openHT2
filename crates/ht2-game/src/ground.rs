//! Drivable surface built from the world triangles, with a 2D grid for fast height queries.

use glam::Vec3;
use ht2_render::Scene;

/// Materials that are never driven on.
const NOT_GROUND: [&str; 9] = ["water", "shadow", "glass", "sky", "fog", "sun", "light", "glow", "tree"];

pub struct Ground {
    cell: f32,
    min: (f32, f32),
    dims: (usize, usize),
    cells: Vec<Vec<u32>>,
    tris: Vec<[Vec3; 3]>,
}

impl Ground {
    pub fn new(scene: &Scene) -> Self {
        let mut tris = Vec::new();
        for b in &scene.batches {
            let name = scene.material_names.get(b.material as usize).map_or("", |s| s.as_str()).to_lowercase();
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
        let cell = 8.0;
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
        Ground { cell, min: lo, dims, cells, tris }
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

    /// A point on a road triangle and the direction of its longest edge, used as a spawn position.
    pub fn spawn_on(scene: &Scene, name_part: &str) -> Option<(Vec3, f32)> {
        let batch = scene.batches.iter().find(|b| {
            scene.material_names.get(b.material as usize).is_some_and(|n| n.to_lowercase().contains(name_part))
        })?;
        let tris: Vec<_> = batch.verts.chunks_exact(3).collect();
        let t = tris.get(tris.len() / 2)?;
        let p: Vec<Vec3> = t.iter().map(|v| Vec3::from(v.pos)).collect();
        let centre = (p[0] + p[1] + p[2]) / 3.0;
        let edge = [(p[1] - p[0]), (p[2] - p[1]), (p[0] - p[2])].into_iter().max_by(|a, b| a.length().total_cmp(&b.length()))?;
        // heading 0 faces +Y; forward is (-sin h, cos h).
        Some((centre, (-edge.x).atan2(edge.y)))
    }
}

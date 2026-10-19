//! The collision blocks of the world (`.b3d` types 12 and 23: planes and polygon meshes) as the
//! original's volume records (`ht2_sim::shape`) in the vehicle's memory, with a 2D grid to find
//! the ones a query segment can reach.
//!
//! The original walks the scene graph from the vehicle's room (`0x4e6059`: the room's road
//! collision, then the neighbour room the segment leads into); that walk and the split nodes
//! (type 9) are not ported yet. Here every road collision block whose area the segment touches
//! is tested with the ported segment test, and the nearest hit wins, as between the children of
//! one node (`0x5f21e0`). A plane counts only inside the area of its room's polygons.

use glam::Vec3;
use ht2_formats::b3d::{CollisionItem, CollisionShape, Room};
use ht2_sim::mem::Mem;
use ht2_sim::shape;

/// Where the volume records, polygon records, vertices and surface records are put.
pub const BASE: u32 = 0x4000_0000;

struct Item {
    lo: (f32, f32),
    hi: (f32, f32),
    vol: u32,
    surface: u32,
}

pub struct Collision {
    cell: f32,
    min: (f32, f32),
    dims: (usize, usize),
    cells: Vec<Vec<u32>>,
    items: Vec<Item>,
    /// Records to write: (address, words).
    image: Vec<(u32, Vec<u32>)>,
    /// Polygon records to initialise: (record, vertices, count).
    polys: Vec<(u32, u32, u16)>,
}

impl Collision {
    pub fn new(rooms: &[Room]) -> Self {
        let mut next = BASE;
        let mut alloc = |n: u32| {
            let a = next;
            next += (n + 15) & !15;
            a
        };
        let mut items = Vec::new();
        let mut image = Vec::new();
        let mut polys = Vec::new();
        let surface_of = |it: &CollisionItem, image: &mut Vec<(u32, Vec<u32>)>, alloc: &mut dyn FnMut(u32) -> u32| {
            let rec = alloc(16);
            let params = if it.params.is_empty() {
                0
            } else {
                let p = alloc(4 * it.params.len() as u32);
                image.push((p, it.params.clone()));
                p
            };
            image.push((rec, vec![it.surface[0], it.surface[1], it.params.len() as u32, params]));
            rec
        };
        for room in rooms {
            let mut rlo = (f32::MAX, f32::MAX);
            let mut rhi = (f32::MIN, f32::MIN);
            for it in &room.road {
                if let CollisionShape::Polys(ps) = &it.shape {
                    for v in ps.iter().flatten() {
                        rlo = (rlo.0.min(v[0]), rlo.1.min(v[1]));
                        rhi = (rhi.0.max(v[0]), rhi.1.max(v[1]));
                    }
                }
            }
            for it in &room.road {
                match &it.shape {
                    CollisionShape::Polys(ps) => {
                        let ps: Vec<&Vec<[f32; 3]>> = ps.iter().filter(|p| p.len() >= 3).collect();
                        if ps.is_empty() {
                            continue;
                        }
                        let surface = surface_of(it, &mut image, &mut alloc);
                        let recs = alloc(shape::POLY_SIZE * ps.len() as u32);
                        let (mut lo, mut hi) = ((f32::MAX, f32::MAX), (f32::MIN, f32::MIN));
                        for (k, p) in ps.iter().enumerate() {
                            let verts = alloc(12 * p.len() as u32);
                            image.push((verts, p.iter().flat_map(|v| v.map(f32::to_bits)).collect()));
                            polys.push((recs + shape::POLY_SIZE * k as u32, verts, p.len() as u16));
                            for v in p.iter() {
                                lo = (lo.0.min(v[0]), lo.1.min(v[1]));
                                hi = (hi.0.max(v[0]), hi.1.max(v[1]));
                            }
                        }
                        let vol = alloc(12);
                        image.push((vol, vec![shape::VOL_POLYS, ps.len() as u32, recs]));
                        items.push(Item { lo, hi, vol, surface });
                    }
                    CollisionShape::Plane(pl) => {
                        if rlo.0 > rhi.0 {
                            continue; // a room without polygons: no area to put the plane in
                        }
                        let surface = surface_of(it, &mut image, &mut alloc);
                        let data = alloc(16);
                        image.push((data, pl.map(f32::to_bits).to_vec()));
                        let vol = alloc(12);
                        image.push((vol, vec![shape::VOL_PLANE, 0, data]));
                        items.push(Item { lo: rlo, hi: rhi, vol, surface });
                    }
                }
            }
        }
        let (mut lo, mut hi) = ((f32::MAX, f32::MAX), (f32::MIN, f32::MIN));
        for it in &items {
            lo = (lo.0.min(it.lo.0), lo.1.min(it.lo.1));
            hi = (hi.0.max(it.hi.0), hi.1.max(it.hi.1));
        }
        let cell = 32.0;
        let dims = if items.is_empty() { (0, 0) } else { (((hi.0 - lo.0) / cell) as usize + 1, ((hi.1 - lo.1) / cell) as usize + 1) };
        let mut cells = vec![Vec::new(); dims.0 * dims.1];
        for (i, it) in items.iter().enumerate() {
            for cy in ((it.lo.1 - lo.1) / cell) as usize..=((it.hi.1 - lo.1) / cell) as usize {
                for cx in ((it.lo.0 - lo.0) / cell) as usize..=((it.hi.0 - lo.0) / cell) as usize {
                    cells[cy * dims.0 + cx].push(i as u32);
                }
            }
        }
        Collision { cell, min: lo, dims, cells, items, image, polys }
    }

    pub fn item_count(&self) -> usize {
        self.items.len()
    }

    /// Write the records into the vehicle's memory.
    pub fn install(&self, m: &mut Mem) {
        for (a, words) in &self.image {
            for (i, w) in words.iter().enumerate() {
                m.set_u32(a + 4 * i as u32, *w);
            }
        }
        for (rec, verts, n) in &self.polys {
            shape::poly_init(m, *rec, *verts, *n);
        }
    }

    /// The query of the scene (`vtable + 0x34`, `mode`) for the segment of the record `rec`;
    /// fills in the result like a collision block does (`0x5f93d0`) and returns 1 on a hit.
    pub fn query(&self, m: &mut Mem, mode: u32, rec: u32, scratch: u32) -> u32 {
        if self.dims.0 == 0 || mode & 4 == 0 {
            return 0; // collision blocks answer only queries with bit 4 (`0x5f93d0`)
        }
        let v = |m: &Mem, a: u32| Vec3::new(m.f32(a), m.f32(a + 4), m.f32(a + 8));
        let (a, b) = (v(m, rec + 0x4c), v(m, rec + 0x58));
        let (x0, x1, y0, y1) = (a.x.min(b.x), a.x.max(b.x), a.y.min(b.y), a.y.max(b.y));
        let cell = |x: f32, lo: f32, n: usize| (((x - lo) / self.cell).floor().max(0.0) as usize).min(n - 1);
        let hi = (self.min.0 + self.cell * self.dims.0 as f32, self.min.1 + self.cell * self.dims.1 as f32);
        if x1 < self.min.0 || y1 < self.min.1 || x0 > hi.0 || y0 > hi.1 {
            return 0;
        }
        let (cx0, cx1) = (cell(x0, self.min.0, self.dims.0), cell(x1, self.min.0, self.dims.0));
        let (cy0, cy1) = (cell(y0, self.min.1, self.dims.1), cell(y1, self.min.1, self.dims.1));
        if (cx1 - cx0 + 1) * (cy1 - cy0 + 1) > 1024 {
            return 0;
        }
        let mut seen = Vec::new();
        let mut best: Option<(f32, [u32; 6], u32, u32)> = None;
        let (s, e) = (scratch, scratch + 12);
        for cy in cy0..=cy1 {
            for cx in cx0..=cx1 {
                for &i in &self.cells[cy * self.dims.0 + cx] {
                    if seen.contains(&i) {
                        continue;
                    }
                    seen.push(i);
                    let it = &self.items[i as usize];
                    if it.hi.0 < x0 || it.lo.0 > x1 || it.hi.1 < y0 || it.lo.1 > y1 {
                        continue;
                    }
                    for k in 0..3 {
                        let (ws, we) = (m.u32(rec + 0x4c + 4 * k), m.u32(rec + 0x58 + 4 * k));
                        m.set_u32(s + 4 * k, ws);
                        m.set_u32(e + 4 * k, we);
                    }
                    let hit = shape::fn_005f6820(m, it.vol, mode | 1, s, e, 0.0);
                    if hit == 0 {
                        continue;
                    }
                    let p = v(m, s);
                    let d = (p - a).length();
                    if best.is_none_or(|(bd, ..)| d < bd) {
                        let words: [u32; 6] = core::array::from_fn(|k| m.u32(s + 4 * k as u32));
                        best = Some((d, words, i, hit));
                    }
                }
            }
        }
        let Some((_, words, i, hit)) = best else { return 0 };
        let it = &self.items[i as usize];
        for (k, w) in words.iter().enumerate() {
            m.set_u32(rec + 0x34 + 4 * k as u32, *w);
        }
        m.set_u32(rec + 0x64, it.surface);
        m.set_u32(rec + 0x6c, it.vol);
        m.set_u32(rec + 0x70, hit);
        1
    }
}

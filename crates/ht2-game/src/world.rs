//! The original's scene collision (`vtable + 0x34` of the scene nodes) over the blocks of the
//! world's `.b3d` files, and the room queries built on it (`0x4e6059`, `0x4e623e`).
//!
//! Node classes, as in the original:
//! - groups and rooms (`0x5fccd0`): the first child of list 0 that reports something
//!   (`0x5f21e0`; with mode bit 8 the nearest hit instead);
//! - references (type 5 / 37 / 7 naming another block, `0x5f38b0`): without mode bit 1 and with
//!   bit 4 the named block is asked with bit 1 added, otherwise the own children;
//! - splits (type 9, `0x5f8ff0`): the plane decides which of the two child lists to search;
//! - collision blocks (types 12, 13, 23, `0x5f93d0`): answer only with mode bits 1 and 4, through
//!   the ported segment test (`ht2_sim::shape`), filling in hit point, normal and surface record;
//! - portals (type 30, `0x5fc2c0`): a segment crossing the portal rectangle along its normal
//!   continues in the room behind it and reports bit 2 with that room in `rec + 0x68`.
//!
//! - links (type 18, `0x5f9b00`): the linked group in the space of the link; the room queries take
//!   the hit back to world space (`0x4e5698`).
//!
//! Not ported yet: the sphere sweeps (mode bit 2), the volumes of types 14..17 and the walls
//! (type 20); they report nothing. Spaces inside spaces are taken as absolute.

use ht2_formats::b3d::{self, Data, Node};
use ht2_sim::contact::World;
use ht2_sim::mem::Mem;
use ht2_sim::shape;
use std::collections::HashMap;

/// Where the volume records, polygon records, vertices and surface records are put.
const BASE: u32 = 0x4000_0000;
/// Scratch for the segment copies of the node tests.
const SCRATCH: u32 = 0x3ff0_0000;

enum Op {
    Group,
    Ref(Option<u32>),
    Split { vol: u32 },
    Volume { vol: u32, surface: u32 },
    Portal { n: [f32; 3], c0: [f32; 3], c2: [f32; 3], target: Option<u32> },
    /// Type 18: a group placed by a space; `inv` is the inverse of the space's transform.
    Link { target: Option<u32>, inv: Option<[f32; 12]> },
    Nothing,
}

struct SNode {
    name: String,
    lists: Vec<Vec<u32>>,
    op: Op,
}

pub struct Scene {
    nodes: Vec<SNode>,
    rooms: HashMap<String, u32>,
    image: Vec<(u32, Vec<u32>)>,
    polys: Vec<(u32, u32, u16)>,
}

struct Builder {
    next: u32,
    image: Vec<(u32, Vec<u32>)>,
    polys: Vec<(u32, u32, u16)>,
}

impl Builder {
    fn alloc(&mut self, n: u32) -> u32 {
        let a = self.next;
        self.next += (n + 15) & !15;
        a
    }

    fn surface(&mut self, surface: [u32; 2], params: &[u32]) -> u32 {
        let rec = self.alloc(16);
        let p = if params.is_empty() {
            0
        } else {
            let p = self.alloc(4 * params.len() as u32);
            self.image.push((p, params.to_vec()));
            p
        };
        self.image.push((rec, vec![surface[0], surface[1], params.len() as u32, p]));
        rec
    }

    fn plane_vol(&mut self, kind: u32, plane: [f32; 4]) -> u32 {
        let data = self.alloc(16);
        self.image.push((data, plane.map(f32::to_bits).to_vec()));
        let vol = self.alloc(12);
        self.image.push((vol, vec![kind, 0, data]));
        vol
    }
}

fn v3(m: &Mem, a: u32) -> [f32; 3] {
    [m.f32(a), m.f32(a + 4), m.f32(a + 8)]
}

fn set3(m: &mut Mem, a: u32, v: [f32; 3]) {
    for (i, x) in v.iter().enumerate() {
        m.set_f32(a + 4 * i as u32, *x);
    }
}

/// The segment end `p` taken into the space of the query record's transform.
fn to_node(m: &Mem, rec: u32, p: [f32; 3]) -> [f32; 3] {
    let t = |i: u32| m.f32(rec + 4 + 4 * i) as f64;
    let (x, y, z) = (p[0] as f64, p[1] as f64, p[2] as f64);
    [
        (x * t(0) + z * t(6) + y * t(3) + t(9)) as f32,
        (z * t(7) + x * t(1) + y * t(4) + t(10)) as f32,
        (z * t(8) + x * t(2) + y * t(5) + t(11)) as f32,
    ]
}

/// `0x5f88d0` for a plane volume: is `p` strictly on the front side?
fn plane_inside(m: &Mem, vol: u32, p: [f32; 3]) -> bool {
    let pl = m.u32(vol + 8);
    let v = p[0] * m.f32(pl) + m.f32(pl + 4) * p[1] + m.f32(pl + 8) * p[2] + m.f32(pl + 12);
    v > 0.0
}

impl Scene {
    pub fn new(files: &[b3d::B3d]) -> Self {
        let mut b = Builder { next: BASE, image: Vec::new(), polys: Vec::new() };
        let mut nodes: Vec<SNode> = Vec::new();
        let mut rooms = HashMap::new();
        // references and portals are resolved after all nodes exist
        let mut refs: Vec<(u32, String, usize)> = Vec::new();
        let mut portals: Vec<(u32, String)> = Vec::new();
        let mut file_names: Vec<HashMap<String, u32>> = Vec::new();
        let mut file_spaces: Vec<HashMap<String, [f32; 12]>> = Vec::new();
        let mut links: Vec<(u32, String, String, usize)> = Vec::new();
        for (fi, f) in files.iter().enumerate() {
            let mut names = HashMap::new();
            let mut spaces = HashMap::new();
            #[allow(clippy::too_many_arguments)]
            fn add(n: &Node, fi: usize, nodes: &mut Vec<SNode>, b: &mut Builder, names: &mut HashMap<String, u32>, spaces: &mut HashMap<String, [f32; 12]>, rooms: &mut HashMap<String, u32>, refs: &mut Vec<(u32, String, usize)>, links: &mut Vec<(u32, String, String, usize)>, portals: &mut Vec<(u32, String)>) -> u32 {
                let id = nodes.len() as u32;
                nodes.push(SNode { name: n.name.clone(), lists: Vec::new(), op: Op::Nothing });
                if let Data::Space { axes, pos } = &n.data {
                    let mut t = [0.0f32; 12];
                    for (i, a) in axes.iter().enumerate() {
                        t[3 * i..3 * i + 3].copy_from_slice(a);
                    }
                    t[9..].copy_from_slice(pos);
                    spaces.entry(n.name.to_ascii_lowercase()).or_insert(t);
                } else if !n.name.is_empty() {
                    names.entry(n.name.to_ascii_lowercase()).or_insert(id);
                }
                let op = match (&n.data, n.kind) {
                    (Data::Collision { surface, params, polys }, _) => {
                        let ps: Vec<&Vec<[f32; 3]>> = polys.iter().filter(|p| p.len() >= 3).collect();
                        if ps.is_empty() {
                            Op::Nothing
                        } else {
                            let surface = b.surface(*surface, params);
                            let recs = b.alloc(shape::POLY_SIZE * ps.len() as u32);
                            for (k, p) in ps.iter().enumerate() {
                                let verts = b.alloc(12 * p.len() as u32);
                                b.image.push((verts, p.iter().flat_map(|v| v.map(f32::to_bits)).collect()));
                                b.polys.push((recs + shape::POLY_SIZE * k as u32, verts, p.len() as u16));
                            }
                            let vol = b.alloc(12);
                            b.image.push((vol, vec![shape::VOL_POLYS, ps.len() as u32, recs]));
                            Op::Volume { vol, surface }
                        }
                    }
                    (Data::CollisionPlane { plane, surface, params, .. }, _) => {
                        let surface = b.surface(*surface, params);
                        Op::Volume { vol: b.plane_vol(shape::VOL_PLANE, *plane), surface }
                    }
                    (Data::Split { plane }, _) => Op::Split { vol: b.plane_vol(shape::VOL_PLANE, *plane) },
                    (Data::Portal { a, b: q }, _) => {
                        // the rectangle between the two corners; the normal is horizontal,
                        // up x (corner 1 - corner 0) (vertical portals; see 0x5fc360)
                        let d = [q[0] - a[0], q[1] - a[1]];
                        let len = (d[0] * d[0] + d[1] * d[1]).sqrt();
                        let nv = if len < 1e-5 { [0.0, 0.0, 0.0] } else { [-d[1] / len, d[0] / len, 0.0] };
                        Op::Portal { n: nv, c0: *a, c2: *q, target: None }
                    }
                    (_, 5 | 7 | 37) if n.names.first().is_some_and(|s| !s.is_empty()) => {
                        refs.push((id, n.names[0].to_ascii_lowercase(), fi));
                        Op::Ref(None)
                    }
                    (_, 18) if n.names.len() == 2 => {
                        links.push((id, n.names[0].to_ascii_lowercase(), n.names[1].to_ascii_lowercase(), fi));
                        Op::Link { target: None, inv: None }
                    }
                    (_, 13) => Op::Nothing,
                    (_, 18 | 14 | 15 | 16 | 17 | 20) => Op::Nothing,
                    _ => Op::Group,
                };
                if n.kind == 30 {
                    if let Some(to) = n.names.first() {
                        portals.push((id, to.clone()));
                    }
                }
                if n.kind == 19 {
                    rooms.insert(n.name.to_ascii_lowercase(), id);
                }
                let mut lists: Vec<Vec<u32>> = Vec::new();
                for c in &n.children {
                    let cid = add(c, fi, nodes, b, names, spaces, rooms, refs, links, portals);
                    let l = c.list as usize;
                    if lists.len() <= l {
                        lists.resize(l + 1, Vec::new());
                    }
                    lists[l].push(cid);
                }
                nodes[id as usize].lists = lists;
                nodes[id as usize].op = op;
                id
            }
            for n in &f.nodes {
                add(n, fi, &mut nodes, &mut b, &mut names, &mut spaces, &mut rooms, &mut refs, &mut links, &mut portals);
            }
            file_names.push(names);
            file_spaces.push(spaces);
        }
        for (id, name, fi) in refs {
            let target = file_names[fi].get(&name).copied().filter(|t| *t != id).or_else(|| file_names.iter().find_map(|m| m.get(&name).copied().filter(|t| *t != id)));
            nodes[id as usize].op = Op::Ref(target);
        }
        let mut scratch = Mem::new();
        for (id, space, group, fi) in links {
            let target = file_names[fi].get(&group).copied().filter(|t| *t != id);
            let inv = file_spaces[fi].get(&space).map(|t| {
                for (i, v) in t.iter().enumerate() {
                    scratch.set_f32(4 * i as u32, *v);
                }
                ht2_sim::math::mat_inverse(&mut scratch, 0x100, 0);
                core::array::from_fn(|i| scratch.f32(0x100 + 4 * i as u32))
            });
            nodes[id as usize].op = Op::Link { target, inv };
        }
        for (id, to) in portals {
            // a portal into another file names it as `file:room`
            let room = to.rsplit(':').next().unwrap_or(&to).to_ascii_lowercase();
            if let Op::Portal { target, .. } = &mut nodes[id as usize].op {
                *target = rooms.get(&room).copied();
            }
        }
        Scene { nodes, rooms, image: b.image, polys: b.polys }
    }

    pub fn room_count(&self) -> usize {
        self.rooms.len()
    }

    /// The id the vehicle keeps for a room (`+0xf4`), never 0.
    pub fn room_id(node: u32) -> u32 {
        node + 1
    }

    /// Write the volume records into the vehicle's memory.
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

    /// `0x5ef2b0`: the first block below `node` called `name`.
    fn find(&self, node: u32, name: &str) -> Option<u32> {
        for list in &self.nodes[node as usize].lists {
            for &c in list {
                if self.nodes[c as usize].name.eq_ignore_ascii_case(name) {
                    return Some(c);
                }
                if let Some(x) = self.find(c, name) {
                    return Some(x);
                }
            }
        }
        None
    }

    /// The child of a room named like the room with its first four letters replaced (`road`, `obj`).
    fn room_part(&self, room: u32, prefix: &str) -> Option<u32> {
        let name = &self.nodes[room as usize].name;
        let suffix = name.get(4..)?;
        self.find(room, &format!("{prefix}{suffix}"))
    }

    /// `0x5f21e0`: the children of list `list`.
    fn traverse(&self, m: &mut Mem, node: u32, list: usize, mode: u32, rec: u32) -> u32 {
        let Some(children) = self.nodes[node as usize].lists.get(list) else { return 0 };
        if mode & 8 == 0 {
            for &c in children {
                let r = self.collide(m, c, mode, rec);
                if r != 0 {
                    return r;
                }
            }
            return 0;
        }
        // nearest hit
        let saved = m.read(rec, 0xd0);
        let mut best = saved.clone();
        let mut best_d = 1e6f64;
        let mut flags = 0;
        let mut have = false;
        let start = v3(m, rec + 0x4c);
        for &c in children {
            let r = self.collide(m, c, mode, rec);
            if r & 1 != 0 {
                let h = v3(m, rec + 0x34);
                let d = ((start[0] - h[0]) as f64).hypot((start[1] - h[1]) as f64).hypot((start[2] - h[2]) as f64);
                if !have || d < best_d {
                    best_d = d;
                    best = m.read(rec, 0xd0);
                    flags = r;
                    have = true;
                }
            }
            if r & 2 != 0 {
                best = m.read(rec, 0xd0);
                flags = r;
            }
            m.write(rec, &saved);
        }
        m.write(rec, &best);
        flags
    }

    /// The collision method of one block.
    fn collide(&self, m: &mut Mem, node: u32, mode: u32, rec: u32) -> u32 {
        let n = &self.nodes[node as usize];
        match &n.op {
            Op::Nothing => 0,
            Op::Group => self.traverse(m, node, 0, mode, rec),
            Op::Ref(target) => {
                if mode & 1 != 0 {
                    return self.traverse(m, node, 0, mode, rec);
                }
                match target {
                    Some(t) if mode & 4 != 0 => self.collide(m, *t, mode | 1, rec),
                    _ => self.traverse(m, node, 0, mode, rec),
                }
            }
            Op::Split { vol } => {
                let vol = *vol;
                let s0 = to_node(m, rec, v3(m, rec + 0x4c));
                let e0 = to_node(m, rec, v3(m, rec + 0x58));
                let rad = m.f32(rec);
                let (a, b) = (SCRATCH, SCRATCH + 12);
                set3(m, a, s0);
                set3(m, b, e0);
                let mut kind = 0;
                if shape::fn_005f6820(m, vol, mode, a, b, rad) == 0 && shape::fn_005f6820(m, vol, mode, b, a, rad) == 0 {
                    if plane_inside(m, vol, s0) {
                        kind = 3;
                    }
                } else {
                    kind = if plane_inside(m, vol, s0) { 1 } else { 2 };
                }
                match kind {
                    0 => self.traverse(m, node, 1, mode, rec),
                    3 => self.traverse(m, node, 0, mode, rec),
                    1 => {
                        let r = self.traverse(m, node, 0, mode, rec);
                        if r != 0 && plane_inside(m, vol, v3(m, rec + 0x34)) {
                            return r;
                        }
                        let r = self.traverse(m, node, 1, mode, rec);
                        if r != 0 && !plane_inside(m, vol, v3(m, rec + 0x34)) {
                            return r;
                        }
                        0
                    }
                    _ => {
                        let r = self.traverse(m, node, 1, mode, rec);
                        if r != 0 && !plane_inside(m, vol, v3(m, rec + 0x34)) {
                            return r;
                        }
                        let r = self.traverse(m, node, 0, mode, rec);
                        if r != 0 && plane_inside(m, vol, v3(m, rec + 0x34)) {
                            return r;
                        }
                        0
                    }
                }
            }
            Op::Volume { vol, surface } => {
                if mode & 1 == 0 || mode & 4 == 0 {
                    return 0;
                }
                let s0 = to_node(m, rec, v3(m, rec + 0x4c));
                let e0 = to_node(m, rec, v3(m, rec + 0x58));
                let (a, b) = (SCRATCH, SCRATCH + 12);
                set3(m, a, s0);
                set3(m, b, e0);
                let rad = m.f32(rec);
                let hit = shape::fn_005f6820(m, *vol, mode, a, b, rad);
                if hit == 0 {
                    return 0;
                }
                m.set_u32(rec + 0x6c, *vol);
                for k in 0..6 {
                    let w = m.u32(SCRATCH + 4 * k);
                    m.set_u32(rec + 0x34 + 4 * k, w);
                }
                m.set_u32(rec + 0x64, *surface);
                m.set_u32(rec + 0x70, hit);
                1
            }
            Op::Link { target, inv } => {
                // 0x5f9b00: the query continues in the space of the link (the query transform is
                // replaced by the inverse of the space's); it is kept on a hit for 0x4e5698
                let Some(t) = target else { return 0 };
                if mode & 4 == 0 {
                    return 0;
                }
                let saved = m.read(rec + 4, 48);
                if let Some(inv) = inv {
                    for (i, v) in inv.iter().enumerate() {
                        m.set_f32(rec + 4 + 4 * i as u32, *v);
                    }
                }
                let r = self.collide(m, *t, mode, rec);
                if r == 0 {
                    m.write(rec + 4, &saved);
                }
                r
            }
            Op::Portal { n: nv, c0, c2, target } => {
                let s0 = to_node(m, rec, v3(m, rec + 0x4c));
                let e0 = to_node(m, rec, v3(m, rec + 0x58));
                let d = [e0[0] - s0[0], e0[1] - s0[1], e0[2] - s0[2]];
                if (d[0] as f64).hypot(d[1] as f64).hypot(d[2] as f64) < 1e-5 {
                    return 0;
                }
                let dn = d[0] * nv[0] + d[1] * nv[1] + d[2] * nv[2];
                if !(0.0 <= dn && 1e-5 < dn as f64) {
                    return 0;
                }
                let t = -((nv[0] * s0[0] + nv[1] * s0[1] + nv[2] * s0[2]) - (c0[0] * nv[0] + c0[1] * nv[1] + c0[2] * nv[2])) / dn;
                if !((-1e-5f64 as f32) < t && t < 1.00001f64 as f32) {
                    return 0;
                }
                let p = [s0[0] + d[0] * t, s0[1] + d[1] * t, s0[2] + d[2] * t];
                let within = |x: f32, a: f32, b: f32| {
                    let tol = (b - a).abs() + 0.001;
                    (x - b).abs() <= tol && (x - a).abs() <= tol
                };
                if !(within(p[0], c0[0], c2[0]) && within(p[1], c0[1], c2[1]) && within(p[2], c0[2], c2[2])) {
                    return 0;
                }
                match target {
                    None => 4,
                    Some(t) => {
                        let r = self.collide(m, *t, mode, rec);
                        if r & 2 == 0 {
                            m.set_u32(rec + 0x68, Self::room_id(*t));
                        }
                        r | 2
                    }
                }
            }
        }
    }

    /// `0x4e5698`: the hit point and normal back from the space of the query transform.
    fn hit_to_world(m: &mut Mem, rec: u32) {
        let inv = SCRATCH + 0x300;
        ht2_sim::math::mat_inverse(m, inv, rec + 4);
        let (p, n) = (SCRATCH + 0x340, SCRATCH + 0x350);
        ht2_sim::matrix::transform(m, p, rec + 0x34, inv);
        ht2_sim::matrix::rotate(m, n, rec + 0x40, inv);
        for k in 0..3 {
            let (a, b) = (m.u32(p + 4 * k), m.u32(n + 4 * k));
            m.set_u32(rec + 0x34 + 4 * k, a);
            m.set_u32(rec + 0x40 + 4 * k, b);
        }
    }

    /// `0x4e5f87`: the room the segment of `rec` leads into from `room`, or 0.
    fn crossing(&self, m: &mut Mem, room: u32, rec: u32) -> u32 {
        let tmp = SCRATCH + 0x100;
        ht2_sim::contact::rec_init(m, tmp);
        for i in 0..12u32 {
            let w = m.u32(ht2_sim::globals::MAT_IDENTITY + 4 * i);
            m.set_u32(tmp + 4 + 4 * i, w);
        }
        for i in 0..6u32 {
            let w = m.u32(rec + 0x4c + 4 * i);
            m.set_u32(tmp + 0x4c + 4 * i, w);
        }
        m.set_u32(tmp + 0x74, 0);
        m.set_u32(tmp + 0x68, 0);
        m.set_u32(tmp + 0x78, 0);
        let rad = m.f32(rec);
        m.set_f32(tmp, rad);
        let flags = self.collide(m, room, if rad == 0.0 { 0 } else { 2 }, tmp);
        if flags & 2 != 2 {
            return 0;
        }
        m.u32(tmp + 0x68)
    }

    /// The room node for an id kept by the vehicle.
    fn room_node(&self, id: u32) -> Option<u32> {
        id.checked_sub(1).filter(|n| (*n as usize) < self.nodes.len())
    }

    /// `0x4e6059`: the road of `room` (its `dynamical` part first), then of the room the segment
    /// leads into.
    fn road_query(&self, m: &mut Mem, room: u32, mode: u32, rec: u32, neighbour: bool) -> u32 {
        let saved = m.read(rec, 0xd0);
        if let Some(road) = self.room_part(room, "road") {
            if let Some(dynamic) = self.find(road, "dynamical") {
                let r = self.collide(m, dynamic, mode, rec);
                if r & 1 != 0 {
                    return r;
                }
                m.write(rec, &saved);
            }
            let r = self.collide(m, road, mode, rec);
            if r & 1 != 0 {
                return r;
            }
        }
        if !neighbour {
            return 0;
        }
        let next = self.crossing(m, room, rec);
        let Some(nr) = self.room_node(next).filter(|_| next != 0) else { return 0 };
        let Some(road) = self.room_part(nr, "road") else { return 0 };
        if let Some(dynamic) = self.find(road, "dynamical") {
            let r = self.collide(m, dynamic, mode, rec);
            m.set_u32(rec + 0x68, next);
            if r & 1 != 0 {
                return r | 2;
            }
            m.write(rec, &saved);
        }
        let r = self.collide(m, road, mode, rec);
        m.set_u32(rec + 0x68, next);
        r | 2
    }

    /// `0x4e623e`: the objects of `room`, then of the room the segment leads into.
    fn obj_query(&self, m: &mut Mem, room: u32, mode: u32, rec: u32) -> u32 {
        let saved = m.read(rec, 0xd0);
        if let Some(obj) = self.room_part(room, "obj") {
            let r = self.collide(m, obj, mode, rec);
            if r & 1 != 0 {
                return r;
            }
        }
        let next = self.crossing(m, room, rec);
        let Some(nr) = self.room_node(next).filter(|_| next != 0) else { return 0 };
        let Some(obj) = self.room_part(nr, "obj") else { return 0 };
        m.write(rec, &saved);
        let r = self.collide(m, obj, mode, rec);
        m.set_u32(rec + 0x68, next);
        r | 2
    }

    /// The room whose road is right below `p` (the vehicle is created in it).
    pub fn room_at(&self, m: &mut Mem, p: [f32; 3]) -> Option<u32> {
        let rec = SCRATCH + 0x200;
        let mut best: Option<(f32, u32)> = None;
        for &room in self.rooms.values() {
            ht2_sim::contact::rec_init(m, rec);
            for i in 0..12u32 {
                let w = m.u32(ht2_sim::globals::MAT_IDENTITY + 4 * i);
                m.set_u32(rec + 4 + 4 * i, w);
            }
            set3(m, rec + 0x4c, [p[0], p[1], p[2] + 2.0]);
            set3(m, rec + 0x58, [p[0], p[1], p[2] - 3.0]);
            if self.road_query(m, room, 4, rec, false) & 1 != 0 {
                let z = m.f32(rec + 0x3c);
                if best.is_none_or(|(bz, _)| z > bz) {
                    best = Some((z, room));
                }
            }
        }
        best.map(|(_, r)| Self::room_id(r))
    }
}

/// The world for the ported vehicle: the scene's rooms.
pub struct SceneWorld<'a> {
    pub scene: &'a Scene,
}

impl World for SceneWorld<'_> {
    fn collide(&mut self, m: &mut Mem, obj: u32, mode: u32, rec: u32) -> u32 {
        match self.scene.room_node(obj) {
            Some(n) => self.scene.collide(m, n, mode, rec),
            None => 0,
        }
    }

    fn room_collide(&mut self, m: &mut Mem, which: u32, room: u32, mode: u32, rec: u32) -> u32 {
        let Some(n) = self.scene.room_node(room) else { return 0 };
        let r = if which == 0 { self.scene.road_query(m, n, mode, rec, true) } else { self.scene.obj_query(m, n, mode, rec) };
        if r & 1 != 0 {
            Scene::hit_to_world(m, rec);
        }
        r
    }
}

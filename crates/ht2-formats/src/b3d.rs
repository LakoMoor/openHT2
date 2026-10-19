//! `.b3d` models and game-logic object trees.
//!
//! Layout (all little-endian; offsets/lengths in the header are in 32-bit words):
//! ```text
//! "b3d\0", len_file, ofc_materials, len_materials, ofc_nodes, len_nodes
//! u32 count, count * name[32]           -- material names, looked up in the .RES MATERIALS section
//! u32 111 | blocks... | u32 222         -- node section, len_nodes words long
//! ```
//! A block is `u32* (ends with 333)`, `name[32]`, `u32 type`, type-specific data, `u32* (ends with 555)`.
//! Types with children store `u32 count` followed by that many blocks.
//!
//! Structure cross-checked against the MIT-licensed reRNR/RNRFileFormats descriptions.

use crate::{Error, Result};
use std::collections::{HashMap, HashSet};

pub type Vec3 = [f32; 3];

#[derive(Debug, Clone, Copy)]
pub struct Vertex {
    pub pos: Vec3,
    pub uv: [f32; 2],
}

#[derive(Debug, Clone)]
pub struct PolyVert {
    /// Index into the nearest ancestor vertex list.
    pub index: u32,
    pub uv: Option<[f32; 2]>,
}

#[derive(Debug, Clone)]
pub struct Poly {
    pub material: u32,
    pub verts: Vec<PolyVert>,
}

#[derive(Debug)]
pub enum Data {
    /// Types 6, 7, 36, 37: a vertex pool used by polygons below it.
    Verts(Vec<Vertex>),
    /// Types 8, 35: polygons indexing the vertex pool.
    Polys(Vec<Poly>),
    /// Type 28: camera-facing sprite polygons with their own vertices.
    Sprites(Vec<(u32, Vec<Vertex>)>),
    /// Type 24: local transform (three axis vectors and an origin).
    Space { axes: [Vec3; 3], pos: Vec3 },
    /// Type 23: a collision mesh. `surface` and `params` are the surface record the original hands
    /// to the vehicle on contact (type, kind, then the parameter words); `polys` are its polygons.
    Collision { surface: [u32; 2], params: Vec<u32>, polys: Vec<Vec<Vec3>> },
    /// Type 12: a collision plane `(n, d)` with its bounding sphere and surface record.
    CollisionPlane { sphere: [f32; 4], plane: [f32; 4], surface: [u32; 2], params: Vec<u32> },
    /// Every other type; its data was consumed but is not retained.
    Other,
}

#[derive(Debug)]
pub struct Node {
    pub name: String,
    pub kind: u32,
    /// Every `name[32]` field of the block body (space names, group names, ...), in file order.
    pub names: Vec<String>,
    pub data: Data,
    pub children: Vec<Node>,
}

#[derive(Debug)]
pub struct B3d {
    pub materials: Vec<String>,
    pub nodes: Vec<Node>,
}

struct R<'a> {
    d: &'a [u8],
    o: usize,
}

impl R<'_> {
    fn take(&mut self, n: usize) -> Result<&[u8]> {
        let end = self.o.checked_add(n).filter(|&e| e <= self.d.len()).ok_or(Error::Eof(self.o))?;
        let s = &self.d[self.o..end];
        self.o = end;
        Ok(s)
    }
    fn u32(&mut self) -> Result<u32> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }
    fn f32(&mut self) -> Result<f32> {
        Ok(f32::from_bits(self.u32()?))
    }
    fn skip(&mut self, words: usize) -> Result<()> {
        self.take(words.checked_mul(4).ok_or(Error::Eof(self.o))?).map(|_| ())
    }
    fn name(&mut self) -> Result<String> {
        let b = self.take(32)?;
        let n = b.iter().position(|&c| c == 0).unwrap_or(32);
        Ok(b[..n].iter().map(|&c| c as char).collect())
    }
    fn names(&mut self, n: usize) -> Result<Vec<String>> {
        (0..n).map(|_| self.name()).collect()
    }
    fn vec3(&mut self) -> Result<Vec3> {
        Ok([self.f32()?, self.f32()?, self.f32()?])
    }
    fn uv(&mut self) -> Result<[f32; 2]> {
        Ok([self.f32()?, self.f32()?])
    }
    /// A `u32` count followed by `count` words that are skipped.
    fn counted_words(&mut self) -> Result<()> {
        let n = self.u32()? as usize;
        self.skip(n)
    }
    fn children(&mut self, depth: usize) -> Result<Vec<Node>> {
        let n = self.u32()? as usize;
        (0..n).map(|_| self.block(depth + 1)).collect()
    }

    fn polygons(&mut self, sprites: bool) -> Result<(Vec<Poly>, Vec<(u32, Vec<Vertex>)>)> {
        let n = self.u32()? as usize;
        let (mut polys, mut spr) = (Vec::new(), Vec::new());
        for _ in 0..n {
            let raw = self.u32()?;
            self.skip(1)?; // unknown f32
            self.skip(1)?; // unknown u32
            let material = self.u32()?;
            let count = self.u32()? as usize;
            if sprites {
                let uv_count = ((raw & 0xff00) >> 8) as usize + 1;
                let use_uv = raw & 2 != 0;
                let mut vs = Vec::with_capacity(count);
                for _ in 0..count {
                    self.skip(2)?; // scale u, v
                    let mut uv = [0.0; 2];
                    if use_uv {
                        uv = self.uv()?;
                        self.skip(2 * (uv_count - 1))?;
                    }
                    vs.push(Vertex { pos: [0.0; 3], uv });
                }
                spr.push((material, vs));
            } else {
                let f = raw ^ 1;
                let use_uv = f & 2 != 0;
                let use_normal = f & 0x20 != 0 && f & 0x10 != 0;
                let uv_count = ((f & 0xff00) >> 8) as usize + usize::from(use_uv);
                let mut verts = Vec::with_capacity(count);
                for _ in 0..count {
                    let index = self.u32()?;
                    let mut first = None;
                    if use_uv {
                        for _ in 0..uv_count {
                            let uv = self.uv()?;
                            first.get_or_insert(uv);
                        }
                    }
                    if use_normal {
                        self.skip(if f & 1 != 0 { 3 } else { 1 })?;
                    }
                    verts.push(PolyVert { index, uv: first });
                }
                polys.push(Poly { material, verts });
            }
        }
        Ok((polys, spr))
    }

    fn simple_verts(&mut self) -> Result<Vec<Vertex>> {
        let n = self.u32()? as usize;
        (0..n).map(|_| Ok(Vertex { pos: self.vec3()?, uv: self.uv()? })).collect()
    }

    fn complex_verts(&mut self, format_raw: u32) -> Result<Vec<Vertex>> {
        let fmt = format_raw & 0xff;
        let uv_count = (format_raw >> 8) as usize;
        let normal_words = if fmt == 1 || fmt == 2 { 3 } else { 1 };
        let n = self.u32()? as usize;
        let mut out = Vec::with_capacity(n);
        for _ in 0..n {
            let pos = self.vec3()?;
            let uv = self.uv()?;
            self.skip(2 * uv_count + normal_words)?;
            out.push(Vertex { pos, uv });
        }
        Ok(out)
    }

    fn block(&mut self, depth: usize) -> Result<Node> {
        if depth > 64 {
            return Err(Error::Other("block nesting too deep".into()));
        }
        while self.u32()? != 333 {}
        let start = self.o;
        let name = self.name()?;
        let kind = self.u32()?;
        self.block_body(depth, name, kind).map_err(|e| match e {
            Error::Eof(at) => Error::Other(format!("in block type {kind} starting at {start}: unexpected end at {at}")),
            e => e,
        })
    }

    fn block_body(&mut self, depth: usize, name: String, kind: u32) -> Result<Node> {
        let mut data = Data::Other;
        let mut children = Vec::new();
        let mut names = Vec::new();
        match kind {
            0 => self.skip(11)?,
            1 => names = self.names(2)?,
            2 | 9 | 10 | 11 | 22 => {
                self.skip(8)?;
                children = self.children(depth)?;
            }
            3 | 19 => {
                self.skip(if kind == 3 { 4 } else { 0 })?;
                children = self.children(depth)?;
            }
            4 => {
                self.skip(4)?;
                names = self.names(2)?;
                children = self.children(depth)?;
            }
            5 => {
                self.skip(4)?;
                names = self.names(1)?;
                children = self.children(depth)?;
            }
            6 | 7 => {
                self.skip(4)?;
                names = self.names(if kind == 6 { 2 } else { 1 })?;
                data = Data::Verts(self.simple_verts()?);
                children = self.children(depth)?;
            }
            8 => {
                self.skip(4)?;
                data = Data::Polys(self.polygons(false)?.0);
            }
            12 => {
                let sphere = [self.f32()?, self.f32()?, self.f32()?, self.f32()?];
                let plane = [self.f32()?, self.f32()?, self.f32()?, self.f32()?];
                let surface = [self.u32()?, self.u32()?];
                let k = self.u32()? as usize;
                let params = (0..k).map(|_| self.u32()).collect::<Result<Vec<_>>>()?;
                data = Data::CollisionPlane { sphere, plane, surface, params };
            }
            13 | 15 => {
                self.skip(4 + 2)?;
                self.counted_words()?;
            }
            14 => {
                self.skip(8 + 2)?;
                self.counted_words()?;
            }
            16 | 17 => {
                self.skip(4 + 6 + 2 + 2)?;
                self.counted_words()?;
            }
            18 => {
                self.skip(4)?;
                names = self.names(2)?;
            }
            20 => {
                self.skip(4)?;
                let coords = self.u32()? as usize;
                self.skip(2)?;
                self.counted_words()?;
                self.skip(3 * coords)?;
            }
            21 => {
                self.skip(4 + 2)?;
                children = self.children(depth)?;
            }
            23 => {
                let surface = [self.u32()?, self.u32()?];
                let k = self.u32()? as usize;
                let params = (0..k).map(|_| self.u32()).collect::<Result<Vec<_>>>()?;
                let n = self.u32()? as usize;
                let mut polys = Vec::with_capacity(n);
                for _ in 0..n {
                    let v = self.u32()? as usize;
                    polys.push((0..v).map(|_| self.vec3()).collect::<Result<Vec<_>>>()?);
                }
                data = Data::Collision { surface, params, polys };
            }
            24 => {
                let axes = [self.vec3()?, self.vec3()?, self.vec3()?];
                let pos = self.vec3()?;
                self.skip(1)?; // flag
                data = Data::Space { axes, pos };
                children = self.children(depth)?;
            }
            25 => self.skip(1 + 2 + 3 + 3 + 5)?,
            26 => {
                self.skip(4 + 9)?;
                children = self.children(depth)?;
            }
            27 => self.skip(4 + 1 + 3 + 1)?,
            28 => {
                self.skip(4 + 3)?;
                data = Data::Sprites(self.polygons(true)?.1);
            }
            29 => {
                // sphere, n, u32, sphere, n floats (the reference description orders this differently)
                self.skip(4)?;
                let n = self.u32()? as usize;
                self.skip(1 + 4 + n)?;
                children = self.children(depth)?;
            }
            30 => {
                self.skip(4)?;
                names = self.names(1)?;
                self.skip(6)?;
            }
            31 => {
                self.skip(4)?;
                let n = self.u32()? as usize;
                self.skip(4 + 1 + 3)?;
                self.skip(2 * n)?;
            }
            33 => {
                self.skip(4 + 3 + 6 + 6 + 3)?;
                children = self.children(depth)?;
            }
            34 => {
                self.skip(4 + 1)?;
                let n = self.u32()? as usize;
                self.skip(4 * n)?;
            }
            35 => {
                self.skip(4 + 2)?;
                data = Data::Polys(self.polygons(false)?.0);
            }
            36 | 37 => {
                self.skip(4)?;
                names = self.names(if kind == 36 { 2 } else { 1 })?;
                let format_raw = self.u32()?;
                data = Data::Verts(self.complex_verts(format_raw)?);
                children = self.children(depth)?;
            }
            39 => {
                self.skip(4 + 5)?;
                children = self.children(depth)?;
            }
            40 => {
                self.skip(4)?;
                names = self.names(2)?;
                self.skip(2)?;
                self.counted_words()?;
            }
            k => return Err(Error::Other(format!("unknown b3d block type {k} at offset {}", self.o))),
        }
        while self.u32()? != 555 {}
        Ok(Node { name, kind, names, data, children })
    }
}

pub fn parse(d: &[u8]) -> Result<B3d> {
    let mut r = R { d, o: 0 };
    if r.take(4)? != b"b3d\0" {
        return Err(Error::Other("bad b3d magic".into()));
    }
    let len_file = r.u32()? as usize;
    r.skip(2)?; // ofc_materials, len_materials
    let ofc_nodes = r.u32()? as usize;
    let len_nodes = r.u32()? as usize;
    if len_file * 4 != d.len() || ofc_nodes + len_nodes != len_file {
        return Err(Error::Other("b3d header sizes do not match the file".into()));
    }
    let n = r.u32()? as usize;
    let materials = (0..n).map(|_| r.name()).collect::<Result<_>>()?;
    if r.o != ofc_nodes * 4 {
        return Err(Error::Other("material table does not end at the node section".into()));
    }
    if r.u32()? != 111 {
        return Err(Error::Other("missing node section start marker".into()));
    }
    let end = d.len() - 4;
    let mut nodes = Vec::new();
    while r.o < end {
        nodes.push(r.block(0)?);
    }
    if r.o != end || u32::from_le_bytes(d[end..].try_into().unwrap()) != 222 {
        return Err(Error::Other("node section does not end with marker 222".into()));
    }
    Ok(B3d { materials, nodes })
}

/// One triangle list that uses a single material.
#[derive(Debug, Default)]
pub struct Batch {
    pub material: u32,
    pub verts: Vec<Vertex>,
}

impl B3d {
    /// Every collision block placed like the geometry: prototype groups only where a link
    /// instantiates them.
    pub fn collision_items(&self) -> Vec<CollisionItem> {
        let ctx = self.context(None, &[]);
        let mut out = Vec::new();
        for n in &self.nodes {
            collect_collision(n, &Xf::IDENTITY, &ctx, 0, false, &mut out);
        }
        out
    }

    /// The rooms of the scene with the collision of their road and objects.
    pub fn rooms(&self) -> Vec<Room> {
        fn find_rooms<'a>(ns: &'a [Node], out: &mut Vec<&'a Node>) {
            for n in ns {
                if n.kind == 19 && n.name.len() > 4 && n.name.is_char_boundary(4) {
                    out.push(n);
                }
                find_rooms(&n.children, out);
            }
        }
        let ctx = self.context(None, &[]);
        let mut rooms = Vec::new();
        find_rooms(&self.nodes, &mut rooms);
        rooms
            .into_iter()
            .map(|n| {
                let suffix = &n.name[4..];
                let items = |prefix: &str| {
                    let mut out = Vec::new();
                    if let Some(r) = find_in_room(n, &format!("{prefix}{suffix}")) {
                        let target = r.names.first().filter(|t| !t.is_empty()).and_then(|t| ctx.defs.get(t.as_str()).copied());
                        if let Some(def) = target {
                            collect_collision(def, &Xf::IDENTITY, &ctx, 0, true, &mut out);
                        } else {
                            collect_collision(r, &Xf::IDENTITY, &ctx, 0, true, &mut out);
                        }
                    }
                    out
                };
                let mut neighbours = Vec::new();
                portals(n, &mut neighbours);
                Room { name: n.name.clone(), road: items("road"), obj: items("obj"), neighbours }
            })
            .collect()
    }

    /// Every collision mesh (type 23) with its surface record, in file order.
    pub fn collision_meshes(&self) -> Vec<(&[u32; 2], &[u32], &[Vec<Vec3>])> {
        fn walk<'a>(ns: &'a [Node], out: &mut Vec<(&'a [u32; 2], &'a [u32], &'a [Vec<Vec3>])>) {
            for n in ns {
                if let Data::Collision { surface, params, polys } = &n.data {
                    out.push((surface, params, polys));
                }
                walk(&n.children, out);
            }
        }
        let mut out = Vec::new();
        walk(&self.nodes, &mut out);
        out
    }

    /// Origin of the `Space` block called `name`, if there is one.
    pub fn space_origin(&self, name: &str) -> Option<Vec3> {
        fn find(ns: &[Node], name: &str) -> Option<Vec3> {
            ns.iter().find_map(|n| match &n.data {
                Data::Space { pos, .. } if n.name == name => Some(*pos),
                _ => find(&n.children, name),
            })
        }
        find(&self.nodes, name)
    }

    /// Flatten the node tree into triangles grouped by material.
    ///
    /// Groups are prototypes: a type-18 "link" block names a `Space` (a transform) and a group,
    /// and instantiates that group there. Groups that some link refers to are therefore drawn
    /// only through their links, never in place.
    pub fn triangles(&self) -> Vec<Batch> {
        self.triangles_in(None)
    }

    /// Like [`triangles`](Self::triangles) but keeps only the subtrees of blocks named `only`.
    pub fn triangles_in(&self, only: Option<&str>) -> Vec<Batch> {
        self.triangles_skipping(only, &[])
    }

    /// Like [`triangles_in`](Self::triangles_in), leaving out links that instantiate the groups in `skip`.
    pub fn triangles_skipping(&self, only: Option<&str>, skip: &[&str]) -> Vec<Batch> {
        let ctx = self.context(only, skip);
        let mut batches: Vec<Batch> = Vec::new();
        for n in &self.nodes {
            collect(n, &Xf::IDENTITY, &[], &ctx, only.is_none(), 0, false, &mut batches);
        }
        batches
    }

    /// Geometry of one prototype group in its own coordinates, without any link transform.
    pub fn group_triangles(&self, group: &str) -> Vec<Batch> {
        let ctx = self.context(None, &[]);
        let mut batches = Vec::new();
        if let Some(def) = ctx.defs.get(group) {
            collect(def, &Xf::IDENTITY, &[], &ctx, true, 0, true, &mut batches);
        }
        batches
    }

    /// Transform (axes and origin) of the `Space` block called `name`.
    pub fn space(&self, name: &str) -> Option<([Vec3; 3], Vec3)> {
        let ctx = self.context(None, &[]);
        ctx.spaces.get(name).map(|x| (x.axes, x.pos))
    }

    /// Name of the group that the first link using space `space` instantiates.
    pub fn linked_group(&self, space: &str) -> Option<&str> {
        fn find<'a>(ns: &'a [Node], space: &str) -> Option<&'a str> {
            ns.iter().find_map(|n| match (n.kind, n.names.as_slice()) {
                (18, [s, g]) if s == space => Some(g.as_str()),
                _ => find(&n.children, space),
            })
        }
        find(&self.nodes, space)
    }

    fn context<'a>(&'a self, only: Option<&'a str>, skip: &'a [&'a str]) -> Ctx<'a> {
        let mut ctx = Ctx { spaces: HashMap::new(), defs: HashMap::new(), targets: HashSet::new(), only, skip };
        index(&self.nodes, &mut ctx);
        ctx
    }
}

struct Ctx<'a> {
    skip: &'a [&'a str],
    spaces: HashMap<&'a str, Xf>,
    /// First block of each name, in file order.
    defs: HashMap<&'a str, &'a Node>,
    /// Names of groups that some link instantiates.
    targets: HashSet<&'a str>,
    only: Option<&'a str>,
}

fn index<'a>(ns: &'a [Node], ctx: &mut Ctx<'a>) {
    for n in ns {
        match (&n.data, n.kind, n.names.as_slice()) {
            (Data::Space { axes, pos }, _, _) => {
                ctx.spaces.insert(&n.name, Xf { axes: *axes, pos: *pos });
            }
            (_, 18, [_, group]) => {
                ctx.targets.insert(group);
            }
            _ if !n.name.is_empty() => {
                ctx.defs.entry(&n.name).or_insert(n);
            }
            _ => {}
        }
        index(&n.children, ctx);
    }
}

#[derive(Clone, Copy)]
struct Xf {
    axes: [Vec3; 3],
    pos: Vec3,
}

impl Xf {
    const IDENTITY: Xf = Xf { axes: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]], pos: [0.0; 3] };

    fn apply(&self, p: Vec3) -> Vec3 {
        let [a, b, c] = self.axes;
        std::array::from_fn(|i| self.pos[i] + p[0] * a[i] + p[1] * b[i] + p[2] * c[i])
    }

    /// `self` is the parent transform, `local` the child's.
    fn then(&self, local: &Xf) -> Xf {
        Xf {
            axes: local.axes.map(|v| {
                let [a, b, c] = self.axes;
                std::array::from_fn(|i| v[0] * a[i] + v[1] * b[i] + v[2] * c[i])
            }),
            pos: self.apply(local.pos),
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn collect(n: &Node, xf: &Xf, pool: &[Vertex], ctx: &Ctx, emit: bool, depth: u32, via_link: bool, out: &mut Vec<Batch>) {
    if depth > 24 {
        return; // cyclic or absurdly deep link chain
    }
    if let (18, [space, group]) = (n.kind, n.names.as_slice()) {
        if ctx.skip.contains(&group.as_str()) {
            return;
        }
        if let Some(def) = ctx.defs.get(group.as_str()) {
            let at = ctx.spaces.get(space.as_str()).map_or(*xf, |local| xf.then(local));
            collect(def, &at, &[], ctx, emit, depth + 1, true, out);
        }
        return;
    }
    if !via_link && ctx.targets.contains(n.name.as_str()) {
        return; // a prototype: drawn only where a link places it
    }
    let emit = emit || ctx.only == Some(n.name.as_str());
    let mut pool = pool;
    match &n.data {
        Data::Verts(v) => pool = v,
        Data::Polys(polys) if emit => {
            for p in polys {
                let Some(vs) = p.verts.iter().map(|pv| pool.get(pv.index as usize).map(|v| (v, pv.uv))).collect::<Option<Vec<_>>>() else {
                    continue;
                };
                let slot = match out.iter().position(|b| b.material == p.material) {
                    Some(i) => i,
                    None => {
                        out.push(Batch { material: p.material, verts: Vec::new() });
                        out.len() - 1
                    }
                };
                let mk = |i: usize| Vertex { pos: xf.apply(vs[i].0.pos), uv: vs[i].1.unwrap_or(vs[i].0.uv) };
                for i in 1..vs.len().saturating_sub(1) {
                    out[slot].verts.extend([mk(0), mk(i), mk(i + 1)]);
                }
            }
        }
        _ => {}
    }
    for c in &n.children {
        collect(c, xf, pool, ctx, emit, depth, false, out);
    }
}

/// The shape of a collision item placed in the scene.
#[derive(Debug, Clone)]
pub enum CollisionShape {
    /// Type 23: polygons (vertex loops).
    Polys(Vec<Vec<Vec3>>),
    /// Type 12: the plane `n.x + d = 0`.
    Plane([f32; 4]),
}

/// A collision block placed in the scene, with its surface record (type, kind, parameter words).
#[derive(Debug, Clone)]
pub struct CollisionItem {
    pub surface: [u32; 2],
    pub params: Vec<u32>,
    pub shape: CollisionShape,
}

/// A room (type 19 block): the collision of its road (`road<suffix>`) and of its objects
/// (`obj<suffix>`), where `<suffix>` is the room name after its first four letters, and the
/// rooms its portals (type 30) lead to.
#[derive(Debug, Clone, Default)]
pub struct Room {
    pub name: String,
    pub road: Vec<CollisionItem>,
    pub obj: Vec<CollisionItem>,
    pub neighbours: Vec<String>,
}

fn collect_collision(n: &Node, xf: &Xf, ctx: &Ctx, depth: u32, via_link: bool, out: &mut Vec<CollisionItem>) {
    if depth > 24 {
        return;
    }
    if let (18, [space, group]) = (n.kind, n.names.as_slice()) {
        if let Some(def) = ctx.defs.get(group.as_str()) {
            let at = ctx.spaces.get(space.as_str()).map_or(*xf, |local| xf.then(local));
            collect_collision(def, &at, ctx, depth + 1, true, out);
        }
        return;
    }
    if !via_link && ctx.targets.contains(n.name.as_str()) {
        return;
    }
    match &n.data {
        Data::Collision { surface, params, polys } => out.push(CollisionItem {
            surface: *surface,
            params: params.clone(),
            shape: CollisionShape::Polys(polys.iter().map(|p| p.iter().map(|v| xf.apply(*v)).collect()).collect()),
        }),
        Data::CollisionPlane { plane, surface, params, .. } => {
            let [a, b, c] = xf.axes;
            let nl = [plane[0], plane[1], plane[2]];
            let nw: Vec3 = std::array::from_fn(|i| nl[0] * a[i] + nl[1] * b[i] + nl[2] * c[i]);
            let d = plane[3] - (nw[0] * xf.pos[0] + nw[1] * xf.pos[1] + nw[2] * xf.pos[2]);
            out.push(CollisionItem { surface: *surface, params: params.clone(), shape: CollisionShape::Plane([nw[0], nw[1], nw[2], d]) });
        }
        _ => {}
    }
    for c in &n.children {
        collect_collision(c, xf, ctx, depth, false, out);
    }
}

/// First block named `name` below `n`, not looking into other rooms.
fn find_in_room<'a>(n: &'a Node, name: &str) -> Option<&'a Node> {
    n.children.iter().find_map(|c| if c.name.eq_ignore_ascii_case(name) { Some(c) } else if c.kind == 19 { None } else { find_in_room(c, name) })
}

fn portals(n: &Node, out: &mut Vec<String>) {
    for c in &n.children {
        match (c.kind, c.names.as_slice()) {
            (30, [to]) => out.push(to.clone()),
            (19, _) => {}
            _ => portals(c, out),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words(ws: &[u32]) -> Vec<u8> {
        ws.iter().flat_map(|w| w.to_le_bytes()).collect()
    }

    fn name32(s: &str) -> Vec<u8> {
        let mut v = s.as_bytes().to_vec();
        v.resize(32, 0);
        v
    }

    /// One material, one type-19 group with one empty type-19 child.
    fn sample() -> Vec<u8> {
        let mut mats = words(&[1]);
        mats.extend(name32("col0"));
        let mut nodes = words(&[111, 333]);
        nodes.extend(name32("root"));
        nodes.extend(words(&[19, 1, 333]));
        nodes.extend(name32("kid"));
        nodes.extend(words(&[19, 0, 555, 555, 222]));

        let ofc_materials = 6u32;
        let len_materials = (mats.len() / 4) as u32;
        let ofc_nodes = ofc_materials + len_materials;
        let len_nodes = (nodes.len() / 4) as u32;
        let mut d = b"b3d\0".to_vec();
        d.extend(words(&[ofc_nodes + len_nodes, ofc_materials, len_materials, ofc_nodes, len_nodes]));
        d.extend(mats);
        d.extend(nodes);
        d
    }

    #[test]
    fn parses_tree() {
        let m = parse(&sample()).unwrap();
        assert_eq!(m.materials, ["col0"]);
        assert_eq!(m.nodes.len(), 1);
        assert_eq!(m.nodes[0].name, "root");
        assert_eq!(m.nodes[0].children[0].name, "kid");
        assert!(m.triangles().is_empty());
    }

    #[test]
    fn rejects_truncated_and_bad_magic() {
        let d = sample();
        assert!(parse(&d[..d.len() - 4]).is_err());
        let mut bad = d.clone();
        bad[0] = b'x';
        assert!(parse(&bad).is_err());
    }
}

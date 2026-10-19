//! Road geometry of the AI world (`Ai_main.cpp`): the road segments of the `.way` files and the
//! points and directions along them.
//!
//! A segment (`RSEG`) is either a polyline (`ATTR` bit 0 clear, class `0x64b5c0`) or a cubic
//! curve through four control points (bit 0 set, class `0x64b600`; the original builds it as a
//! B-spline with the knots `0,0,0,0,1,1,1,1` and keeps it as polynomial pieces in power form,
//! `0x4904e0`, `0x48dbb0`). A place on a segment is a piece index and a parameter `t` in `0..1`
//! (the `PositionId` of the original).

/// A road segment.
#[derive(Debug, Clone)]
pub struct Segment {
    pub shape: Shape,
    /// `WDTH`: the road width at the start and at the end of the segment.
    pub width: [f64; 2],
    /// `ATTR`: flags, a real value and a further word.
    pub flags: u32,
    pub value: f64,
    pub attr2: u32,
}

#[derive(Debug, Clone)]
pub enum Shape {
    /// Straight pieces between consecutive points.
    Polyline(Vec<[f64; 3]>),
    /// One cubic piece: the coefficients of the position and of its derivative, highest power
    /// first (the original evaluates them with Horner's rule, `0x454b00` / `0x454bd0`).
    Curve { pos: [[f64; 3]; 4], tangent: [[f64; 3]; 3], arc: Vec<f64> },
}

/// Value of the scalar polynomial `c` (highest power first) at `t` (`0x48e960`).
fn eval(c: &[f64], t: f64) -> f64 {
    c.iter().skip(1).fold(c[0], |r, k| r * t + k)
}

/// Product of two scalar polynomials.
fn mul(a: &[f64], b: &[f64]) -> Vec<f64> {
    let mut r = vec![0.0; a.len() + b.len() - 1];
    for (i, x) in a.iter().enumerate() {
        for (j, y) in b.iter().enumerate() {
            r[i + j] += x * y;
        }
    }
    r
}

/// The arc length along a curve as a polynomial in `t` (`0x48dbb0`): the speed `|P'(t)|` is
/// interpolated at the nine Chebyshev nodes of `0..1` (`0x48fe80`, degree 8) and integrated
/// (`0x48ea30`, zero at `t = 0`).
fn arc_length_poly(tangent: &[[f64; 3]; 3]) -> Vec<f64> {
    // |P'|^2 = sum over the coordinates of the squared derivative polynomials (0x48e600)
    let mut sq = vec![0.0; 5];
    for i in 0..3 {
        let d: Vec<f64> = tangent.iter().map(|c| c[i]).collect();
        for (k, v) in mul(&d, &d).iter().enumerate() {
            sq[k] += v;
        }
    }
    let n = 8usize;
    let nodes: Vec<f64> = (0..=n).map(|j| (((2 * j) as f64 + 1.0) * std::f64::consts::PI / ((n as f64 + 1.0) * 2.0)).cos() * 0.5 + 0.5).collect();
    let mut poly = vec![0.0; n + 1];
    for (j, xj) in nodes.iter().enumerate() {
        let mut w = eval(&sq, *xj).sqrt();
        let mut term = vec![1.0];
        for (k, xk) in nodes.iter().enumerate() {
            if k != j {
                w /= xj - xk;
                term = mul(&term, &[1.0, -xk]);
            }
        }
        for (k, v) in term.iter().enumerate() {
            poly[k] += v * w;
        }
    }
    let m = poly.len();
    let mut s: Vec<f64> = poly.iter().enumerate().map(|(i, c)| c / (m - i) as f64).collect();
    s.push(0.0);
    s
}

fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn horner<const N: usize>(c: &[[f64; 3]; N], t: f64) -> [f64; 3] {
    let mut r = c[0];
    for k in c.iter().skip(1) {
        r = [r[0] * t + k[0], r[1] * t + k[1], r[2] * t + k[2]];
    }
    r
}

impl Segment {
    /// Build a segment from `.way` data. Polylines lose points that repeat the previous one
    /// (closer than 1e-10, as `0x406600` drops them).
    pub fn new(flags: u32, value: f64, attr2: u32, width: [f64; 2], points: &[[f64; 3]]) -> Option<Self> {
        let shape = if flags & 1 == 0 {
            let mut pts: Vec<[f64; 3]> = Vec::new();
            for p in points {
                if let Some(last) = pts.last() {
                    let d = sub(*p, *last);
                    if (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt() < 1e-10 {
                        continue;
                    }
                }
                pts.push(*p);
            }
            if pts.len() < 2 {
                return None;
            }
            Shape::Polyline(pts)
        } else {
            let [p0, p1, p2, p3]: [[f64; 3]; 4] = points.try_into().ok()?;
            let c = |f: &dyn Fn(usize) -> f64| -> [f64; 3] { [f(0), f(1), f(2)] };
            let a = c(&|i| -p0[i] + 3.0 * p1[i] - 3.0 * p2[i] + p3[i]);
            let b = c(&|i| 3.0 * p0[i] - 6.0 * p1[i] + 3.0 * p2[i]);
            let d = c(&|i| -3.0 * p0[i] + 3.0 * p1[i]);
            let pos = [a, b, d, p0];
            let tangent = [c(&|i| 3.0 * a[i]), c(&|i| 2.0 * b[i]), d];
            let arc = arc_length_poly(&tangent);
            Shape::Curve { pos, tangent, arc }
        };
        Some(Segment { shape, width, flags, value, attr2 })
    }

    /// Number of pieces (`vtable + 0x18`): polyline points less one, one for a curve.
    pub fn pieces(&self) -> usize {
        match &self.shape {
            Shape::Polyline(p) => p.len() - 1,
            Shape::Curve { .. } => 1,
        }
    }

    /// The point at `t` of piece `k` (`vtable + 0`: `0x455440`, `0x454b00`).
    pub fn point(&self, t: f64, k: usize) -> [f64; 3] {
        match &self.shape {
            Shape::Polyline(p) => {
                let (a, b) = (p[k], p[k + 1]);
                let s = 1.0 - t;
                [a[0] * s + b[0] * t, a[1] * s + b[1] * t, a[2] * s + b[2] * t]
            }
            Shape::Curve { pos, .. } => horner(pos, t),
        }
    }

    /// Move `dist` metres (negative: backwards) from `t` on piece `k` (`vtable + 0x10`:
    /// `0x48d100` for polylines, `0x454f10` / `0x48df80` for curves; `step` is the parameter step
    /// the curve search starts from, kept between calls). Returns `true` when the end (or the
    /// start) of the segment was reached; `dist` keeps what is left to go.
    pub fn advance(&self, t: &mut f64, dist: &mut f64, k: &mut usize, step: &mut f64) -> bool {
        match &self.shape {
            Shape::Polyline(p) => {
                let len = |a: [f64; 3], b: [f64; 3]| {
                    let d = sub(a, b);
                    (d[2] * d[2] + d[1] * d[1] + d[0] * d[0]).sqrt()
                };
                if *dist > 0.0 {
                    while *k < p.len() - 1 {
                        let rem = len(p[*k + 1], self.point(*t, *k));
                        if *dist <= rem {
                            *t += *dist / len(p[*k + 1], p[*k]);
                            *dist = 0.0;
                            if 1.0 < *t {
                                *t = 1.0;
                            }
                            return false;
                        }
                        *dist -= rem;
                        if p.len() - 2 <= *k {
                            *t = 1.0;
                            return true;
                        }
                        *k += 1;
                        *t = 0.0;
                    }
                    true
                } else if *dist < 0.0 {
                    loop {
                        let rem = len(p[*k], self.point(*t, *k));
                        if -*dist <= rem {
                            *t += *dist / len(p[*k + 1], p[*k]);
                            *dist = 0.0;
                            if *t < 0.0 {
                                *t = 0.0;
                            }
                            return false;
                        }
                        *dist += rem;
                        if *k == 0 {
                            *t = 0.0;
                            return true;
                        }
                        *k -= 1;
                        *t = 1.0;
                    }
                } else {
                    false
                }
            }
            Shape::Curve { arc, .. } => {
                let len = |a: f64, b: f64| eval(arc, b) - eval(arc, a);
                if *dist > 0.0 {
                    let l = len(*t, 1.0);
                    if l <= *dist {
                        *t = 1.0;
                        *dist -= l;
                        if *dist < 0.0 {
                            *dist = 0.0;
                        }
                        return true;
                    }
                } else if *dist < 0.0 {
                    let l = len(*t, 0.0);
                    if *dist <= l {
                        *t = 0.0;
                        *dist -= l;
                        if 0.0 < *dist {
                            *dist = 0.0;
                        }
                        return true;
                    }
                }
                let t0 = *t;
                for _ in 0..6 {
                    let l = len(t0, t0 + *step);
                    *step = *dist / l * *step;
                }
                *t = (*step + t0).clamp(0.0, 1.0);
                *dist = 0.0;
                false
            }
        }
    }

    /// The arc-length polynomial of a curve (highest power first), as the original keeps it at
    /// `piece + 0x570`.
    pub fn arc_poly(&self) -> Option<&[f64]> {
        match &self.shape {
            Shape::Curve { arc, .. } => Some(arc),
            Shape::Polyline(_) => None,
        }
    }

    /// Length of the whole segment.
    pub fn length(&self) -> f64 {
        match &self.shape {
            Shape::Polyline(p) => p.windows(2).map(|w| {
                let d = sub(w[1], w[0]);
                (d[2] * d[2] + d[1] * d[1] + d[0] * d[0]).sqrt()
            }).sum(),
            Shape::Curve { arc, .. } => eval(arc, 1.0) - eval(arc, 0.0),
        }
    }

    /// The direction at `t` of piece `k`, not normalised (`vtable + 4`: `0x4554d0`, `0x454bd0`).
    pub fn tangent(&self, t: f64, k: usize) -> [f64; 3] {
        match &self.shape {
            Shape::Polyline(p) => sub(p[k + 1], p[k]),
            Shape::Curve { tangent, .. } => horner(tangent, t),
        }
    }
}

/// The road segments of a `.way` room (`GROM`).
#[derive(Debug, Clone, Default)]
pub struct Room {
    pub name: String,
    pub segments: Vec<Segment>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn straight_curve_is_linear() {
        let p = [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [2.0, 0.0, 0.0], [3.0, 0.0, 0.0]];
        let s = Segment::new(1, 0.0, 1, [12.0, 12.0], &p).unwrap();
        let q = s.point(0.5, 0);
        assert!((q[0] - 1.5).abs() < 1e-12);
        let d = s.tangent(0.25, 0);
        assert!((d[0] - 3.0).abs() < 1e-12);
        assert!((s.length() - 3.0).abs() < 1e-9);
        let (mut t, mut dist, mut k, mut step) = (0.0, 1.2, 0, 0.1);
        assert!(!s.advance(&mut t, &mut dist, &mut k, &mut step));
        assert!((t - 0.4).abs() < 1e-9);
    }

    #[test]
    fn polyline_drops_repeated_points() {
        let p = [[0.0, 0.0, 0.0], [0.0, 0.0, 0.0], [1.0, 0.0, 0.0]];
        let s = Segment::new(0, 0.0, 1, [6.0, 6.0], &p).unwrap();
        assert_eq!(s.pieces(), 1);
        assert_eq!(s.point(1.0, 0), [1.0, 0.0, 0.0]);
    }
}

impl Segment {
    /// `vtable + 0x1c` (`0x4555e0`, `0x4551d0`): run the segment the other way (the side flags 4
    /// and 8 swap).
    pub fn reverse(&mut self) {
        let f = self.flags;
        self.flags = (f & !0xc) | if f & 4 != 0 { 8 } else { 0 } | if f & 8 != 0 { 4 } else { 0 };
        match &mut self.shape {
            Shape::Polyline(p) => p.reverse(),
            Shape::Curve { pos, .. } => {
                let c = control_points(pos);
                let rebuilt = Segment::new(1, self.value, self.attr2, self.width, &[c[3], c[2], c[1], c[0]]).expect("four points");
                self.shape = rebuilt.shape;
            }
        }
    }
}

/// The control points back from the power coefficients of a curve.
fn control_points(pos: &[[f64; 3]; 4]) -> [[f64; 3]; 4] {
    let [a, b, d, p0] = *pos;
    let p1: [f64; 3] = core::array::from_fn(|i| p0[i] + d[i] / 3.0);
    let p2: [f64; 3] = core::array::from_fn(|i| b[i] / 3.0 + 2.0 * p1[i] - p0[i]);
    let p3: [f64; 3] = core::array::from_fn(|i| a[i] + p0[i] - 3.0 * p1[i] + 3.0 * p2[i]);
    [p0, p1, p2, p3]
}

/// A node of the road graph: a named place of a `.way` file or an automatic one (`anode`)
/// where roads meet.
#[derive(Debug, Clone)]
pub struct Node {
    pub name: String,
    pub pos: [f64; 3],
    /// `+0xc`: 8 for nodes roads do not snap to (`FLAG` bit 0), 0x40 for `FLAG` bit 1.
    pub flags: u32,
    /// Roads leaving (`+0x18`) and entering (`+0x24`) the node.
    pub out: Vec<usize>,
    pub inc: Vec<usize>,
}

/// A road: segments chained from one node to another (`0x405c00`).
#[derive(Debug, Clone)]
pub struct Road {
    pub start: usize,
    pub end: usize,
    pub segments: Vec<usize>,
    /// `+0x18`: total length; `+0x28`: the length of this and all earlier roads.
    pub length: f64,
    pub cumulative: f64,
}

/// The road network of the AI world.
#[derive(Debug, Clone, Default)]
pub struct RoadNet {
    pub nodes: Vec<Node>,
    pub segments: Vec<Segment>,
    pub roads: Vec<Road>,
}

struct Endpoint {
    pos: [f64; 3],
    visited: bool,
    node: Option<usize>,
    /// (segment, start endpoint, end endpoint)
    ends: Vec<(usize, usize, usize)>,
}

fn dist(a: [f64; 3], b: [f64; 3]) -> f64 {
    let d = sub(a, b);
    (d[0] * d[0] + d[2] * d[2] + d[1] * d[1]).sqrt()
}

impl RoadNet {
    /// Build the network from the rooms of the `.way` files, in the order the original reads
    /// them (`0x406600`: files sorted by name).
    pub fn build(ways: &[Vec<ht2_formats::way::Room>]) -> Self {
        let mut net = RoadNet::default();
        let mut eps: Vec<Endpoint> = Vec::new();
        let mut seg_done: Vec<bool> = Vec::new();
        // 0x405730 / 0x45b220: the first endpoint within 0.1 of `p`, or a new one at the end
        let find = |eps: &mut Vec<Endpoint>, p: [f64; 3]| -> usize {
            match eps.iter().position(|e| dist(p, e.pos) < 0.1) {
                Some(i) => i,
                None => {
                    eps.push(Endpoint { pos: p, visited: false, node: None, ends: Vec::new() });
                    eps.len() - 1
                }
            }
        };
        for rooms in ways {
            for room in rooms {
                for s in &room.segments {
                    let Some(seg) = Segment::new(s.kind, s.value, s.flag, s.width, &s.points) else { continue };
                    let a = seg.point(0.0, 0);
                    let b = seg.point(1.0, seg.pieces() - 1);
                    let id = net.segments.len();
                    net.segments.push(seg);
                    seg_done.push(false);
                    let ea = find(&mut eps, a);
                    let eb = find(&mut eps, b);
                    eps[ea].ends.push((id, ea, eb));
                    eps[eb].ends.push((id, ea, eb));
                }
                for n in &room.nodes {
                    if net.nodes.iter().any(|x| x.name == n.name) {
                        continue;
                    }
                    let flags = if n.flag & 1 != 0 { 8 } else { 0 } | if n.flag & 2 != 0 { 0x40 } else { 0 };
                    net.nodes.push(Node { name: n.name.clone(), pos: n.pos, flags, out: Vec::new(), inc: Vec::new() });
                }
            }
        }
        // 0x405f90: a node at every endpoint: a named one within 0.1, else an automatic one where
        // other than two segment ends meet
        for e in eps.iter_mut() {
            let named = net.nodes.iter().position(|n| dist(e.pos, n.pos) < 0.1 && n.flags & 8 == 0);
            e.node = match named {
                Some(i) => Some(i),
                None if e.ends.len() != 2 => {
                    net.nodes.push(Node { name: "anode".into(), pos: e.pos, flags: 0, out: Vec::new(), inc: Vec::new() });
                    Some(net.nodes.len() - 1)
                }
                None => None,
            };
        }
        while let Some(i) = eps.iter().position(|e| !e.visited && e.node.is_some()) {
            net.walk(&mut eps, &mut seg_done, i);
        }
        // 0x406160: road lengths
        let mut total = 0.0;
        for r in net.roads.iter_mut() {
            r.length = r.segments.iter().map(|s| net.segments[*s].length()).sum();
            total += r.length;
            r.cumulative = total;
        }
        net
    }

    /// `0x405c00`: the roads leaving the endpoint `ep`, depth first (a road is numbered after
    /// the roads beyond its end).
    fn walk(&mut self, eps: &mut Vec<Endpoint>, done: &mut Vec<bool>, ep: usize) {
        if eps[ep].visited {
            return;
        }
        eps[ep].visited = true;
        let start = eps[ep].node.expect("endpoint with a node");
        let mut k = 0;
        while k < eps[ep].ends.len() {
            let first = eps[ep].ends[k];
            k += 1;
            if done[first.0] {
                continue;
            }
            let mut road = Road { start, end: 0, segments: Vec::new(), length: 0.0, cumulative: 0.0 };
            let mut cur = ep;
            let mut entry = first;
            let next = loop {
                done[entry.0] = true;
                let next = if entry.1 == cur {
                    entry.2
                } else {
                    self.segments[entry.0].reverse();
                    entry.1
                };
                road.segments.push(entry.0);
                if eps[next].node.is_some() {
                    break next;
                }
                let ends = &eps[next].ends;
                entry = if ends[0].0 == entry.0 { ends[1] } else { ends[0] };
                cur = next;
            };
            road.end = eps[next].node.expect("endpoint with a node");
            self.walk(eps, done, next);
            let id = self.roads.len();
            self.nodes[road.start].out.push(id);
            self.nodes[road.end].inc.push(id);
            self.roads.push(road);
        }
    }
}

impl Segment {
    /// Distance along the segment from `(t1, k1)` to `(t2, k2)` (`vtable + 0x20`: `0x48d710` for
    /// polylines, `0x455280` for curves). A place given as the piece one past the last is taken
    /// as the end of the last piece, as the original does.
    pub fn distance(&self, mut t1: f64, mut k1: usize, mut t2: f64, mut k2: usize) -> f64 {
        match &self.shape {
            Shape::Polyline(p) => {
                let n = p.len();
                if k1 + 1 == n {
                    t1 += 1.0;
                    k1 -= 1;
                }
                if k2 + 1 == n {
                    t2 += 1.0;
                    k2 -= 1;
                }
                let piece = |k: usize| dist(p[k + 1], p[k]);
                if k1 == k2 {
                    return (t2 - t1) * piece(k1);
                }
                let mut s = (1.0 - t1) * piece(k1);
                for k in k1 + 1..k2 {
                    s += piece(k);
                }
                piece(k2) * t2 + s
            }
            Shape::Curve { arc, .. } => {
                // one piece: the arc length between the two parameters, unsigned (0x48e430)
                if k1 == 1 {
                    t1 += 1.0;
                }
                if k2 == 1 {
                    t2 += 1.0;
                }
                (eval(arc, t2) - eval(arc, t1)).abs()
            }
        }
    }

    /// The road width at `(t, k)` (`0x438d70`, `getRoadParameters`): the widths at the two ends,
    /// interpolated by the distance along the segment.
    pub fn width_at(&self, t: f64, k: usize) -> f64 {
        let s = self.distance(0.0, 0, t, k);
        (self.width[1] - self.width[0]) * s / self.length() + self.width[0]
    }
}

impl Segment {
    /// `getRightDirection` (`0x445c90`): the direction traffic must take on the segment:
    /// `Some(false)` along it (flag 4), `Some(true)` against it (flag 8), `None` both ways.
    pub fn one_way(&self) -> Option<bool> {
        if self.flags & 8 != 0 {
            Some(true)
        } else if self.flags & 4 != 0 {
            Some(false)
        } else {
            None
        }
    }
}

/// A place in the road network (`PositionId` of the original, 8 words): a road, or a node
/// when the place is at the end of a road, the segment of the road, the piece of the segment,
/// the parameter on the piece and whether the place faces against the road.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Place {
    pub road: usize,
    pub node: Option<usize>,
    pub t: f64,
    pub piece: usize,
    pub segment: usize,
    pub reversed: bool,
}

fn norm(v: [f64; 3]) -> [f64; 3] {
    let l = (v[2] * v[2] + v[1] * v[1] + v[0] * v[0]).sqrt();
    if l < 1e-5 {
        [0.0; 3]
    } else {
        [v[0] / l, v[1] / l, v[2] / l]
    }
}

impl RoadNet {
    fn seg(&self, p: &Place) -> &Segment {
        &self.segments[self.roads[p.road].segments[p.segment]]
    }

    /// `getCoordinates` (`0x439580` → `0x435bf0`): the point of a place (a node's position when
    /// the place is a node).
    pub fn coordinates(&self, p: &Place) -> [f64; 3] {
        match p.node {
            Some(n) => self.nodes[n].pos,
            None => self.seg(p).point(p.t, p.piece),
        }
    }

    /// `getTangent` (`0x439320`): the direction of the road at a place, not normalised.
    pub fn tangent(&self, p: &Place) -> [f64; 3] {
        self.seg(p).tangent(p.t, p.piece)
    }

    /// `advancePosition` (`0x435d60`): move `dist` metres along the road (against it when the
    /// place is reversed), segment by segment. At the end of the road the place becomes the
    /// node there and `dist` keeps what is left.
    pub fn advance(&self, p: &mut Place, dist: &mut f64) {
        if p.reversed {
            *dist = -*dist;
        }
        let road = &self.roads[p.road];
        let mut step = 0.1;
        if *dist > 0.0 {
            while p.segment < road.segments.len() {
                let s = &self.segments[road.segments[p.segment]];
                if !s.advance(&mut p.t, dist, &mut p.piece, &mut step) || road.segments.len() - 1 <= p.segment {
                    break;
                }
                p.segment += 1;
                p.piece = 0;
                p.t = 0.0;
            }
        } else if *dist < 0.0 {
            while p.segment < road.segments.len() {
                let s = &self.segments[road.segments[p.segment]];
                if !s.advance(&mut p.t, dist, &mut p.piece, &mut step) || p.segment == 0 {
                    break;
                }
                p.segment -= 1;
                p.piece = self.segments[road.segments[p.segment]].pieces() - 1;
                p.t = 1.0;
            }
        }
        if *dist > 0.0 {
            p.node = Some(road.end);
        } else if *dist < 0.0 {
            p.node = Some(road.start);
        } else {
            p.node = None;
            return;
        }
        if p.reversed {
            *dist = -*dist;
        }
    }

    /// `0x4cda20`: the horizontal direction of the road at a place (against the road when
    /// `back`), and the slope angle of the road there.
    pub fn heading(&self, p: &Place, back: bool) -> ([f64; 3], f64) {
        let mut t = self.tangent(p);
        if back {
            t = [-t[0], -t[1], -t[2]];
        }
        let slope = norm(t)[2].asin();
        (norm([t[0], t[1], 0.0]), slope)
    }

    /// `0x4cd770`: the signed sideways distance of the point `q` from the centre line of the
    /// road at a place (positive to the left of the direction of travel), half the road width,
    /// the speed value of the segment (its `ATTR` real times 10) and the segment flags.
    pub fn lateral(&self, p: &Place, back: bool, q: [f64; 3]) -> (f64, f64, f64, u32) {
        let c = self.coordinates(p);
        let d = [q[0] - c[0], q[1] - c[1], 0.0];
        let mut t = self.tangent(p);
        if back {
            t = [-t[0], -t[1], -t[2]];
        }
        let u = norm([t[0], t[1], 0.0]);
        let k = u[1] * d[1] + u[0] * d[0] + u[2] * d[2];
        let l = [d[0] - u[0] * k, d[1] - u[1] * k, d[2] - u[2] * k];
        let mut side = (l[0] * l[0] + l[1] * l[1] + l[2] * l[2]).sqrt();
        if u[0] * l[1] - u[1] * l[0] < 0.0 {
            side = -side;
        }
        let s = self.seg(p);
        (side, s.width_at(p.t, p.piece) * 0.5, s.value * 10.0, s.flags)
    }
}

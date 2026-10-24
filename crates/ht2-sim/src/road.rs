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

/// Product of two scalar polynomials (`0x48eae0`): each coefficient sums `a[u - i] * b[i]` for
/// `i` rising, from 0 (the order fixes the rounding).
fn mul(a: &[f64], b: &[f64]) -> Vec<f64> {
    let (la, lb) = (a.len(), b.len());
    (0..la + lb - 1)
        .map(|u| {
            let i0 = (u + 1).saturating_sub(la);
            let i1 = (u + 1).min(lb);
            (i0..i1).fold(0.0, |acc, i| a[u - i] * b[i] + acc)
        })
        .collect()
}

/// The arc length along a curve as a polynomial in `t` (`0x48dbb0`): the speed `|P'(t)|` is
/// interpolated at the nine Chebyshev nodes of `0..1` (`0x48fe80`, degree 8) and integrated
/// (`0x48ea30`, zero at `t = 0`).
fn arc_length_poly(tangent: &[[f64; 3]; 3]) -> Vec<f64> {
    // |P'|^2: the squared derivative polynomials of the coordinates (0x48f000), added z + x + y
    // (0x48e600)
    let sq_of = |i: usize| {
        let d: Vec<f64> = tangent.iter().map(|c| c[i]).collect();
        mul(&d, &d)
    };
    let (x, y, z) = (sq_of(0), sq_of(1), sq_of(2));
    let sq: Vec<f64> = (0..x.len()).map(|k| z[k] + x[k] + y[k]).collect();
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

/// The B-spline basis polynomial `N(i, order)` on the knot span `span` (`0x48fa90`, Cox-de Boor):
/// order 1 is 1 on its own span and nothing elsewhere; above, `N(i, k - 1)` times
/// `(u - knot[i]) / (knot[i + k - 1] - knot[i])` plus `N(i + 1, k - 1)` times
/// `(knot[i + k] - u) / (knot[i + k] - knot[i + 1])`, the linear factors as their coefficients
/// times `1 / d` (`0x4903a0`).
fn bspline_basis(knots: &[f64], i: usize, order: usize, span: usize) -> Vec<f64> {
    if order == 1 {
        return if i == span { vec![1.0] } else { Vec::new() };
    }
    let left = bspline_basis(knots, i, order - 1, span);
    let right = bspline_basis(knots, i + 1, order - 1, span);
    let mut terms = Vec::new();
    if !right.is_empty() {
        let d = knots[i + order] - knots[i + 1];
        terms.push(mul(&right, &[-1.0 * (1.0 / d), knots[i + order] * (1.0 / d)]));
    }
    if !left.is_empty() {
        let d = knots[i + order - 1] - knots[i];
        terms.push(mul(&left, &[1.0 * (1.0 / d), -knots[i] * (1.0 / d)]));
    }
    terms.into_iter().reduce(|a, b| add(&a, &b)).unwrap_or_default()
}

/// The sum of two polynomials (highest power first), lined up at the constant terms (`0x4901b0`).
fn add(a: &[f64], b: &[f64]) -> Vec<f64> {
    let (long, short) = if a.len() >= b.len() { (a, b) } else { (b, a) };
    let off = long.len() - short.len();
    long.iter().enumerate().map(|(k, v)| if k < off { *v } else { *v + short[k - off] }).collect()
}

/// A curve segment as a cubic in its parameter (`0x4904e0`, `0x48d8f0`): the `.way` points are the
/// control points of a cubic B-spline with the knots 0, 0, 0, 0, 1, 1, 1, 1; the polynomial is
/// the sum of the basis polynomials times the control points, in their order.
fn bspline_piece(ctrl: &[[f64; 3]; 4]) -> [[f64; 3]; 4] {
    let knots = [0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 1.0];
    let mut acc: [Vec<f64>; 3] = Default::default();
    for (k, p) in ctrl.iter().enumerate() {
        let n = bspline_basis(&knots, k, 4, 3);
        for c in 0..3 {
            let term: Vec<f64> = n.iter().map(|b| b * p[c]).collect();
            acc[c] = if acc[c].is_empty() { term } else { add(&acc[c], &term) };
        }
    }
    let at = |c: usize, k: usize| {
        let v = &acc[c];
        let off = 4 - v.len();
        if k < off { 0.0 } else { v[k - off] }
    };
    core::array::from_fn(|k| [at(0, k), at(1, k), at(2, k)])
}

/// A curve from its position polynomial (`0x48dbb0`): the derivative (`0x48ef00`: each coefficient
/// times its power) and the arc-length polynomial.
fn curve_shape(pos: [[f64; 3]; 4]) -> Shape {
    let c = |f: &dyn Fn(usize) -> f64| -> [f64; 3] { [f(0), f(1), f(2)] };
    let tangent = [c(&|i| pos[0][i] * 3.0), c(&|i| pos[1][i] * 2.0), c(&|i| pos[2][i] * 1.0)];
    let arc = arc_length_poly(&tangent);
    Shape::Curve { pos, tangent, arc }
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
            let ctrl: [[f64; 3]; 4] = points.try_into().ok()?;
            curve_shape(bspline_piece(&ctrl))
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
                    // backwards the squares add as (x² + y²) + z² (0x48d435, 0x48d4e7)
                    let len_back = |a: [f64; 3], b: [f64; 3]| {
                        let d = sub(a, b);
                        (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt()
                    };
                    loop {
                        let rem = len_back(p[*k], self.point(*t, *k));
                        if -*dist <= rem {
                            *t += *dist / len_back(p[*k + 1], p[*k]);
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
                // 0x48e190: the polynomial in 1 - t, by Horner's rule over polynomials (0x48eda0):
                // the sum so far times (1 - t), plus the next coefficient
                let mut acc: Vec<[f64; 3]> = vec![pos[0]];
                for next in &pos[1..] {
                    let m = acc.len();
                    let mut out: Vec<[f64; 3]> = (0..=m)
                        .map(|u| {
                            core::array::from_fn(|c| {
                                let mut v = 0.0;
                                if u < m {
                                    v = acc[u][c] * -1.0 + v;
                                }
                                if u >= 1 {
                                    v = acc[u - 1][c] * 1.0 + v;
                                }
                                v
                            })
                        })
                        .collect();
                    let last = out.last_mut().unwrap();
                    *last = core::array::from_fn(|c| last[c] + next[c]);
                    acc = out;
                }
                let pos: [[f64; 3]; 4] = core::array::from_fn(|k| acc[k]);
                self.shape = curve_shape(pos);
            }
        }
    }
}

/// A node of the road graph: a named place of a `.way` file or an automatic one (`anode`)
/// where roads meet.
#[derive(Debug, Clone)]
pub struct Node {
    pub name: String,
    pub pos: [f64; 3],
    /// `+0xc`: 8 for nodes roads do not snap to (`FLAG` bit 0), 0x40 for `FLAG` bit 1, 0x80 for
    /// nodes with a placement matrix; towns add 1 and 2 ([`RoadNet::mark_towns`]).
    pub flags: u32,
    /// The `FLAG` of the `.way` node (0 for automatic nodes).
    pub flag_raw: u32,
    /// `+0x48`: the `ORTN` placement matrix of the `.way` node.
    pub orientation: Option<[f64; 12]>,
    /// Roads leaving (`+0x18`) and entering (`+0x24`) the node.
    pub out: Vec<usize>,
    pub inc: Vec<usize>,
    /// `+0xa8`, `+0xb4`: the town this node belongs to and its place in the town's node list
    /// (set by [`RoadNet::mark_towns`]).
    pub town: Option<(usize, usize)>,
    /// `+0xac`, `+0xb4`: the parking place and slot (set by [`RoadNet::mark_parkings`]).
    pub parking: Option<(usize, usize)>,
}

/// A town of the map scripts (`mod_*.sch`): `( town ID "name" ... ( ( node "name" attrs... ) ... ) ...)`,
/// read by `0x40e8e0`.
#[derive(Debug, Clone)]
pub struct Town {
    pub id: String,
    /// The number after the name (`+0x18`); the towns are kept in its order.
    pub number: i32,
    /// The town's nodes and their attribute flags: `unload` 1, `hidden` 2, `fuel` 4, `dummy` 8.
    pub nodes: Vec<(String, u32)>,
    /// `+0x60`: the number after the town's number (2 for a filling station; `()` is 0); bit 0
    /// is added by `0x434070` for towns that deal in a liquid (`fuel`) merchandise.
    pub flags: u32,
    /// The merchandise of the town (`( NAME amount )` list), by name.
    pub goods: Vec<String>,
}

/// The top-level forms of all map scripts (`mod_*.sch`) of a map folder, in file name order.
pub fn read_map_scripts(dir: &std::path::Path) -> Vec<ht2_formats::sexpr::Sexp> {
    let mut scripts: Vec<_> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            let n = p.file_name().map_or(String::new(), |n| n.to_string_lossy().to_ascii_lowercase());
            n.starts_with("mod_") && n.ends_with(".sch")
        })
        .collect();
    scripts.sort();
    let mut forms = Vec::new();
    for f in scripts {
        let Ok(data) = std::fs::read(&f) else { continue };
        let Ok(plain) = ht2_formats::sdte::decrypt(&data) else { continue };
        if let Ok(fs) = ht2_formats::sexpr::parse(&ht2_formats::sdte::cp1251_to_string(&plain)) {
            forms.extend(fs);
        }
    }
    forms
}

/// The forms called `name` among `forms` (and the lists they are wrapped in).
fn forms_named<'a>(forms: &'a [ht2_formats::sexpr::Sexp], name: &'a str) -> impl Iterator<Item = &'a [ht2_formats::sexpr::Sexp]> + 'a {
    forms.iter().flat_map(|f| f.as_list().map_or(vec![f], |l| l.iter().collect())).filter_map(move |f| f.form(name))
}

/// The `( node "name" ... )` entries of the first list of `rest` that has any.
fn node_entries(rest: &[ht2_formats::sexpr::Sexp]) -> Vec<&[ht2_formats::sexpr::Sexp]> {
    rest.iter().filter_map(|s| s.as_list()).find(|l| l.iter().any(|e| e.form("node").is_some())).map_or(Vec::new(), |l| l.iter().filter_map(|e| e.form("node")).collect())
}

/// A parking place of the map scripts: `( parking ID "name" number () ( ( node "name" [kind] ["town"] ) ... ) ... )`.
#[derive(Debug, Clone)]
pub struct Parking {
    pub id: String,
    /// `+0x18`; the parking places are sorted by it (`0x4143c0`).
    pub number: i32,
    /// The nodes: name, the number after it (0 when none) and the town named after that.
    pub nodes: Vec<(String, i32, Option<String>)>,
    /// `+0x30`: the number after the node list (4 when none): for a market, how many vehicle
    /// models each place offers and their price level ([`crate::world::market_offers`]).
    pub level: f64,
}

impl Parking {
    /// The parking places of the map scripts, in the order of the original.
    pub fn from_scripts(forms: &[ht2_formats::sexpr::Sexp]) -> Vec<Parking> {
        let mut lots: Vec<Parking> = forms_named(forms, "parking")
            .map(|rest| Parking {
                id: rest.first().and_then(|s| s.as_text()).unwrap_or_default().to_string(),
                number: rest.get(2).and_then(|s| s.as_text()).and_then(|t| t.parse().ok()).unwrap_or(0),
                nodes: node_entries(rest)
                    .into_iter()
                    .map(|n| {
                        let name = n.first().and_then(|s| s.as_text()).unwrap_or_default().to_string();
                        let kind = n.get(1).and_then(|s| s.as_text()).and_then(|t| t.parse().ok()).unwrap_or(0);
                        (name, kind, n.get(2).and_then(|s| s.as_text()).map(str::to_string))
                    })
                    .collect(),
                level: rest
                    .iter()
                    .position(|s| s.as_list().is_some_and(|l| l.iter().any(|e| e.form("node").is_some())))
                    .and_then(|k| rest.get(k + 1))
                    .and_then(|s| s.as_text())
                    .and_then(|t| t.parse().ok())
                    .unwrap_or(4.0),
            })
            .collect();
        crate::stl::sort(&mut lots, |a, b| a.number < b.number);
        lots
    }
}

impl Town {
    /// The towns of all map scripts of a map folder, in the order of the original.
    pub fn read_folder(dir: &std::path::Path) -> Vec<Town> {
        Town::from_scripts(&read_map_scripts(dir))
    }

    /// The towns of the map scripts, sorted by number (`0x4143c0`).
    pub fn from_scripts(forms: &[ht2_formats::sexpr::Sexp]) -> Vec<Town> {
        let mut towns = Town::parse(forms);
        crate::stl::sort(&mut towns, |a, b| a.number < b.number);
        towns
    }

    /// The towns of decrypted map scripts, in the order they are written.
    pub fn parse(forms: &[ht2_formats::sexpr::Sexp]) -> Vec<Town> {
        forms_named(forms, "town")
            .map(|rest| Town {
                id: rest.first().and_then(|s| s.as_text()).unwrap_or_default().to_string(),
                number: rest.get(2).and_then(|s| s.as_text()).and_then(|t| t.parse().ok()).unwrap_or(0),
                flags: rest.get(3).and_then(|s| s.as_text()).and_then(|t| t.parse().ok()).unwrap_or(0),
                goods: rest
                    .get(6)
                    .and_then(|s| s.as_list())
                    .map_or(Vec::new(), |l| l.iter().filter_map(|g| g.as_list().and_then(|g| g.first()).and_then(|n| n.as_text()).map(str::to_string)).collect()),
                nodes: node_entries(rest)
                    .into_iter()
                    .map(|n| {
                        let name = n.first().and_then(|s| s.as_text()).unwrap_or_default().to_string();
                        let flags = n[1..].iter().filter_map(|a| a.as_text()).fold(0, |f, a| {
                            f | match a {
                                "unload" => 1,
                                "hidden" => 2,
                                "fuel" => 4,
                                "dummy" => 8,
                                _ => 0,
                            }
                        });
                        (name, flags)
                    })
                    .collect(),
            })
            .collect()
    }
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
    /// The `.way` room each segment comes from (`RNAM`).
    pub segment_rooms: Vec<String>,
    /// `node + 0xc4 + mode * 4`: what the route searches so far found, per mode, node and
    /// destination: the road to take there (`direction << 7 | its place in the node's list of
    /// leaving (0) or arriving (1) roads`), 0x7f for no route, 0xff not known yet. Later searches
    /// follow it, so which of two routes of equal cost is taken depends on the searches before.
    pub next_step: std::cell::RefCell<Vec<Vec<Vec<u8>>>>,
    /// The flags of the segments as the game changes them (`segment + 8`): obstacles set 0x40 on
    /// the segments of their road and clear it again (`0x42b060`). The route costs, the random
    /// places and the ghosts read these.
    pub seg_flags: std::cell::RefCell<Vec<u32>>,
    /// `road + 0x38 + (mode · 2 + against) · 0x10`: the cost of each road once found
    /// (`0x409980`), kept until the searches of the mode start afresh (`0x484140`). So after an
    /// obstacle without a road replaces one with a road, the old road still costs what it did.
    pub cost_cache: std::cell::RefCell<Vec<[Option<f64>; 4]>>,
    /// `road + 0x10` bit 0x200: the human player has driven the road (`updatePlayerBoxes`,
    /// `0x42cf50`). Its segments then lose flag 0x20 where `getRoadParameters` (`0x438d70`)
    /// reads them, so vehicles may appear there.
    pub explored: std::cell::RefCell<Vec<bool>>,
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
                    net.segment_rooms.push(room.name.clone());
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
                    let flags = if n.flag & 1 != 0 { 8 } else { 0 } | if n.flag & 2 != 0 { 0x40 } else { 0 } | if n.orientation.is_some() { 0x80 } else { 0 };
                    net.nodes.push(Node { name: n.name.clone(), pos: n.pos, flags, flag_raw: n.flag, orientation: n.orientation, out: Vec::new(), inc: Vec::new(), town: None, parking: None });
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
                    net.nodes.push(Node { name: "anode".into(), pos: e.pos, flags: 0, flag_raw: 0, orientation: None, out: Vec::new(), inc: Vec::new(), town: None, parking: None });
                    Some(net.nodes.len() - 1)
                }
                None => None,
            };
        }
        while let Some(i) = eps.iter().position(|e| !e.visited && e.node.is_some()) {
            net.walk(&mut eps, &mut seg_done, i);
        }
        // 0x406600: a segment closed in the map (ATTR 0x40) gets 0xc0; 0x80 keeps it closed when
        // an obstacle there goes
        *net.seg_flags.borrow_mut() = net.segments.iter().map(|s| if s.flags & 0x40 != 0 { s.flags | 0xc0 } else { s.flags }).collect();
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

    /// The bounding sphere of the segment (`0x490a20`): the middle of the box around its points
    /// and the largest distance from there; for a curve the points at t = 0, 0.01 .. 0.99
    /// (`0x48dbb0`).
    pub fn bounds(&self) -> ([f64; 3], f64) {
        let pts: Vec<[f64; 3]> = match &self.shape {
            Shape::Polyline(p) => p.clone(),
            Shape::Curve { pos, .. } => (0..100).map(|i| horner(pos, i as f64 * 0.01)).collect(),
        };
        let (mut lo, mut hi) = (pts[0], pts[0]);
        for p in &pts[1..] {
            for c in 0..3 {
                if p[c] < lo[c] {
                    lo[c] = p[c];
                }
                if hi[c] < p[c] {
                    hi[c] = p[c];
                }
            }
        }
        let centre: [f64; 3] = core::array::from_fn(|c| (lo[c] + hi[c]) * 0.5);
        let mut r = 0.0;
        for p in &pts {
            let d = [centre[0] - p[0], centre[1] - p[1], centre[2] - p[2]];
            let l = (d[2] * d[2] + d[1] * d[1] + d[0] * d[0]).sqrt();
            if r < l {
                r = l;
            }
        }
        (centre, r)
    }

    /// `vtable + 0xc` (`0x48ce40` for polylines, `0x48dd50` for curves): the point of the segment
    /// nearest to `q` if its squared distance is below `best` (then `best`, `t` and the piece `k`
    /// are set and `true` returned). Segments whose bounding sphere is too far are skipped.
    pub fn project(&self, q: [f64; 3], best: &mut f64, t: &mut f64, k: &mut usize) -> bool {
        let (c, r) = self.bounds();
        let v = [q[0] - c[0], q[1] - c[1], q[2] - c[2]];
        match &self.shape {
            Shape::Polyline(p) => {
                if (v[0] * v[0] + v[2] * v[2] + v[1] * v[1]).sqrt() > r + *best {
                    return false;
                }
                // the sums of squares as (y + z) + x and the dot product as (z + y) + x (0x48cf68)
                let mut found = false;
                let mut d0 = [q[0] - p[0][0], q[1] - p[0][1], q[2] - p[0][2]];
                let mut dd0 = d0[1] * d0[1] + d0[2] * d0[2] + d0[0] * d0[0];
                for i in 0..p.len() - 1 {
                    let d1 = [q[0] - p[i + 1][0], q[1] - p[i + 1][1], q[2] - p[i + 1][2]];
                    let dd1 = d1[1] * d1[1] + d1[2] * d1[2] + d1[0] * d1[0];
                    let e = [p[i + 1][0] - p[i][0], p[i + 1][1] - p[i][1], p[i + 1][2] - p[i][2]];
                    let ee = e[1] * e[1] + e[2] * e[2] + e[0] * e[0];
                    if dd1 < ee + dd0 {
                        if dd0 < ee + dd1 {
                            let pr = e[2] * d0[2] + e[1] * d0[1] + e[0] * d0[0];
                            let pp = pr * pr / ee;
                            let dist = dd0 - pp;
                            if dist < *best {
                                *best = dist;
                                found = true;
                                *k = i;
                                *t = (pp / ee).sqrt();
                            }
                        } else if dd1 < *best {
                            found = true;
                            *best = dd1;
                            *k = i;
                            *t = 1.0;
                        }
                    } else if dd0 < *best {
                        *best = dd0;
                        found = true;
                        *k = i;
                        *t = 0.0;
                    }
                    d0 = d1;
                    dd0 = dd1;
                }
                found
            }
            Shape::Curve { pos, .. } => {
                let dy = c[1] - q[1];
                if ((c[0] - q[0]) * (c[0] - q[0]) + (c[2] - q[2]) * (c[2] - q[2]) + dy * dy).sqrt() > r + *best {
                    return false;
                }
                // 0x48e470: |P(t) - q|^2, the squares of the coordinates added (z + y) + x
                let comp = |i: usize| -> Vec<f64> {
                    let mut d: Vec<f64> = pos.iter().map(|c| c[i]).collect();
                    d[3] = d[3] + -q[i];
                    mul(&d, &d)
                };
                let (x, y, z) = (comp(0), comp(1), comp(2));
                let f: Vec<f64> = (0..x.len()).map(|n| z[n] + y[n] + x[n]).collect();
                let deriv = |c: &[f64]| -> Vec<f64> { (0..c.len() - 1).map(|i| (c.len() - 1 - i) as f64 * c[i]).collect() };
                let f1 = deriv(&f);
                let f2 = deriv(&f1);
                // 0x48e700: from t = 0, 0.5 and 1, three Newton steps on f'
                let mut found = false;
                for i in 0..3 {
                    let mut tt = (i as f64 * (1.0 - 0.0)) / 2.0 + 0.0;
                    let v = eval(&f, tt);
                    if v < *best {
                        *best = v;
                        found = true;
                        *t = tt;
                    }
                    for _ in 0..3 {
                        tt = tt - eval(&f1, tt) / eval(&f2, tt);
                        let v = eval(&f, tt);
                        if v < *best && tt <= 1.0 && 0.0 <= tt {
                            *best = v;
                            found = true;
                            *t = tt;
                        }
                    }
                }
                if found {
                    *k = 0;
                }
                found
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

impl RoadNet {
    /// The flags of segment `s` now (see `seg_flags`).
    pub fn flags_of(&self, s: usize) -> u32 {
        self.seg_flags.borrow().get(s).copied().unwrap_or_else(|| self.segments[s].flags)
    }

    /// The box around road `r` (`road + 0x88` the lowest corner, `+0xa0` the highest; `0x406160`):
    /// the box around the points `vtable + 0x14` steps through on each segment from its start,
    /// which is not one of them (`0x48d530`: the further points of a polyline; `0x48e100`: a
    /// curve at t = 0.05001, 0.10002 .. 1, the sum kept as it adds up).
    pub fn road_box(&self, r: usize) -> [[f64; 3]; 2] {
        let road = &self.roads[r];
        let mut b = [[f64::MAX; 3], [-f64::MAX; 3]];
        let mut widen = |p: [f64; 3]| {
            for c in 0..3 {
                if p[c] < b[0][c] {
                    b[0][c] = p[c];
                }
                if b[1][c] < p[c] {
                    b[1][c] = p[c];
                }
            }
        };
        for &s in &road.segments {
            let seg = &self.segments[s];
            match &seg.shape {
                Shape::Polyline(p) => {
                    for k in 1..p.len() {
                        widen(p[k]);
                    }
                }
                Shape::Curve { .. } => {
                    let mut t = 0.0f64;
                    while t < 1.0 {
                        t += 0.05001;
                        if 1.0 <= t {
                            t = 1.0;
                        }
                        widen(seg.point(t, 0));
                    }
                }
            }
        }
        b
    }

    /// Has the human player driven road `r` (see `explored`)?
    pub fn is_explored(&self, r: usize) -> bool {
        self.explored.borrow().get(r).copied().unwrap_or(false)
    }

    /// Mark road `r` as driven by the human player.
    pub fn set_explored(&self, r: usize) {
        let mut e = self.explored.borrow_mut();
        if e.len() <= r {
            e.resize(self.roads.len().max(r + 1), false);
        }
        e[r] = true;
    }

    /// The segment flags at a place as `getRoadParameters` (`0x438d70`) gives them: without
    /// 0x20 on a road the human player has driven.
    pub fn place_flags(&self, p: &Place) -> u32 {
        let f = self.flags_of(self.roads[p.road].segments[p.segment]);
        if self.is_explored(p.road) { f & !0x20 } else { f }
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

    /// The `.way` room of the segment a place is on.
    pub fn room_of(&self, p: &Place) -> &str {
        &self.segment_rooms[self.roads[p.road].segments[p.segment]]
    }

    /// `0x406370`: the roads of a room: those with a segment in it and those at the nodes where
    /// these start or end. The original sorts them by the addresses of the road objects; here by
    /// number (stand-in), which matters only between equally near places.
    pub fn room_roads(&self, room: &str) -> Vec<usize> {
        let mut list: Vec<usize> = Vec::new();
        for (r, road) in self.roads.iter().enumerate() {
            for &s in &road.segments {
                if self.segment_rooms[s] == room {
                    list.push(r);
                }
            }
        }
        let mut more: Vec<usize> = Vec::new();
        for &r in &list {
            for n in [self.roads[r].start, self.roads[r].end] {
                more.extend(&self.nodes[n].out);
                more.extend(&self.nodes[n].inc);
            }
        }
        list.extend(more);
        list.sort_unstable();
        list.dedup();
        list
    }

    /// `getPositionIdEx` (`0x4365d0`): the place nearest to `q` on the roads of a room (each
    /// segment's `project` in turn, the last one that came nearer wins).
    pub fn nearest_in_room(&self, room: &str, q: [f64; 3]) -> Option<Place> {
        let (mut best, mut t, mut k) = (f64::MAX, 0.0, 0usize);
        let mut found = None;
        for road in self.room_roads(room) {
            for (i, &s) in self.roads[road].segments.iter().enumerate() {
                if self.segments[s].project(q, &mut best, &mut t, &mut k) {
                    found = Some((road, i));
                }
            }
        }
        found.map(|(road, segment)| Place { road, node: None, t, piece: k, segment, reversed: false })
    }

    /// `getPositionId` (`0x436900`): the place nearest to `q` on all the roads (each segment's
    /// `project` in turn, the last one that came nearer wins).
    pub fn nearest(&self, q: [f64; 3]) -> Option<Place> {
        let (mut best, mut t, mut k) = (f64::MAX, 0.0, 0usize);
        let mut found = None;
        for (road, r) in self.roads.iter().enumerate() {
            for (i, &s) in r.segments.iter().enumerate() {
                if self.segments[s].project(q, &mut best, &mut t, &mut k) {
                    found = Some((road, i));
                }
            }
        }
        found.map(|(road, segment)| Place { road, node: None, t, piece: k, segment, reversed: false })
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

    /// The tangent the way a place faces: against the road when the place is reversed, and
    /// turned round again when `back`.
    pub fn direction(&self, p: &Place, back: bool) -> [f64; 3] {
        let t = self.tangent(p);
        if back != p.reversed {
            [-t[0], -t[1], -t[2]]
        } else {
            t
        }
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

    /// `setPlayerPosition` (`0x43dc60`): the place of the point `q` on a route: the nearest point
    /// on the roads of the route (on the road of the current place first, its distance counting
    /// 1.5 times plus 10 so that the vehicle keeps to it), facing the way the route takes that
    /// road; the destination node when within 0.1 of it. A route of one step puts the player on
    /// its node.
    pub fn locate(&self, route: &[RouteStep], q: [f64; 3], current: &Spot) -> Spot {
        if route.len() == 1 {
            return Spot::Node(route[0].node);
        }
        let (mut best, mut t, mut k) = (f64::MAX, 0.0, 0usize);
        let mut result: Option<Place> = None;
        if let Spot::Road(cur) = current {
            if route.iter().any(|s| s.road == Some(cur.road)) {
                let mut seg = 0;
                for (i, &si) in self.roads[cur.road].segments.iter().enumerate() {
                    if self.segments[si].project(q, &mut best, &mut t, &mut k) {
                        seg = i;
                    }
                }
                result = Some(Place { road: cur.road, node: None, t, piece: k, segment: seg, reversed: false });
                best = best * 1.5 + 10.0;
            }
        }
        loop {
            let mut found = false;
            for step in route {
                let Some(road) = step.road else { continue };
                let mut seg = None;
                for (i, &si) in self.roads[road].segments.iter().enumerate() {
                    if self.segments[si].project(q, &mut best, &mut t, &mut k) {
                        seg = Some(i);
                    }
                }
                if let Some(seg) = seg {
                    let reversed = step.node == self.roads[road].end;
                    result = Some(Place { road, node: None, t, piece: k, segment: seg, reversed });
                    found = true;
                }
            }
            if found || route.is_empty() {
                break;
            }
            best = f64::MAX;
            t = 0.0;
            k = 0;
        }
        let Some(mut place) = result.or_else(|| self.nearest(q)) else { return current.clone() };
        if let Some(last) = route.last() {
            let c = self.coordinates(&place);
            let n = self.nodes[last.node].pos;
            let d = [c[0] - n[0], c[1] - n[1], c[2] - n[2]];
            if (d[2] * d[2] + d[1] * d[1] + d[0] * d[0]).sqrt() < 0.1 {
                place.node = Some(last.node);
            }
        }
        Spot::Road(place)
    }

    /// `setPosition` (`0x474290`): a player put at `new` keeps the node it had (`+0x24` of its
    /// place) when the new point is within 1 mm of that node.
    pub fn keep_node(&self, old: Option<&Spot>, new: Spot) -> Spot {
        let Spot::Road(mut place) = new else { return new };
        let node = match old {
            Some(Spot::Node(n)) => Some(*n),
            Some(Spot::Road(q)) => q.node,
            None => None,
        };
        if let Some(n) = node {
            let c = self.coordinates(&place);
            let q = self.nodes[n].pos;
            let (x, y, z) = (q[0] - c[0], q[1] - c[1], q[2] - c[2]);
            if (z * z + y * y + x * x).sqrt() < 0.001 {
                place.node = Some(n);
            }
        }
        Spot::Road(place)
    }

    /// The coordinates of a spot (`0x435bf0`).
    pub fn spot_coordinates(&self, s: &Spot) -> [f64; 3] {
        match s {
            Spot::Node(n) => self.nodes[*n].pos,
            Spot::Road(p) => self.coordinates(p),
        }
    }

    /// `0x4801f0`: a random place on the network: a distance along all roads drawn in
    /// centimetres (less one metre, not below 0), on the road it falls in, reversed when that
    /// segment is one-way against the road (flag 8, `0x438d70`).
    pub fn random_place(&self, rng: &mut AiRng) -> Option<Place> {
        let last = self.roads.last()?;
        let total = (last.cumulative * 100.0) as u32;
        let d = ((rng.next() % total) as f64 * 0.01 - 1.0).max(0.0);
        let road = self.roads.iter().position(|r| d < r.cumulative)?;
        let r = &self.roads[road];
        let mut off = d - r.cumulative + r.length;
        let mut p = Place { road, node: Some(r.start), t: 0.0, piece: 0, segment: 0, reversed: false };
        if 0.0 <= off {
            self.advance(&mut p, &mut off);
            if self.place_flags(&p) & 8 != 0 {
                p.reversed = true;
            }
        }
        Some(p)
    }

    /// `0x422d60`: a random place to put a vehicle: [`RoadNet::random_place`] again while the
    /// segment there has flag 0x20 or 0x40.
    pub fn random_start(&self, rng: &mut AiRng) -> Option<Place> {
        loop {
            let p = self.random_place(rng)?;
            if self.place_flags(&p) & 0x60 == 0 {
                return Some(p);
            }
        }
    }

    /// `0x4cda20`: the horizontal direction of the road at a place (against the road when
    /// `back`), and the slope angle of the road there.
    pub fn heading(&self, p: &Place, back: bool) -> ([f64; 3], f64) {
        let t = self.direction(p, back);
        let slope = norm(t)[2].asin();
        (norm([t[0], t[1], 0.0]), slope)
    }

    /// `0x4cd770`: the signed sideways distance of the point `q` from the centre line of the
    /// road at a place (positive to the left of the direction of travel), half the road width,
    /// the speed value of the segment (its `ATTR` real times 10) and the segment flags.
    pub fn lateral(&self, p: &Place, back: bool, q: [f64; 3]) -> (f64, f64, f64, u32) {
        let c = self.coordinates(p);
        let d = [q[0] - c[0], q[1] - c[1], 0.0];
        let t = self.direction(p, back);
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

/// A step of a route (12-byte entries of the original's route list): at `node`, take `road`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RouteStep {
    pub node: usize,
    pub road: Option<usize>,
}

/// Where a route starts or ends (`PositionId`): a node, or a place on a road.
#[derive(Debug, Clone, Copy)]
pub enum Spot {
    Node(usize),
    Road(Place),
}

impl RoadNet {
    /// The place at the start of `road`, facing along it (`0x47ff20`).
    pub fn road_start(&self, road: usize) -> Place {
        Place { road, node: Some(self.roads[road].start), t: 0.0, piece: 0, segment: 0, reversed: false }
    }

    /// The place at the end of `road`, facing back along it (`0x4800a0`).
    pub fn road_end(&self, road: usize) -> Place {
        let r = &self.roads[road];
        let segment = r.segments.len() - 1;
        let piece = self.segments[r.segments[segment]].pieces() - 1;
        Place { road, node: Some(r.end), t: 1.0, piece, segment, reversed: true }
    }

    /// `advancePosition(PlayerId, ...)` (`0x43b570`): move `dist` metres along a route. At a
    /// node the route says which road to take next; the place continues on it from the end
    /// that touches the node. Stops (keeping the rest of `dist`) at a node the route does not
    /// continue from; a step back onto the same road ends the move.
    ///
    /// The step of the node is looked for from the start of the route each time (the position's
    /// step hint is only used when it is past the end, so never); when that is the first step
    /// and the node comes again later, the later one is taken. Moving back, the road of the
    /// step before is taken (none before the first step).
    pub fn advance_route(&self, p: &mut Place, dist: &mut f64, route: &[RouteStep]) {
        let back = *dist <= 0.0;
        loop {
            self.advance(p, dist);
            let (Some(node), true) = (p.node, *dist != 0.0) else { return };
            let Some(mut k) = route.iter().position(|s| s.node == node) else { return };
            if k == 0 {
                if let Some(j) = route[1..].iter().position(|s| s.node == node) {
                    k = j + 1;
                }
            }
            let step = if back {
                if k == 0 {
                    return;
                }
                k - 1
            } else {
                k
            };
            let Some(road) = route[step].road else { return };
            if road == p.road {
                *dist = 0.0;
                return;
            }
            let r = &self.roads[road];
            // The new place faces the way the move goes on: away from the node.
            let back = *dist < 0.0;
            *p = if r.start == node {
                self.road_start(road)
            } else if r.end == node {
                self.road_end(road)
            } else {
                return;
            };
            p.reversed ^= back;
        }
    }

    /// `0x4cdaf0`: how fast the road turns ahead: the angle between the direction at `p` and
    /// `dist` metres further along the route, per metre actually travelled (at least 0.1),
    /// positive for a left turn.
    pub fn curvature(&self, p: &Place, dist: f64, back: bool, route: &[RouteStep]) -> f64 {
        let mut q = *p;
        let mut left = if back { -dist } else { dist };
        self.advance_route(&mut q, &mut left, route);
        let travelled = (if back { -dist } else { dist }) - left;
        let (a, _) = self.heading(p, back);
        let (b, _) = self.heading(&q, back);
        let mut ang = (a[0] * b[0] + a[1] * b[1] + a[2] * b[2]).clamp(-1.0, 1.0).acos();
        if a[0] * b[1] - a[1] * b[0] < 0.0 {
            ang = -ang;
        }
        let d = travelled.abs().max(0.1);
        ang / d
    }
}

#[cfg(test)]
mod net_tests {
    use super::*;
    use ht2_formats::way;

    fn seg(a: [f64; 3], b: [f64; 3]) -> way::Segment {
        way::Segment { kind: 0, value: 0.5, flag: 1, width: [8.0, 8.0], points: vec![a, b] }
    }

    #[test]
    fn route_crosses_a_node() {
        // a T junction: (0,0) - (10,0) - (20,0) and (10,0) - (10,10)
        let room = way::Room {
            name: "r".into(),
            nodes: vec![],
            segments: vec![seg([0.0, 0.0, 0.0], [10.0, 0.0, 0.0]), seg([10.0, 0.0, 0.0], [20.0, 0.0, 0.0]), seg([10.0, 10.0, 0.0], [10.0, 0.0, 0.0])],
        };
        let net = RoadNet::build(&[vec![room]]);
        assert_eq!(net.roads.len(), 3);
        let first = net.roads.iter().position(|r| net.nodes[r.start].pos == [0.0, 0.0, 0.0] || net.nodes[r.end].pos == [0.0, 0.0, 0.0]).unwrap();
        let up = net.roads.iter().position(|r| net.nodes[r.start].pos == [10.0, 10.0, 0.0] || net.nodes[r.end].pos == [10.0, 10.0, 0.0]).unwrap();
        let mut p = if net.nodes[net.roads[first].start].pos == [0.0, 0.0, 0.0] { net.road_start(first) } else { net.road_end(first) };
        p.node = None;
        let mid = net.nodes.iter().position(|n| n.pos == [10.0, 0.0, 0.0]).unwrap();
        let mut d = 14.0;
        net.advance_route(&mut p, &mut d, &[RouteStep { node: mid, road: Some(up) }]);
        let c = net.coordinates(&p);
        assert!((c[0] - 10.0).abs() < 1e-9 && (c[1] - 4.0).abs() < 1e-9, "{c:?}");
        // turning from the first road onto the side road is a quarter turn
        let k = net.curvature(&net.road_start(first), 14.0, false, &[RouteStep { node: mid, road: Some(up) }]);
        assert!((k.abs() * 14.0 - std::f64::consts::FRAC_PI_2).abs() < 1e-9 || k == 0.0, "{k}");
    }
}

impl RoadNet {
    /// `0x409980`: what it costs to drive a road (against its direction when `against`): the
    /// length of each segment, half as much again on `attr2` 2 roads and double on 3, and for
    /// `mode` other than 1 ten thousand times on segments flagged 0x40 or one-way the other way.
    pub fn road_cost(&self, mode: u32, road: usize, against: bool) -> f64 {
        let slot = (mode as usize) * 2 + against as usize;
        if let Some(c) = self.cost_cache.borrow().get(road).and_then(|c| c.get(slot).copied().flatten()) {
            return c;
        }
        let total = self.road_cost_now(mode, road, against);
        let mut cache = self.cost_cache.borrow_mut();
        if cache.len() < self.roads.len() {
            cache.resize(self.roads.len(), [None; 4]);
        }
        if slot < 4 {
            cache[road][slot] = Some(total);
        }
        total
    }

    /// The cost of a road from the flags of its segments now (the body of `0x409980`).
    pub fn road_cost_now(&self, mode: u32, road: usize, against: bool) -> f64 {
        let mut total = 0.0;
        for &s in &self.roads[road].segments {
            let seg = &self.segments[s];
            let flags = self.flags_of(s);
            let mut k = match seg.attr2 {
                2 => 1.5,
                3 => 2.0,
                _ => 1.0,
            };
            if mode != 1 && (flags & 0x40 != 0 || flags & if against { 4 } else { 8 } != 0) {
                k *= 10000.0;
            }
            total += seg.length() * k;
        }
        total
    }

    /// `0x484c80`: the cheapest route from node `src` to node `dst` (Dijkstra over the nodes:
    /// the open node with the least cost, the first one in node order on a tie; from each, the
    /// roads leaving it, then those arriving). The route lists, for each node on the way, the
    /// road taken there, and ends with `dst`. `None` when `dst` cannot be reached.
    /// The next-step entry of `node` toward `dst` for `mode`.
    fn step_entry(&self, mode: u32, node: usize, dst: usize) -> u8 {
        let t = self.next_step.borrow();
        t.get(mode as usize).and_then(|m| m.get(node)).map_or(0xff, |r| r[dst])
    }

    /// `0x484140`: forget what the route searches of `mode` found (after the roads changed).
    pub fn clear_steps(&self, mode: u32) {
        if let Some(m) = self.next_step.borrow_mut().get_mut(mode as usize) {
            m.clear();
        }
        // and the costs of the roads for the mode
        for c in self.cost_cache.borrow_mut().iter_mut() {
            for slot in [mode as usize * 2, mode as usize * 2 + 1] {
                if let Some(x) = c.get_mut(slot) {
                    *x = None;
                }
            }
        }
    }

    fn set_step_entry(&self, mode: u32, node: usize, dst: usize, v: u8) {
        let mut t = self.next_step.borrow_mut();
        let n = self.nodes.len();
        while t.len() <= mode as usize {
            t.push(Vec::new());
        }
        let m = &mut t[mode as usize];
        if m.is_empty() {
            *m = vec![vec![0xff; n]; n];
        }
        m[node][dst] = v;
    }

    /// The road a next-step entry stands for, and whether it is driven against its direction.
    fn entry_road(&self, node: usize, e: u8) -> (usize, bool) {
        let against = e & 0x80 != 0;
        let list = if against { &self.nodes[node].inc } else { &self.nodes[node].out };
        (list[(e & 0x7f) as usize], against)
    }

    /// `0x484be0`: remember along a found path that from each node on it the next road leads to
    /// every later node of it.
    fn remember(&self, mode: u32, path: &[(usize, u8)]) {
        for j in 1..path.len() {
            for i in 0..j {
                self.set_step_entry(mode, path[i].0, path[j].0, path[i].1);
            }
        }
    }

    /// `0x484c80` (`ai_trce.cpp`): the cheapest route between two nodes. A route known from
    /// earlier searches is followed (`0x484810`); otherwise Dijkstra: the open nodes kept in order of
    /// number, the first cheapest taken, from it the leaving roads, then the arriving ones (cost
    /// `0x409980`); the paths to the nodes taken, and to the destination, are remembered.
    pub fn route_between(&self, src: usize, dst: usize, mode: u32) -> Option<(Vec<RouteStep>, f64)> {
        if src == dst {
            return Some((vec![RouteStep { node: dst, road: None }], 0.0));
        }
        if self.step_entry(mode, src, dst) != 0xff {
            // 0x484810
            let mut route = Vec::new();
            let mut cost = 0.0;
            let mut node = src;
            loop {
                let e = self.step_entry(mode, node, dst);
                if e == 0x7f || e == 0xff {
                    return None;
                }
                let (road, against) = self.entry_road(node, e);
                route.push(RouteStep { node, road: Some(road) });
                cost = self.road_cost(mode, road, against) + cost;
                let r = &self.roads[road];
                node = if against { r.start } else { r.end };
                if node == dst {
                    break;
                }
            }
            route.push(RouteStep { node: dst, road: None });
            return Some((route, cost));
        }
        let n = self.nodes.len();
        let mut cost = vec![f64::MAX; n];
        let mut done = vec![false; n];
        let mut path: Vec<Vec<(usize, u8)>> = vec![Vec::new(); n];
        let mut via: Vec<Vec<RouteStep>> = vec![Vec::new(); n];
        cost[src] = 0.0;
        let mut open: Vec<usize> = (0..n).collect();
        while !open.is_empty() {
            let mut best = 0;
            for k in 1..open.len() {
                if cost[open[k]] < cost[open[best]] {
                    best = k;
                }
            }
            let cur = open[best];
            if f64::MAX <= cost[cur] {
                self.set_step_entry(mode, src, dst, 0x7f);
                return None;
            }
            if cur == dst {
                let mut route = std::mem::take(&mut via[cur]);
                route.push(RouteStep { node: dst, road: None });
                let mut p = std::mem::take(&mut path[cur]);
                p.push((dst, 0xff));
                self.remember(mode, &p);
                return Some((route, cost[cur]));
            }
            open.remove(best);
            done[cur] = true;
            self.remember(mode, &path[cur]);
            for (against, roads) in [(false, &self.nodes[cur].out), (true, &self.nodes[cur].inc)] {
                for (k, &road) in roads.iter().enumerate() {
                    let r = &self.roads[road];
                    let other = if against { r.start } else { r.end };
                    if done[other] {
                        continue;
                    }
                    let c = self.road_cost(mode, road, against) + cost[cur];
                    if c < cost[other] {
                        cost[other] = c;
                        let mut p = path[cur].clone();
                        p.push((cur, (against as u8) << 7 | k as u8));
                        path[other] = p;
                        let mut v = via[cur].clone();
                        v.push(RouteStep { node: cur, road: Some(road) });
                        via[other] = v;
                    }
                }
            }
        }
        self.set_step_entry(mode, src, dst, 0x7f);
        None
    }
}

/// The random generator of the road module (`0x68b9c8`): a subtractive generator of 55 words
/// (the STL's `subtractive_rng`), seeded with 161803398 when the module starts (`0x403940`).
#[derive(Debug, Clone)]
pub struct AiRng {
    table: [u32; 55],
    i: usize,
    j: usize,
}

impl Default for AiRng {
    fn default() -> Self {
        Self::new(161_803_398)
    }
}

impl AiRng {
    /// `0x403940`.
    pub fn new(seed: u32) -> Self {
        let mut table = [0u32; 55];
        table[54] = seed;
        let (mut k, mut s) = (1u32, seed);
        let mut e = 0usize;
        while e < 0x46e {
            s = s.wrapping_sub(k);
            let idx = e % 55;
            e += 21;
            table[idx] = k;
            k = s;
            s = table[idx];
        }
        for _ in 0..4 {
            for c in 0..55 {
                table[c] = table[c].wrapping_sub(table[(c + 31) % 55]);
            }
        }
        AiRng { table, i: 0, j: 31 }
    }

    /// A generator in a given state: the table and the two indices (`0x68baa4`, `0x68baa8`).
    pub fn from_state(table: [u32; 55], i: usize, j: usize) -> Self {
        AiRng { table, i, j }
    }

    /// The next number.
    pub fn next(&mut self) -> u32 {
        self.i = (self.i + 1) % 55;
        self.j = (self.j + 1) % 55;
        self.table[self.i] = self.table[self.i].wrapping_sub(self.table[self.j]);
        self.table[self.i]
    }
}

impl RoadNet {
    /// `0x433f90` (new game): mark the nodes of the towns: the town and slot, and flag 2 for the
    /// unloading node of a town (`unload`), flag 1 for its other nodes except `dummy` ones. Town
    /// nodes are found by name (`0x436510`); names that are not nodes are skipped.
    pub fn mark_towns(&mut self, towns: &[Town]) {
        for (ti, town) in towns.iter().enumerate() {
            for (k, (name, attrs)) in town.nodes.iter().enumerate() {
                let Some(n) = self.nodes.iter().position(|n| &n.name == name) else { continue };
                let node = &mut self.nodes[n];
                node.town = Some((ti, k));
                if attrs & 1 != 0 {
                    node.flags |= 2;
                } else if attrs & 8 == 0 {
                    node.flags |= 1;
                }
            }
        }
    }

    /// `0x404e90`: the node a town is reached at: its last `unload` node (`+0x40`, set by
    /// `0x4305f0`), else its first node that is on the network.
    pub fn town_node(&self, town: &Town) -> Option<usize> {
        let find = |name: &String| self.nodes.iter().position(|n| &n.name == name);
        match town.nodes.iter().rev().find(|(_, a)| a & 1 != 0) {
            Some((n, _)) => find(n),
            None => town.nodes.iter().find_map(|(n, _)| find(n)),
        }
    }

    /// `0x4340f0` (new game): mark the nodes of the parking places: flag 4, the parking place
    /// (`+0xac`) and the slot (`+0xb4`).
    pub fn mark_parkings(&mut self, lots: &[Parking]) {
        for (li, lot) in lots.iter().enumerate() {
            for (k, (name, _, _)) in lot.nodes.iter().enumerate() {
                let Some(n) = self.nodes.iter().position(|n| &n.name == name) else { continue };
                self.nodes[n].flags |= 4;
                self.nodes[n].parking = Some((li, k));
            }
        }
    }

    /// `0x487590` with mode 0: the cost of the way from `a` to `b`: the cheapest route, or the
    /// cheapest by mode 1 (which ignores the one-way and 0x40 penalties) when that is under a
    /// tenth of it.
    pub fn trip_cost(&self, a: usize, b: usize) -> f64 {
        let cost = |mode| self.route_between(a, b, mode).map_or(0.0, |(_, c)| c);
        let (c0, c1) = (cost(0), cost(1));
        if c1 * 10.0 < c0 { c1 } else { c0 }
    }

    /// `0x414670` (new game): the cost of the way between every two towns (`town + 0x64`), 0 for
    /// a town to itself or when either has no node.
    pub fn town_distances(&self, towns: &[Town]) -> Vec<Vec<f64>> {
        let nodes: Vec<Option<usize>> = towns.iter().map(|t| self.town_node(t)).collect();
        nodes
            .iter()
            .map(|&a| {
                nodes
                    .iter()
                    .map(|&b| match (a, b) {
                        (Some(a), Some(b)) if a != b => self.trip_cost(a, b),
                        _ => 0.0,
                    })
                    .collect()
            })
            .collect()
    }

    /// The factor of a segment in route costs (`0x409f10`): 1.5 for `attr2` 2, 2 for 3, else 1;
    /// for modes other than 1, times 10 000 on segments with flag 0x40 and on one-way segments
    /// driven the other way (flag 8 along the road, flag 4 against it).
    fn segment_factor(seg: &Segment, flags: u32, mode: u32, against: bool) -> f64 {
        let k = match seg.attr2 {
            2 => 1.5,
            3 => 2.0,
            _ => 1.0,
        };
        if mode != 1 && (flags & 0x40 != 0 || flags & if against { 4 } else { 8 } != 0) {
            k * 10000.0
        } else {
            k
        }
    }

    /// A spot as a place on `road`: (segment, piece, t, node). A node at either end of the road
    /// becomes its start (`0x47fcf0`) or its end (`0x47fe20`: one segment past the last).
    fn on_road(&self, road: usize, spot: &Spot) -> Option<(usize, usize, f64, Option<usize>)> {
        let r = &self.roads[road];
        match spot {
            Spot::Road(p) => Some((p.segment, p.piece, p.t, p.node)),
            Spot::Node(n) if *n == r.start => Some((0, 0, 0.0, Some(*n))),
            Spot::Node(n) if *n == r.end => Some((r.segments.len(), 0, 0.0, Some(*n))),
            Spot::Node(_) => None,
        }
    }

    /// `0x402740`: whether the spot `a` comes before `b` on `road` (a spot at the end node is
    /// last, at the start node first).
    fn before(&self, road: usize, a: &Spot, b: &Spot) -> bool {
        let r = &self.roads[road];
        let (Some(pa), Some(pb)) = (self.on_road(road, a), self.on_road(road, b)) else { return false };
        if pa == pb {
            return false;
        }
        if pb.3 == Some(r.end) {
            return true;
        }
        if pb.3 == Some(r.start) {
            return false;
        }
        if pa.3 == Some(r.start) {
            return true;
        }
        if pa.3 == Some(r.end) {
            return false;
        }
        (pa.0, pa.1) < (pb.0, pb.1) || (pa.0, pa.1) == (pb.0, pb.1) && pa.2 < pb.2
    }

    /// `0x409aa0`: the cost of driving along `road` from `a` to `b` (against the road when `b`
    /// comes first): the distance on each segment times its factor.
    pub fn partial_cost(&self, mode: u32, road: usize, a: &Spot, b: &Spot) -> f64 {
        let forward = self.before(road, a, b);
        let (Some(pa), Some(pb)) = (self.on_road(road, a), self.on_road(road, b)) else { return 0.0 };
        let (from, to) = if forward { (pa, pb) } else { (pb, pa) };
        let r = &self.roads[road];
        let seg = |i: usize| &self.segments[r.segments[i]];
        let fl = |i: usize| self.flags_of(r.segments[i]);
        // a place one segment past the last is the end of the last one
        let (mut s2, mut k2, mut t2) = (to.0, to.1, to.2);
        if s2 == r.segments.len() {
            s2 -= 1;
            k2 = seg(s2).pieces();
            t2 = 0.0;
        }
        if from.0 == s2 {
            return Self::segment_factor(seg(s2), fl(s2), mode, !forward) * seg(s2).distance(from.2, from.1, t2, k2);
        }
        let first = seg(from.0);
        let mut sum = first.distance(from.2, from.1, 0.0, first.pieces()) * Self::segment_factor(first, fl(from.0), mode, !forward);
        for i in from.0 + 1..s2 {
            sum += Self::segment_factor(seg(i), fl(i), mode, !forward) * seg(i).length();
        }
        Self::segment_factor(seg(s2), fl(s2), mode, !forward) * seg(s2).distance(0.0, 0, t2, k2) + sum
    }

    /// `0x409660`: the distance along `road` between two spots on it.
    fn along(&self, road: usize, a: &Spot, b: &Spot) -> f64 {
        self.partial_cost(1, road, a, b)
    }

    /// `0x4858b0` (`ai_trce.cpp`): the cheapest route between two spots, each a node or a place on
    /// a road. From a place, both ends of its road are tried, with the cost of getting there along
    /// the road (a place within 0.1 of an end counts as that node); the same for a place to reach.
    /// When both are on one road, driving straight along it is tried first. A route from a place
    /// starts with the far end of its road (the way it is driven along it), one to a place ends
    /// with the far end of the road it is on.
    pub fn route_from(&self, from: &Spot, to: &Spot, mode: u32) -> Option<(Vec<RouteStep>, f64)> {
        if let (Spot::Node(a), Spot::Node(b)) = (from, to) {
            return self.route_between(*a, *b, mode);
        }
        // the ends a route may leave from / arrive at, with their costs and the step before /
        // after the part on the network
        let mut starts: Vec<(usize, f64, Option<RouteStep>)> = Vec::new();
        match from {
            Spot::Node(n) => starts.push((*n, 0.0, None)),
            Spot::Road(p) => {
                let r = &self.roads[p.road];
                let c = self.partial_cost(mode, p.road, from, &Spot::Node(r.start));
                if c < 0.1 {
                    return self.route_from(&Spot::Node(r.start), to, mode);
                }
                starts.push((r.start, c, Some(RouteStep { node: r.end, road: Some(p.road) })));
                let c = self.partial_cost(mode, p.road, from, &Spot::Node(r.end));
                if c < 0.1 {
                    return self.route_from(&Spot::Node(r.end), to, mode);
                }
                starts.push((r.end, c, Some(RouteStep { node: r.start, road: Some(p.road) })));
            }
        }
        let mut ends: Vec<(usize, f64, Option<RouteStep>)> = Vec::new();
        let mut last_road = None;
        match to {
            Spot::Node(n) => ends.push((*n, 0.0, None)),
            Spot::Road(p) => {
                let r = &self.roads[p.road];
                last_road = Some(p.road);
                let c = self.partial_cost(mode, p.road, &Spot::Node(r.start), to);
                if c < 0.1 {
                    return self.route_from(from, &Spot::Node(r.start), mode);
                }
                ends.push((r.start, c, Some(RouteStep { node: r.end, road: None })));
                let c = self.partial_cost(mode, p.road, &Spot::Node(r.end), to);
                if c < 0.1 {
                    return self.route_from(from, &Spot::Node(r.end), mode);
                }
                ends.push((r.end, c, Some(RouteStep { node: r.start, road: None })));
            }
        }
        let mut best = f64::MAX;
        let mut route = Vec::new();
        if let (Spot::Road(a), Spot::Road(b)) = (from, to) {
            if a.road == b.road {
                let r = &self.roads[a.road];
                best = self.partial_cost(mode, a.road, from, to);
                let end = Spot::Node(r.end);
                route = if self.along(a.road, &end, from) <= self.along(a.road, &end, to) {
                    vec![RouteStep { node: r.end, road: Some(a.road) }, RouteStep { node: r.start, road: None }]
                } else {
                    vec![RouteStep { node: r.start, road: Some(a.road) }, RouteStep { node: r.end, road: None }]
                };
            }
        }
        for (s, cs, before) in &starts {
            for (e, ce, after) in &ends {
                let Some((steps, c)) = self.route_between(*s, *e, mode) else { continue };
                let total = c + cs + ce;
                if total < best {
                    best = total;
                    route.clear();
                    route.extend(*before);
                    route.extend(steps);
                    if let Some(after) = after {
                        if let Some(l) = route.last_mut() {
                            l.road = last_road;
                        }
                        route.push(*after);
                    }
                }
            }
        }
        (!route.is_empty()).then_some((route, best))
    }

    /// `0x456140` / `0x45cdc0`: a random destination node: among the nodes that are not hidden
    /// (flag 8), those marked as destinations (`FLAG` bit 1, flag 0x40) for players of type other
    /// than 2; for type 2 those with flag 2 (when `which` bit 0) or flag 1. `None` when there are
    /// none (then nothing is drawn).
    pub fn random_destination(&self, rng: &mut AiRng, which: u32, player_type: u32) -> Option<usize> {
        let picks: Vec<usize> = (0..self.nodes.len())
            .filter(|&k| {
                let f = self.nodes[k].flags;
                if f & 8 != 0 {
                    return false;
                }
                if player_type != 2 {
                    f & 0x40 != 0
                } else if which & 1 != 0 {
                    f & 2 != 0
                } else {
                    f & 1 != 0
                }
            })
            .collect();
        if picks.is_empty() {
            return None;
        }
        Some(picks[rng.next() as usize % picks.len()])
    }
}

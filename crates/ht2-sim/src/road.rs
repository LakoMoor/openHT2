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

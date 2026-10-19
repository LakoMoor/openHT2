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
    Curve { pos: [[f64; 3]; 4], tangent: [[f64; 3]; 3] },
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
            Shape::Curve { pos, tangent }
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
    }

    #[test]
    fn polyline_drops_repeated_points() {
        let p = [[0.0, 0.0, 0.0], [0.0, 0.0, 0.0], [1.0, 0.0, 0.0]];
        let s = Segment::new(0, 0.0, 1, [6.0, 6.0], &p).unwrap();
        assert_eq!(s.pieces(), 1);
        assert_eq!(s.point(1.0, 0), [1.0, 0.0, 0.0]);
    }
}

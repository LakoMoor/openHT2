//! `.way` files: RIFF-like tagged chunks (`tag[4] u32le size payload [pad to 4]`).
//! `WTWR`, `GDAT`, `GROM`, `RNOD` and `RSEG` contain further chunks.

use crate::{Error, Result};

const CONTAINERS: [&[u8; 4]; 5] = [b"WTWR", b"GDAT", b"GROM", b"RNOD", b"RSEG"];

#[derive(Debug)]
pub struct Chunk<'a> {
    pub tag: [u8; 4],
    pub data: &'a [u8],
    pub children: Vec<Chunk<'a>>,
}

impl Chunk<'_> {
    pub fn tag_str(&self) -> &str {
        std::str::from_utf8(&self.tag).unwrap_or("????")
    }
    /// Payload as a NUL-terminated string (for `MNAM`, `RNAM`, `NNAM`).
    pub fn as_cstr(&self) -> Option<&str> {
        let n = self.data.iter().position(|&b| b == 0).unwrap_or(self.data.len());
        std::str::from_utf8(&self.data[..n]).ok()
    }
}

pub fn parse(data: &[u8]) -> Result<Vec<Chunk<'_>>> {
    let mut d = data;
    let mut out = Vec::new();
    while !d.is_empty() {
        let at = data.len() - d.len();
        if d.len() < 8 {
            return Err(Error::Eof(at));
        }
        let tag: [u8; 4] = d[..4].try_into().unwrap();
        let size = u32::from_le_bytes(d[4..8].try_into().unwrap()) as usize;
        let body = d.get(8..8 + size).ok_or(Error::Eof(at))?;
        let children = if CONTAINERS.contains(&&tag) { parse(body)? } else { Vec::new() };
        out.push(Chunk { tag, data: body, children });
        // Chunk payloads are padded to a multiple of 4 (a 3-byte MNAM is followed by 1 pad byte).
        d = d.get(8 + ((size + 3) & !3)..).unwrap_or(&[]);
    }
    Ok(out)
}

/// A road graph node (`RNOD`).
#[derive(Debug, Clone)]
pub struct Node {
    pub name: String,
    pub pos: [f64; 3],
    pub flag: u32,
    /// `ORTN`: a placement matrix (three axes, then a position), for nodes where vehicles are
    /// put down facing a given way.
    pub orientation: Option<[f64; 12]>,
}

/// A road segment (`RSEG`): a polyline (`ATTR` bit 0 clear) or a cubic curve through four
/// control points (bit 0 set), with the road width at its start and end.
#[derive(Debug, Clone)]
pub struct Segment {
    /// First field of `ATTR`: flags; bit 0 marks a curve (observed values such as 1, 2, 22, 53, 98).
    pub kind: u32,
    /// Second field of `ATTR`, a real number.
    pub value: f64,
    /// Third field of `ATTR`.
    pub flag: u32,
    /// `WDTH`: the road width at the start and at the end (the original reads a list of doubles).
    pub width: [f64; 2],
    /// `VDAT`: the centre line.
    pub points: Vec<[f64; 3]>,
}

/// A `GROM`: one spatial cell of the world with its nodes and road segments.
#[derive(Debug, Clone, Default)]
pub struct Room {
    pub name: String,
    pub nodes: Vec<Node>,
    pub segments: Vec<Segment>,
}

fn f64_at(d: &[u8], o: usize) -> Option<f64> {
    Some(f64::from_le_bytes(d.get(o..o + 8)?.try_into().ok()?))
}

fn u32_at(d: &[u8], o: usize) -> Option<u32> {
    Some(u32::from_le_bytes(d.get(o..o + 4)?.try_into().ok()?))
}

fn segment(rseg: &Chunk) -> Option<Segment> {
    let mut s = Segment { kind: 0, value: 0.0, flag: 0, width: [0.0; 2], points: Vec::new() };
    for c in &rseg.children {
        match &c.tag {
            b"ATTR" => (s.kind, s.value, s.flag) = (u32_at(c.data, 0)?, f64_at(c.data, 4)?, u32_at(c.data, 12)?),
            b"WDTH" => s.width = [f64_at(c.data, 0)?, f64_at(c.data, 8)?],
            b"VDAT" => {
                let n = u32_at(c.data, 0)? as usize;
                s.points = (0..n)
                    .map(|i| Some([f64_at(c.data, 4 + i * 24)?, f64_at(c.data, 12 + i * 24)?, f64_at(c.data, 20 + i * 24)?]))
                    .collect::<Option<_>>()?;
            }
            _ => {}
        }
    }
    Some(s)
}

fn node(rnod: &Chunk) -> Option<Node> {
    let mut n = Node { name: String::new(), pos: [0.0; 3], flag: 0, orientation: None };
    for c in &rnod.children {
        match &c.tag {
            b"NNAM" => n.name = c.as_cstr()?.to_owned(),
            b"POSN" => n.pos = [f64_at(c.data, 0)?, f64_at(c.data, 8)?, f64_at(c.data, 16)?],
            b"FLAG" => n.flag = u32_at(c.data, 0)?,
            b"ORTN" => {
                let mut m = [0.0; 12];
                for (k, v) in m.iter_mut().enumerate() {
                    *v = f64_at(c.data, 8 * k)?;
                }
                n.orientation = Some(m);
            }
            _ => {}
        }
    }
    Some(n)
}

/// Collect the rooms of a parsed `.way` file.
pub fn rooms(chunks: &[Chunk]) -> Vec<Room> {
    fn walk(cs: &[Chunk], out: &mut Vec<Room>) {
        for c in cs {
            if &c.tag == b"GROM" {
                let mut room = Room::default();
                for k in &c.children {
                    match &k.tag {
                        b"RNAM" => room.name = k.as_cstr().unwrap_or("").to_owned(),
                        b"RNOD" => room.nodes.extend(node(k)),
                        b"RSEG" => room.segments.extend(segment(k)),
                        _ => {}
                    }
                }
                out.push(room);
            } else {
                walk(&c.children, out);
            }
        }
    }
    let mut out = Vec::new();
    walk(chunks, &mut out);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chunk(tag: &[u8; 4], body: &[u8]) -> Vec<u8> {
        let mut v = tag.to_vec();
        v.extend((body.len() as u32).to_le_bytes());
        v.extend(body);
        v.resize(v.len().next_multiple_of(4), 0);
        v
    }

    #[test]
    fn extracts_rooms() {
        let mut vdat = 2u32.to_le_bytes().to_vec();
        for p in [[1600.0f64, -3650.0, 405.0], [1590.0, -3640.0, 405.0]] {
            for x in p {
                vdat.extend(x.to_le_bytes());
            }
        }
        let mut attr = 98u32.to_le_bytes().to_vec();
        attr.extend(0.1f64.to_le_bytes());
        attr.extend(1u32.to_le_bytes());
        let mut wdth = Vec::new();
        for w in [3.0f64, 3.0] {
            wdth.extend(w.to_le_bytes());
        }
        let rseg = chunk(b"RSEG", &[chunk(b"ATTR", &attr), chunk(b"WDTH", &wdth), chunk(b"VDAT", &vdat)].concat());
        let grom = chunk(b"GROM", &[chunk(b"RNAM", b"room_da_010\0"), rseg].concat());
        let file = chunk(b"WTWR", &chunk(b"GDAT", &grom));
        let rs = rooms(&parse(&file).unwrap());
        assert_eq!(rs.len(), 1);
        assert_eq!(rs[0].name, "room_da_010");
        let s = &rs[0].segments[0];
        assert_eq!((s.kind, s.flag, s.width), (98, 1, [3.0, 3.0]));
        assert_eq!(s.points[1], [1590.0, -3640.0, 405.0]);
    }

    #[test]
    fn pads_to_four_and_nests() {
        // WTWR { MNAM("aa\0" + 1 pad), FLAG(4) }
        let mut inner = Vec::<u8>::new();
        inner.extend(b"MNAM\x03\0\0\0aa\0\0");
        inner.extend(b"FLAG\x04\0\0\0\0\0\0\0");
        let mut d = b"WTWR".to_vec();
        d.extend((inner.len() as u32).to_le_bytes());
        d.extend(inner);
        let cs = parse(&d).unwrap();
        assert_eq!(cs[0].children.len(), 2);
        assert_eq!(cs[0].children[0].as_cstr(), Some("aa"));
        assert_eq!(cs[0].children[1].tag_str(), "FLAG");
    }

    #[test]
    fn truncated_is_error() {
        assert!(parse(b"MNAM\x10\0\0\0ab").is_err());
    }
}

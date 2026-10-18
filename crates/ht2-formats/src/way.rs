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

#[cfg(test)]
mod tests {
    use super::*;

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

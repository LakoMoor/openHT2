//! `.RES` resource container: a sequence of `NAME COUNT\0` sections.
//!
//! `*FILES` sections hold `COUNT` entries of `name\0 u32le size, payload[size]`.
//! `COLORS`, `MATERIALS` and `SOUNDS` hold `COUNT` NUL-terminated text tokens.

use crate::{Error, Result};

#[derive(Debug)]
pub enum Section<'a> {
    Files { kind: &'a str, entries: Vec<FileEntry<'a>> },
    Tokens { kind: &'a str, tokens: Vec<&'a str> },
}

#[derive(Debug)]
pub struct FileEntry<'a> {
    /// Full name as stored, may carry flags after a space (e.g. `memfix`, `noload`).
    pub raw_name: &'a str,
    pub data: &'a [u8],
}

impl FileEntry<'_> {
    pub fn name(&self) -> &str {
        self.raw_name.split(' ').next().unwrap_or("")
    }
    pub fn flags(&self) -> impl Iterator<Item = &str> {
        self.raw_name.split(' ').skip(1)
    }
}

#[derive(Debug)]
pub struct Res<'a> {
    pub sections: Vec<Section<'a>>,
}

struct Cursor<'a> {
    d: &'a [u8],
    o: usize,
}

impl<'a> Cursor<'a> {
    fn cstr(&mut self) -> Result<&'a str> {
        let rest = self.d.get(self.o..).ok_or(Error::Eof(self.o))?;
        let n = rest.iter().position(|&b| b == 0).ok_or(Error::Eof(self.o))?;
        let s = std::str::from_utf8(&rest[..n])
            .map_err(|_| Error::Other(format!("non-ASCII string at {}", self.o)))?;
        self.o += n + 1;
        Ok(s)
    }
    fn u32(&mut self) -> Result<u32> {
        let b = self.d.get(self.o..self.o + 4).ok_or(Error::Eof(self.o))?;
        self.o += 4;
        Ok(u32::from_le_bytes(b.try_into().unwrap()))
    }
    fn bytes(&mut self, n: usize) -> Result<&'a [u8]> {
        let b = self.d.get(self.o..self.o + n).ok_or(Error::Eof(self.o))?;
        self.o += n;
        Ok(b)
    }
}

impl<'a> Res<'a> {
    pub fn parse(data: &'a [u8]) -> Result<Self> {
        let mut c = Cursor { d: data, o: 0 };
        let mut sections = Vec::new();
        while c.o < data.len() {
            let start = c.o;
            let h = c.cstr()?;
            let (kind, count) = h
                .split_once(' ')
                .and_then(|(k, n)| Some((k, n.parse::<usize>().ok()?)))
                .ok_or_else(|| Error::BadHeader(h.to_owned(), start))?;
            if kind.ends_with("FILES") {
                let mut entries = Vec::with_capacity(count);
                for _ in 0..count {
                    let raw_name = c.cstr()?;
                    let size = c.u32()? as usize;
                    entries.push(FileEntry { raw_name, data: c.bytes(size)? });
                }
                sections.push(Section::Files { kind, entries });
            } else {
                let tokens = (0..count).map(|_| c.cstr()).collect::<Result<_>>()?;
                sections.push(Section::Tokens { kind, tokens });
            }
        }
        Ok(Res { sections })
    }
}

/// Size guess for a `.txr` payload inside a `TEXTUREFILES` entry.
///
/// Observed: payload length = 74 + width*height*2 for 16-bit square textures
/// (8266 = 64x64, 32842 = 128x128, 131146 = 256x256). The 74-byte header is
/// not decoded yet, so non-square textures return `None`.
#[derive(Debug, Clone, Copy)]
pub struct TxrInfo {
    pub width: u32,
    pub height: u32,
}

pub fn txr_info(payload: &[u8]) -> Option<TxrInfo> {
    let body = payload.len().checked_sub(74)?;
    if body % 2 != 0 {
        return None;
    }
    let px = (body / 2) as u64;
    let side = (px as f64).sqrt().round() as u64;
    (side * side == px).then_some(TxrInfo { width: side as u32, height: side as u32 })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_files_and_tokens() {
        let mut d = b"TEXTUREFILES 1\0txr/a.txr memfix\0".to_vec();
        d.extend(3u32.to_le_bytes());
        d.extend([1, 2, 3]);
        d.extend(b"COLORS 2\x000.500\x001.000\0");
        let r = Res::parse(&d).unwrap();
        let Section::Files { entries, .. } = &r.sections[0] else { panic!() };
        assert_eq!(entries[0].name(), "txr/a.txr");
        assert_eq!(entries[0].flags().collect::<Vec<&str>>(), ["memfix"]);
        assert_eq!(entries[0].data, [1, 2, 3]);
        let Section::Tokens { tokens, .. } = &r.sections[1] else { panic!() };
        assert_eq!(tokens, &["0.500", "1.000"]);
    }

    #[test]
    fn txr_size_rule() {
        assert_eq!(txr_info(&[0; 8266]).unwrap().width, 64);
        assert_eq!(txr_info(&[0; 131146]).unwrap().width, 256);
        assert!(txr_info(&[0; 100]).is_none());
    }
}

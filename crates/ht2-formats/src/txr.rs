//! `.txr` textures stored in `TEXTUREFILES` entries of `.RES` files (3D-world variant).
//!
//! Layout (all little-endian):
//! ```text
//! 0   u16  0x000c
//! 12  u16  width
//! 14  u16  height
//! 16  u16  0x2010 (16 bits per pixel)
//! 18  "LOFF" u32 4, u32 end   -- `end` is the offset of the block after the pixels
//! 30  width*height u16 pixels
//! end "PFRM" u32 size, then R,G,B,A channel masks (u32 each; size is 16 or 28)
//!     "ENDR" u32 0
//! ```
//! The UI variant (header starts `00 01 01 00`, used by `LOAD.RES`) is not decoded yet.

use crate::{Error, Result};

#[derive(Debug)]
pub struct Texture {
    pub width: u32,
    pub height: u32,
    /// Tightly packed RGBA8, row-major, top row first.
    pub rgba: Vec<u8>,
}

fn u16_at(d: &[u8], o: usize) -> Result<u16> {
    d.get(o..o + 2).map(|b| u16::from_le_bytes(b.try_into().unwrap())).ok_or(Error::Eof(o))
}

fn u32_at(d: &[u8], o: usize) -> Result<u32> {
    d.get(o..o + 4).map(|b| u32::from_le_bytes(b.try_into().unwrap())).ok_or(Error::Eof(o))
}

/// Expand the bits selected by `mask` to 0..=255.
fn channel(px: u32, mask: u32) -> u8 {
    if mask == 0 {
        return 255;
    }
    let shift = mask.trailing_zeros();
    let max = mask >> shift;
    (((px & mask) >> shift) * 255 / max) as u8
}

pub fn decode(d: &[u8]) -> Result<Texture> {
    if u16_at(d, 0)? != 0x000c || d.get(18..22) != Some(b"LOFF") {
        return Err(Error::Other("not a 3D-world .txr (UI variant is unsupported)".into()));
    }
    let (w, h) = (u16_at(d, 12)? as usize, u16_at(d, 14)? as usize);
    let end = u32_at(d, 26)? as usize;
    if end != 30 + w * h * 2 {
        return Err(Error::Other(format!("pixel block ends at {end}, expected {}", 30 + w * h * 2)));
    }
    if d.get(end..end + 4) != Some(b"PFRM") {
        return Err(Error::Other("missing PFRM block".into()));
    }
    let [r, g, b, a] = [0, 4, 8, 12].map(|i| u32_at(d, end + 8 + i));
    let (r, g, b, a) = (r?, g?, b?, a?);

    let mut rgba = Vec::with_capacity(w * h * 4);
    for i in 0..w * h {
        let px = u16_at(d, 30 + i * 2)? as u32;
        rgba.extend([channel(px, r), channel(px, g), channel(px, b), channel(px, a)]);
    }
    Ok(Texture { width: w as u32, height: h as u32, rgba })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tex(w: u16, h: u16, px: &[u16], masks: [u32; 4]) -> Vec<u8> {
        let mut d = vec![0u8; 30];
        d[0] = 0x0c;
        d[12..14].copy_from_slice(&w.to_le_bytes());
        d[14..16].copy_from_slice(&h.to_le_bytes());
        d[16..18].copy_from_slice(&[0x10, 0x20]);
        d[18..22].copy_from_slice(b"LOFF");
        d[22] = 4;
        let end = (30 + px.len() * 2) as u32;
        d[26..30].copy_from_slice(&end.to_le_bytes());
        for p in px {
            d.extend(p.to_le_bytes());
        }
        d.extend(b"PFRM");
        d.extend(16u32.to_le_bytes());
        for m in masks {
            d.extend(m.to_le_bytes());
        }
        d.extend(b"ENDR\0\0\0\0");
        d
    }

    #[test]
    fn rgb565() {
        let d = tex(2, 1, &[0xf800, 0x07ff], [0xf800, 0x07e0, 0x001f, 0]);
        let t = decode(&d).unwrap();
        assert_eq!(t.rgba, [255, 0, 0, 255, 0, 255, 255, 255]);
    }

    #[test]
    fn argb4444() {
        let d = tex(1, 1, &[0x8f00], [0x0f00, 0x00f0, 0x000f, 0xf000]);
        let t = decode(&d).unwrap();
        assert_eq!(t.rgba, [255, 0, 0, 136]);
    }
}

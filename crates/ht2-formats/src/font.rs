//! Font-atlas extraction following king.exe `0x5e8e20`. Character codes are bytes;
//! conversion from Unicode is a separate layer. The top row of each cell is metadata.
use crate::{txr, Error, Result};
pub struct Glyph {
    pub left: u8,
    pub advance: u8,
    /// Row-major coverage, 0 or 255, excluding the cell's metadata row.
    pub coverage: Vec<u8>,
}
pub struct Font {
    pub cell_width: usize,
    pub height: usize,
    pub glyphs: Vec<Glyph>,
}
impl Font {
    pub fn parse(data: &[u8]) -> Result<Self> {
        let image = txr::decode(data)?;
        let (w, h) = (image.width as usize, image.height as usize);
        if w % 16 != 0 || h % 16 != 0 || w == 0 || h < 32 || w / 16 > 254 {
            return Err(Error::Other("invalid 16x16 font atlas".into()));
        }
        let cell_width = w / 16;
        let height = h / 16 - 1;
        let mut mask = vec![false; w * h];
        let ui = data.get(1..3) == Some(&[1, 1]);
        let pixel_start = if ui {
            18 + usize::from(data[0]) + usize::from(u16::from_le_bytes([data[5], data[6]])) * usize::from(data[7] / 8)
        } else { 30 };
        for i in 0..w * h {
            let value = if ui { data[pixel_start + i] != 0 } else {
                u16::from_le_bytes([data[pixel_start + i * 2], data[pixel_start + i * 2 + 1]]) != 0
            };
            let x = if data[17] & 0x10 != 0 { w - 1 - i % w } else { i % w };
            let y = if data[17] & 0x20 != 0 { i / w } else { h - 1 - i / w };
            mask[y * w + x] = value;
        }
        let mut glyphs = Vec::with_capacity(256);
        for code in 0..256 {
            let x = code % 16 * cell_width;
            let y = code / 16 * (height + 1);
            let row = &mask[y * w + x..y * w + x + cell_width];
            let left = row.iter().position(|&on| on).unwrap_or(cell_width);
            let mut end = left;
            while end < cell_width && row[end] { end += 1; }
            let advance = end + usize::from(end != 0);
            let coverage = (1..=height).flat_map(|dy| (0..cell_width).map(move |dx| (dy, dx)))
                .map(|(dy, dx)| if mask[(y + dy) * w + x + dx] { 255 } else { 0 }).collect();
            glyphs.push(Glyph { left: left as u8, advance: advance as u8, coverage });
        }
        Ok(Self { cell_width, height, glyphs })
    }
}

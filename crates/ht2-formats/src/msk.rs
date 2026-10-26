//! MS16 run-length masks (`0x5e94d0`, `0x5b8980`). Pixels retain their native
//! 16-bit colour; transparent runs contain no colour data.
use crate::{Error, Result};
pub struct Mask {
    pub width: u16,
    pub height: u16,
    pub pixels: Vec<Option<u16>>,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_invalid_runs_and_truncation() {
        let mut data = vec![0; 0x308];
        data[..8].copy_from_slice(b"MS16\x02\0\x01\0");
        for run in [0, 3, 128, 131, 1] {
            data.truncate(0x308);
            data.push(run);
            assert!(Mask::parse(&data).is_err());
        }
        data.truncate(0x308);
        data.extend_from_slice(&[129,1,0x34,0x12]);
        assert_eq!(Mask::parse(&data).unwrap().pixels, vec![None,Some(0x1234)]);
    }
}
impl Mask {
    pub fn parse(data: &[u8]) -> Result<Self> {
        if data.get(..4) != Some(b"MS16") { return Err(Error::Other("unsupported mask format".into())); }
        if data.len() < 0x308 { return Err(Error::Eof(data.len())); }
        let width = u16::from_le_bytes([data[4],data[5]]);
        let height = u16::from_le_bytes([data[6],data[7]]);
        let count = usize::from(width) * usize::from(height);
        if count == 0 || count > 16 * 1024 * 1024 { return Err(Error::Other("invalid mask dimensions".into())); }
        let mut pixels = Vec::with_capacity(count);
        let mut offset = 0x308;
        for _ in 0..height {
            let mut x = 0usize;
            while x < usize::from(width) {
                let control = *data.get(offset).ok_or(Error::Eof(offset))?;
                offset += 1;
                let n = usize::from(control & 127);
                if n == 0 || x + n > usize::from(width) { return Err(Error::Other("invalid mask run".into())); }
                if control & 128 != 0 { pixels.extend(std::iter::repeat_n(None,n)); }
                else {
                    let end = offset + n * 2;
                    for p in data.get(offset..end).ok_or(Error::Eof(offset))?.chunks_exact(2) {
                        pixels.push(Some(u16::from_le_bytes([p[0],p[1]])));
                    }
                    offset = end;
                }
                x += n;
            }
        }
        Ok(Self { width, height, pixels })
    }
}

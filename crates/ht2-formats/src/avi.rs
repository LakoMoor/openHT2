//! RIFF/AVI container used by the game's `.bne` movies. Compressed packets are borrowed;
//! decoding Indeo video is a separate layer. Never infer the codec from the extension.
use crate::{Error, Result};

#[derive(Debug)]
pub struct Movie<'a> {
    pub microseconds_per_frame: u32,
    pub frames: u32,
    pub width: u32,
    pub height: u32,
    pub streams: Vec<Stream<'a>>,
    pub packets: Vec<Packet<'a>>,
}
#[derive(Debug)]
pub struct Stream<'a> {
    pub kind: [u8; 4],
    pub handler: [u8; 4],
    pub scale: u32,
    pub rate: u32,
    pub start: u32,
    pub length: u32,
    pub format: &'a [u8],
}
#[derive(Debug)]
pub struct Packet<'a> {
    pub stream: usize,
    pub kind: [u8; 2],
    pub data: &'a [u8],
}

/// Interleaved signed PCM samples in their original channel order.
#[derive(Debug)]
pub struct PcmAudio {
    pub channels: u16,
    pub sample_rate: u32,
    pub samples: Vec<i16>,
}
impl Movie<'_> {
    /// Decode the uncompressed audio track; reject unsupported formats explicitly rather
    /// than playing compressed bytes as noise. AVI chunk padding is not audio data.
    pub fn pcm_audio(&self, stream: usize) -> Result<PcmAudio> {
        let s = self.streams.get(stream).ok_or_else(|| Error::Other("AVI audio stream absent".into()))?;
        let f = s.format;
        if s.kind != *b"auds" || f.len() < 16 {
            return Err(Error::Other("AVI stream has no WAVEFORMAT".into()));
        }
        let short = |off| u16::from_le_bytes([f[off], f[off + 1]]);
        let channels = short(2);
        let sample_rate = word(f, 4)?;
        if short(0) != 1 || short(14) != 16 {
            return Err(Error::Other("AVI audio requires signed 16-bit PCM".into()));
        }
        if channels == 0 || sample_rate == 0 || u32::from(short(12)) != u32::from(channels) * 2 {
            return Err(Error::Other("invalid PCM channel/rate/alignment".into()));
        }
        let mut samples = Vec::new();
        for p in self.packets.iter().filter(|p| p.stream == stream && p.kind == *b"wb") {
            if p.data.len() % usize::from(short(12)) != 0 {
                return Err(Error::Other("partial PCM sample frame".into()));
            }
            samples.extend(p.data.chunks_exact(2).map(|b| i16::from_le_bytes([b[0], b[1]])));
        }
        Ok(PcmAudio { channels, sample_rate, samples })
    }
}
fn word(d: &[u8], off: usize) -> Result<u32> {
    Ok(u32::from_le_bytes(d.get(off..off + 4).ok_or(Error::Eof(off))?.try_into().unwrap()))
}
fn chunks<'a>(mut d: &'a [u8], mut f: impl FnMut([u8; 4], &'a [u8]) -> Result<()>) -> Result<()> {
    while !d.is_empty() {
        let id = d.get(..4).ok_or(Error::Eof(0))?.try_into().unwrap();
        let n = word(d, 4)? as usize;
        let end = 8usize.checked_add(n).ok_or_else(|| Error::Other("AVI chunk overflow".into()))?;
        let data = d.get(8..end).ok_or(Error::Eof(d.len()))?;
        f(id, data)?;
        let padded = end.checked_add(n & 1).ok_or_else(|| Error::Other("AVI padding overflow".into()))?;
        d = d.get(padded..).ok_or(Error::Eof(end))?;
    }
    Ok(())
}
fn packets<'a>(d: &'a [u8], out: &mut Vec<Packet<'a>>, depth: usize) -> Result<()> {
    if depth > 16 { return Err(Error::Other("AVI record nesting too deep".into())); }
    chunks(d, |id, data| {
        if &id == b"LIST" {
            if data.get(..4) == Some(b"rec ") { packets(&data[4..], out, depth + 1)?; }
        } else if id[0].is_ascii_digit() && id[1].is_ascii_digit() {
            out.push(Packet { stream: ((id[0] - b'0') * 10 + id[1] - b'0') as usize, kind: [id[2], id[3]], data });
        }
        Ok(())
    })
}
pub fn parse(d: &[u8]) -> Result<Movie<'_>> {
    if d.get(..4) != Some(b"RIFF") || d.get(8..12) != Some(b"AVI ") {
        return Err(Error::BadHeader("RIFF/AVI".into(), 0));
    }
    let end = 8usize.checked_add(word(d, 4)? as usize).ok_or_else(|| Error::Other("AVI size overflow".into()))?;
    let body = d.get(12..end).ok_or(Error::Eof(d.len()))?;
    let mut movie = Movie { microseconds_per_frame: 0, frames: 0, width: 0, height: 0, streams: vec![], packets: vec![] };
    let mut header = false;
    chunks(body, |id, data| {
        if &id != b"LIST" { return Ok(()); }
        match data.get(..4) {
            Some(b"hdrl") => chunks(&data[4..], |id, data| {
                if &id == b"avih" {
                    movie.microseconds_per_frame = word(data, 0)?;
                    movie.frames = word(data, 16)?;
                    movie.width = word(data, 32)?;
                    movie.height = word(data, 36)?;
                    header = true;
                } else if &id == b"LIST" && data.get(..4) == Some(b"strl") {
                    let mut stream = None;
                    let mut format = &[][..];
                    chunks(&data[4..], |id, data| {
                        if &id == b"strh" {
                            stream = Some(Stream { kind: data.get(..4).ok_or(Error::Eof(0))?.try_into().unwrap(), handler: data.get(4..8).ok_or(Error::Eof(4))?.try_into().unwrap(), scale: word(data, 20)?, rate: word(data, 24)?, start: word(data, 28)?, length: word(data, 32)?, format: &[] });
                        } else if &id == b"strf" { format = data; }
                        Ok(())
                    })?;
                    let mut stream = stream.ok_or_else(|| Error::Other("AVI stream without strh".into()))?;
                    stream.format = format;
                    movie.streams.push(stream);
                }
                Ok(())
            })?,
            Some(b"movi") => packets(&data[4..], &mut movie.packets, 0)?,
            _ => {},
        }
        Ok(())
    })?;
    if !header { return Err(Error::Other("AVI main header missing".into())); }
    if movie.packets.iter().any(|p| p.stream >= movie.streams.len()) { return Err(Error::Other("AVI packet references absent stream".into())); }
    Ok(movie)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn chunk(id: &[u8; 4], data: &[u8]) -> Vec<u8> {
        let mut b = id.to_vec();
        b.extend((data.len() as u32).to_le_bytes());
        b.extend(data);
        if data.len() & 1 != 0 { b.push(0); }
        b
    }
    fn sample() -> Vec<u8> {
        let mut avih = vec![0u8; 56];
        for (off, v) in [(0, 83333u32), (16, 1), (32, 640), (36, 480)] { avih[off..off + 4].copy_from_slice(&v.to_le_bytes()); }
        let mut strh = vec![0u8; 56];
        strh[..8].copy_from_slice(b"vidsIV50");
        for (off, v) in [(20, 1u32), (24, 12), (32, 1)] { strh[off..off + 4].copy_from_slice(&v.to_le_bytes()); }
        let mut strl = b"strl".to_vec();
        strl.extend(chunk(b"strh", &strh));
        strl.extend(chunk(b"strf", &[1, 2, 3]));
        let mut hdrl = b"hdrl".to_vec();
        hdrl.extend(chunk(b"avih", &avih));
        hdrl.extend(chunk(b"LIST", &strl));
        let mut record = b"rec ".to_vec();
        record.extend(chunk(b"00dc", &[7, 8, 9]));
        let mut movi = b"movi".to_vec();
        movi.extend(chunk(b"LIST", &record));
        let mut avi = b"AVI ".to_vec();
        avi.extend(chunk(b"JUNK", &[1]));
        avi.extend(chunk(b"LIST", &hdrl));
        avi.extend(chunk(b"LIST", &movi));
        chunk(b"RIFF", &avi)
    }
    #[test]
    fn streams_nested_records_and_padding() {
        let data = sample();
        let m = parse(&data).unwrap();
        assert_eq!((m.width, m.height, m.frames), (640, 480, 1));
        assert_eq!((m.streams[0].scale, m.streams[0].rate), (1, 12));
        assert_eq!(m.streams[0].format, &[1, 2, 3]);
        assert_eq!(m.packets[0].data, &[7, 8, 9]);
        assert_eq!(m.packets[0].kind, *b"dc");
    }
    #[test]
    fn rejects_truncation_and_unknown_stream() {
        let data = sample();
        for n in 0..data.len() { assert!(parse(&data[..n]).is_err(), "prefix {n}"); }
        let mut bad = data.clone();
        let at = bad.windows(4).position(|w| w == b"00dc").unwrap();
        bad[at] = b'9';
        assert!(parse(&bad).is_err());
    }
    #[test]
    fn pcm_preserves_signed_stereo_samples_and_skips_other_packets() {
        let mut format = [0u8; 16];
        format[0..2].copy_from_slice(&1u16.to_le_bytes());
        format[2..4].copy_from_slice(&2u16.to_le_bytes());
        format[4..8].copy_from_slice(&22050u32.to_le_bytes());
        format[12..14].copy_from_slice(&4u16.to_le_bytes());
        format[14..16].copy_from_slice(&16u16.to_le_bytes());
        let a = [0, 128, 255, 127];
        let b = [255, 255, 0, 0];
        let mut m = Movie { microseconds_per_frame: 1, frames: 0, width: 0, height: 0,
            streams: vec![Stream { kind: *b"auds", handler: [0; 4], scale: 4, rate: 88200, start: 0, length: 2, format: &format }],
            packets: vec![Packet { stream: 0, kind: *b"wb", data: &a }, Packet { stream: 0, kind: *b"pc", data: &[1] }, Packet { stream: 0, kind: *b"wb", data: &b }] };
        let pcm = m.pcm_audio(0).unwrap();
        assert_eq!((pcm.channels, pcm.sample_rate), (2, 22050));
        assert_eq!(pcm.samples, [i16::MIN, i16::MAX, -1, 0]);
        m.packets.push(Packet { stream: 0, kind: *b"wb", data: &[0, 0] });
        assert!(m.pcm_audio(0).is_err());
        assert!(m.pcm_audio(1).is_err());
    }
}

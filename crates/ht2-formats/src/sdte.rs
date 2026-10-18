//! `SDTE` files (`*.sch`, `GAME.CNF`): text scripts XOR-ed with a keystream.
//!
//! The file starts with the ASCII tag `SDTE`; everything after it is XOR-ed with the low bytes
//! of a subtractive lagged-Fibonacci generator (55 words, lag 31, the `ran3` construction from
//! Knuth with `mj = seed`). The game uses the fixed seed 23728. The generator is restarted for
//! every file.

use crate::{Error, Result};

/// Seed the game passes to the generator for every `SDTE` file.
pub const SEED: u32 = 23728;

const LEN: usize = 55;
const LAG: usize = 31;

pub struct KeyStream {
    state: [u32; LEN],
    next: usize,
    next_lag: usize,
}

impl KeyStream {
    pub fn new(seed: u32) -> Self {
        let mut s = [0u32; LEN];
        s[LEN - 1] = seed;
        let (mut mj, mut mk) = (seed, 1u32);
        for i in (0..54).map(|i| i * 21) {
            let slot = i % LEN;
            let next = mj.wrapping_sub(mk);
            s[slot] = mk;
            mk = next;
            mj = s[slot];
        }
        for _ in 0..4 {
            for i in 0..LEN {
                s[i] = s[i].wrapping_sub(s[(i + LAG) % LEN]);
            }
        }
        KeyStream { state: s, next: 0, next_lag: LAG }
    }
}

impl Iterator for KeyStream {
    type Item = u8;

    fn next(&mut self) -> Option<u8> {
        self.next = (self.next + 1) % LEN;
        self.next_lag = (self.next_lag + 1) % LEN;
        let v = self.state[self.next].wrapping_sub(self.state[self.next_lag]);
        self.state[self.next] = v;
        Some(v as u8)
    }
}

/// True if `data` carries the `SDTE` tag.
pub fn is_sdte(data: &[u8]) -> bool {
    data.starts_with(b"SDTE")
}

/// Decrypt a whole file. The result is Windows-1251 text.
pub fn decrypt(data: &[u8]) -> Result<Vec<u8>> {
    if !is_sdte(data) {
        return Err(Error::Other("missing SDTE tag".into()));
    }
    Ok(data[4..].iter().zip(KeyStream::new(SEED)).map(|(c, k)| c ^ k).collect())
}

/// Decode Windows-1251 bytes to a `String`.
pub fn cp1251_to_string(b: &[u8]) -> String {
    b.iter()
        .map(|&c| match c {
            0..=0x7f => c as char,
            0xc0..=0xff => char::from_u32(0x0410 + (c as u32 - 0xc0)).unwrap(),
            0xa8 => '\u{0401}',
            0xb8 => '\u{0451}',
            _ => '\u{fffd}',
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keystream_known_answer() {
        let ks: Vec<u8> = KeyStream::new(SEED).take(12).collect();
        assert_eq!(ks, [0x50, 0x42, 0x80, 0xe2, 0xcc, 0xe4, 0x3b, 0x3d, 0xb4, 0x8b, 0x83, 0xe9]);
    }

    #[test]
    fn round_trip() {
        let plain = b"\n(\n  ( \"COP1\"  male  police  1  0 )\n)";
        let mut file = b"SDTE".to_vec();
        file.extend(plain.iter().zip(KeyStream::new(SEED)).map(|(p, k)| p ^ k));
        assert_eq!(decrypt(&file).unwrap(), plain);
        assert!(decrypt(b"nope").is_err());
    }

    #[test]
    fn cyrillic() {
        assert_eq!(cp1251_to_string(&[0xd5, 0xeb, 0xe5, 0xe1]), "Хлеб");
    }
}

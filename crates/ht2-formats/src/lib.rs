//! Clean-room readers for the data formats of Hard Truck 2 / King of the Road.
//!
//! Formats are documented in `docs/FORMATS.md`. Nothing here contains game data.

pub mod b3d;
pub mod avi;
pub mod font;
pub mod res;
pub mod sdte;
pub mod sexpr;
pub mod tech;
pub mod txr;
pub mod way;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("unexpected end of data at offset {0}")]
    Eof(usize),
    #[error("bad section header {0:?} at offset {1}")]
    BadHeader(String, usize),
    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, Error>;

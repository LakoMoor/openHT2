//! Vehicle simulation, ported function by function from the original game.
//!
//! Each ported function names its original address (`fn_00507330` = `0x507330` in `king.exe`),
//! and is checked against the original code running in an emulator (see `tools/emu`).
//! Where the meaning of a function is known it is documented; the names stay address-based
//! until then so they can always be traced back.

pub mod carv;
pub mod curves;
pub mod filter;
pub mod gearbox;
pub mod gearbox_update;
pub mod globals;
pub mod init;
pub mod input;
pub mod math;
pub mod matrix;
pub mod mem;
pub mod motion;
pub mod tech_system;
pub mod tech_rec;

pub use tech_rec::TechRec;

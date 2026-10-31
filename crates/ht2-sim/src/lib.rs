//! Vehicle simulation, ported function by function from the original game.
//!
//! Each ported function names its original address (`fn_00507330` = `0x507330` in `king.exe`),
//! and is checked against the original code running in an emulator (see `tools/emu`).
//! Where the meaning of a function is known it is documented; the names stay address-based
//! until then so they can always be traced back.

pub mod ai;
pub mod carv;
pub mod chassis;
pub mod companies;
pub mod competition;
pub mod contact;
pub mod ctor;
pub mod curves;
pub mod filter;
pub mod gearbox;
pub mod gearbox_update;
pub mod globals;
pub mod init;
pub mod loader;
pub mod input;
pub mod math;
pub mod matrix;
pub mod mem;
pub mod rigid;
pub mod road;
pub mod shape;
pub mod step;
pub mod objects;
pub mod orders;
pub mod police;
pub mod stl;
pub mod motion;
pub mod tech_system;
pub mod tech_rec;
pub mod world;
pub mod game_message;
pub mod controller;
pub mod scene_state;
pub mod instruments;
pub mod gauges;
pub mod maneuver;
pub mod scene_probe;
pub mod scene_requests;
pub mod scene_return;
pub mod vehicle_reset;
pub mod condition;
pub mod ai_scene;

pub use tech_rec::TechRec;

pub mod scene_events;

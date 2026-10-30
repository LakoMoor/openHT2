//! Engine, clutch, automatic gearbox and fuel of the player's vehicle, run by the code ported
//! from the original (`ht2-sim`). Everything else about the vehicle is still approximate.

use ht2_sim::gearbox::TECH_TABLE;
use ht2_sim::mem::Mem;
use ht2_sim::{globals, init, input, tech_rec::RECORD_LEN, tech_system as ts};

const CAR: u32 = 0x1000_0000;
const CLOCK: u32 = 0x1800_0000;

/// What the powertrain wants from the chassis this step.
#[derive(Clone, Copy, Debug)]
pub struct Output {
    /// Force at the driven wheels, N (negative in reverse).
    pub drive_force: f32,
    /// Engine speed in revolutions per second.
    pub engine_rev: f32,
    /// 0 reverse, 1 neutral, 2.. forward gears.
    pub gear: i32,
    /// Remaining fuel, litres.
    pub fuel: f32,
    /// Engine state: 0 off, 1 starting, 2 running, 3 stalled / out of fuel.
    pub engine_state: i32,
    /// For diagnostics: gear being shifted to, shift code, and the throttle after processing.
    pub target_gear: i32,
    pub shift_code: i32,
    pub throttle_eff: f32,
    /// The effect event `+0x3c18` written by `0x4ff800` (1 or 2, 0 none); consumed (zeroed) here the way
    /// `0x548250` zeroes it after reading, so each event is seen once.
    pub effect_event: u32,
}

pub struct Powertrain {
    m: Mem,
    time: f64,
}

/// Gear request codes the original's gearbox understands (`+0x2746`).
const REQUEST_REVERSE: i16 = 0x14;
const REQUEST_NEUTRAL: i16 = 0x15;
const REQUEST_DRIVE: i16 = 0x16;

impl Powertrain {
    /// `tech_file`: the contents of `vehicle.tech`; `record`: index of the vehicle in it.
    pub fn new(tech_file: &[u8], record: usize, wheels: usize, driven: &[bool], mass: f32) -> Result<Self, String> {
        let name_len = u32::from_le_bytes(tech_file.get(0..4).ok_or("short vehicle.tech")?.try_into().unwrap()) as usize;
        let hdr = 4 + name_len;
        let rec_len = u32::from_le_bytes(tech_file[hdr + 4..hdr + 8].try_into().unwrap()) as usize;
        let count = u32::from_le_bytes(tech_file[hdr + 8..hdr + 12].try_into().unwrap()) as usize;
        if rec_len != RECORD_LEN || record >= count {
            return Err("unexpected vehicle.tech layout".into());
        }
        let mut m = Mem::new();
        for i in 0..count {
            m.write(TECH_TABLE + (i * RECORD_LEN) as u32, &tech_file[57 + i * rec_len..57 + (i + 1) * rec_len]);
        }
        globals::install_clock(&mut m, CLOCK, 0.0, 0.0);
        globals::install_constants(&mut m);
        init::init_engine(&mut m, CAR, record as i32);

        m.set_i32(CAR + 0xdd4, wheels as i32);
        let load = mass * 9.81 / wheels.max(1) as f32;
        for i in 0..wheels as u32 {
            m.set_i32(CAR + 0x1bb4 + 4 * i, driven.get(i as usize).copied().unwrap_or(false) as i32);
            m.set_i16(CAR + 0x19a4 + 2 * i, 0); // wheel not spinning
            m.set_i32(CAR + 0x19d4 + 4 * i, 0); // wheel is not slipping
            m.set_f32(CAR + 0xd20 + 4 * i, load);
            m.set_f32(CAR + 0x2670 + 4 * i, 0.9);
        }
        // Engine running, in drive-by-pedal mode, automatic gearbox.
        m.set_i32(CAR + 0x2774, 1);
        m.set_i32(CAR + 0x2760, 1); // fuel is used
        m.set_i32(CAR + 0x1b24, 2);
        m.set_i16(CAR + 0x2704, 1);
        m.set_i16(CAR + 0x2706, 1);
        ht2_sim::gearbox::fn_004ff770(&mut m, CAR, 1); // automatic
        Ok(Powertrain { m, time: 0.0 })
    }

    /// Advance by `dt` seconds. `speed` is the forward speed in m/s.
    pub fn step(&mut self, dt: f32, throttle: f32, brake: f32, speed: f32) -> Output {
        let m = &mut self.m;
        self.time += dt as f64;
        globals::install_clock(m, CLOCK, self.time, dt as f64);

        m.set_f32(CAR + 0xdc8, speed);
        m.set_f32(CAR + 0xdc0, speed.abs());
        m.set_f32(CAR + 0x270c, throttle.clamp(0.0, 1.0));
        // 0x53b4c0 (analog pedals): the axis f in -1..1 gives +0x270c = f above the threshold and
        // +0x2710 = -f^3 below it, so a brake of strength b is f = -b. STAND-IN: the axis itself
        // (device filters `injoyfilterA`) is not ported, the brake value is used unfiltered.
        let pedal = if throttle > 0.0 { 0.0 } else { brake.clamp(0.0, 1.0).powi(3) };
        m.set_f32(CAR + 0x2710, pedal);
        m.set_f32(CAR + 0x3bd4, 1.0); // seconds since the last reset of the vehicle

        // Pick drive or reverse like a driver would: throttle from standstill goes forward,
        // brake at standstill goes backward.
        let gear = m.i32(CAR + 0x1c1c);
        let shifting = m.i32(CAR + 0x1c24) != 0; // neutral between gears is not a request to start
        let request = if shifting {
            0
        } else if throttle > 0.0 && gear < 2 && speed > -0.5 {
            REQUEST_DRIVE
        } else if brake > 0.0 && throttle == 0.0 && speed < 0.5 && gear != 0 {
            REQUEST_REVERSE
        } else if throttle == 0.0 && brake == 0.0 && speed.abs() < 0.2 && gear != 1 {
            REQUEST_NEUTRAL
        } else {
            0
        };
        m.set_i16(CAR + 0x2746, request);

        input::fn_00500850(m, CAR);

        let effect_event = m.u32(CAR + 0x3c18);
        m.set_u32(CAR + 0x3c18, 0);
        Output {
            drive_force: m.f32(CAR + 0x1c94),
            engine_rev: m.f32(CAR + 0x1c84),
            gear: m.i32(CAR + 0x1c1c),
            fuel: ts::value(m, CAR + 0x3ef0),
            engine_state: m.i32(CAR + 0x1b24),
            target_gear: m.i32(CAR + 0x1c20),
            shift_code: m.i32(CAR + 0x1c24),
            throttle_eff: m.f32(CAR + 0x29d8),
            effect_event,
        }
    }

    /// Steering position in `-1..=1` after the original's smoothing (positive is left).
    pub fn steering(&self) -> f32 {
        self.m.f32(CAR + 0x29e0)
    }

    /// Set the steering keys: -1 right, 0 none, 1 left.
    pub fn set_steer_keys(&mut self, dir: i32) {
        self.m.wr8(CAR + 0x2702, (dir > 0) as u8);
        self.m.wr8(CAR + 0x2703, (dir < 0) as u8);
        self.m.set_i16(CAR + 0x2706, 0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    /// Braking while rolling raises the demand `+0x29dc` above 0.5 and the gearbox raises the
    /// effect event (original `0x4ff800`); coasting without the brake never does.
    #[test]
    #[ignore = "requires HT2_GAME installed game data"]
    fn braking_at_speed_raises_the_effect_event() {
        let game = std::path::PathBuf::from(std::env::var("HT2_GAME").unwrap());
        let tech = std::fs::read(game.join("vehicle.tech")).unwrap();
        let events = |brake: f32| {
            let mut p = Powertrain::new(&tech, 0, 4, &[false, false, true, true], 8000.0).unwrap();
            (0..200).map(|_| p.step(0.02, 0.0, brake, 15.0).effect_event).filter(|&e| e != 0).count()
        };
        assert!(events(1.0) > 0, "braking produced no effect event");
        assert_eq!(events(0.0), 0);
    }
}

//! A raw `vehicle.tech` record (2416 bytes), read by byte offset like the original does.

pub const RECORD_LEN: usize = 0x970;

#[derive(Clone, Copy)]
pub struct TechRec<'a>(pub &'a [u8]);

impl<'a> TechRec<'a> {
    pub fn new(bytes: &'a [u8]) -> Self {
        assert!(bytes.len() >= RECORD_LEN, "a tech record is {RECORD_LEN} bytes");
        TechRec(bytes)
    }

    fn f(&self, off: usize) -> f32 {
        f32::from_le_bytes(self.0[off..off + 4].try_into().unwrap())
    }

    /// Wheel radius, m (`+100`).
    pub fn wheel_radius(&self) -> f32 {
        self.f(100)
    }

    /// Number of gears (`+144`, an integer).
    pub fn num_gears(&self) -> i32 {
        i32::from_le_bytes(self.0[144..148].try_into().unwrap())
    }

    /// `air_press_coeff` (`+172`).
    pub fn air_press(&self) -> f32 {
        self.f(172)
    }

    /// `air_resist_coeff` (`+176`).
    pub fn air_resist(&self) -> f32 {
        self.f(176)
    }

    /// Peak engine torque, N*m (`+192`).
    pub fn max_torque(&self) -> f32 {
        self.f(192)
    }

    /// Final drive ratio (`+204`).
    pub fn rear_axle(&self) -> f32 {
        self.f(204)
    }

    /// Transmission ratio of gear `g`: 0 reverse (`+208`), 1 neutral, 2.. forward gears (`+212 + 4*(g-1)`).
    pub fn gear_ratio(&self, g: i32) -> f32 {
        self.f(208 + 4 * g as usize)
    }

    /// Engine speed at peak power, rad/s (`+268`).
    pub fn tach_peak(&self) -> f32 {
        self.f(268)
    }
}

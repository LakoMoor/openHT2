//! `vehicle.tech`: physics and shop parameters of every drivable vehicle.
//!
//! Layout: a header (`u32 len`, file name, `u32 timestamp`, `u32 record_len`, `u32 count`, ...,
//! 57 bytes for the shipped file) followed by fixed-size records of `record_len` bytes.
//! Only the fields that were verified against the data are exposed; the rest is skipped.
//! Field offsets were cross-checked against the MIT-licensed reRNR/RNRFileFormats description.

use crate::{Error, Result};

const HEADER_LEN: usize = 57;

#[derive(Debug, Clone)]
pub struct Tech {
    /// Position of the record in the file (the game addresses vehicles by this index).
    pub index: usize,
    /// Upper-case catalogue name, e.g. `KAMAZ`.
    pub name: String,
    /// Prefix of the model blocks in `.b3d` files, e.g. `Kamaz`.
    pub prefix: String,
    /// Cabin node prefix at record offset 2322, used by `0x4dde70`.
    pub cabin: String,
    /// Initial camera position and X tilt, copied by `0x531cc0`.
    pub driver_seat:[f32;3],
    pub driver_tilt:f32,
    pub mass: f32,
    pub wheel_radius: f32,
    /// Fuel tank, litres.
    pub tank: f32,
    pub horse_power: u32,
    pub price: u32,
    /// Number of forward gears actually used.
    pub num_gears: u32,
    /// Ratios for gears 1.. (index 0 is neutral).
    pub gear_ratios: [f32; 14],
    pub rear_axle_ratio: f32,
    pub reverse_ratio: f32,
    /// Peak engine torque, N*m.
    pub max_torque: f32,
    pub brake_power: f32,
    pub air_resistance: f32,
    pub center_of_mass: [f32; 3],
    pub suspension_travel: f32,
    pub spring: f32,
    pub driving_wheels: [bool; 8],
    pub steering_wheels: [bool; 8],
    /// The model in the vehicle catalogue (`evehicles.sch`) the record is for, e.g. `KamazR`
    /// (offset 2342; the game finds a vehicle's record by it, `0x42e9f0`).
    pub key: String,
    /// What it can load, kg (offset 272; `0x41b830`).
    pub capacity: f32,
    /// The width and length of its load space, m (offsets 280 and 284; their product is the
    /// load area, `0x41b830`).
    pub load_space: [f32; 2],
    /// Fuel used per metre (offset 156; `calculatePlayerSpeedsAndFares`, `0x423f40`).
    pub fuel_use: f32,
}

fn f32_at(d: &[u8], o: usize) -> f32 {
    f32::from_le_bytes(d[o..o + 4].try_into().unwrap())
}

fn u32_at(d: &[u8], o: usize) -> u32 {
    u32::from_le_bytes(d[o..o + 4].try_into().unwrap())
}

fn text(d: &[u8], o: usize, len: usize) -> String {
    let b = &d[o..o + len];
    let n = b.iter().position(|&c| c == 0).unwrap_or(len);
    b[..n].iter().map(|&c| c as char).collect()
}

fn flags(d: &[u8], o: usize) -> [bool; 8] {
    std::array::from_fn(|i| u32_at(d, o + i * 4) != 0)
}

pub fn parse(d: &[u8]) -> Result<Vec<Tech>> {
    if d.len() < HEADER_LEN {
        return Err(Error::Eof(d.len()));
    }
    let name_len = u32_at(d, 0) as usize;
    let rec_len = u32_at(d, 4 + name_len + 4) as usize;
    let count = u32_at(d, 4 + name_len + 8) as usize;
    // The last field read below ends at offset 2372.
    if rec_len < 2372 || HEADER_LEN + count * rec_len > d.len() {
        return Err(Error::Other(format!("implausible vehicle.tech header: record {rec_len} bytes x {count}")));
    }
    let mut out = Vec::new();
    for i in 0..count {
        let r = &d[HEADER_LEN + i * rec_len..HEADER_LEN + (i + 1) * rec_len];
        let name = text(r, 2272, 30);
        if name.is_empty() {
            continue;
        }
        out.push(Tech {
            index: i,
            name,
            prefix: text(r, 2302, 20),
            cabin: text(r, 2322, 20),
            driver_seat:[f32_at(r,396),f32_at(r,400),f32_at(r,404)],
            driver_tilt:f32_at(r,408),
            mass: f32_at(r, 96),
            wheel_radius: f32_at(r, 100),
            tank: f32_at(r, 108),
            horse_power: u32_at(r, 1988),
            price: u32_at(r, 1992),
            num_gears: u32_at(r, 144),
            gear_ratios: std::array::from_fn(|g| f32_at(r, 212 + g * 4)),
            rear_axle_ratio: f32_at(r, 204),
            reverse_ratio: f32_at(r, 208),
            max_torque: f32_at(r, 192),
            brake_power: f32_at(r, 200),
            air_resistance: f32_at(r, 176),
            center_of_mass: [f32_at(r, 36), f32_at(r, 40), f32_at(r, 44)],
            suspension_travel: f32_at(r, 748),
            spring: f32_at(r, 780),
            driving_wheels: flags(r, 300),
            steering_wheels: flags(r, 332),
            key: text(r, 2342, 30),
            capacity: f32_at(r, 272),
            load_space: [f32_at(r, 280), f32_at(r, 284)],
            fuel_use: f32_at(r, 156),
        });
    }
    Ok(out)
}

/// `0x4dde70`: the second cabin lookup uses the record following the
/// tractor boundary loaded into 0x6ce338 from the vehicle.tech header.
pub fn cabin_fallback_index(d:&[u8])->Result<usize> {
    if d.len()<HEADER_LEN {return Err(Error::Eof(d.len()));}
    let name_len=u32_at(d,0) as usize;
    let start=name_len.checked_add(16).ok_or_else(||Error::Other("invalid tech header length".into()))?;
    if start.checked_add(12).is_none_or(|end|end>HEADER_LEN){return Err(Error::Other("invalid tech category header".into()));}
    let count=u32_at(d,start-4) as usize;
    let index=(u32_at(d,start+8) as usize).checked_add(1).ok_or_else(||Error::Other("invalid fallback index".into()))?;
    if index>=count {return Err(Error::Other("fallback cabin record outside vehicle table".into()));}
    Ok(index)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One record named "TEST" with a mass and a wheel radius.
    fn sample() -> Vec<u8> {
        let rec_len = 2416;
        let mut d = Vec::new();
        d.extend(4u32.to_le_bytes());
        d.extend(b"a.tc");
        d.extend(0u32.to_le_bytes()); // timestamp
        d.extend((rec_len as u32).to_le_bytes());
        d.extend(1u32.to_le_bytes()); // count
        d.resize(HEADER_LEN, 0);
        let mut r = vec![0u8; rec_len];
        r[96..100].copy_from_slice(&5100f32.to_le_bytes());
        r[100..104].copy_from_slice(&0.53f32.to_le_bytes());
        r[2272..2276].copy_from_slice(b"TEST");
        r[300 + 8..300 + 12].copy_from_slice(&1u32.to_le_bytes());
        d.extend(r);
        d
    }

    #[test]
    fn reads_fields() {
        let v = parse(&sample()).unwrap();
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].name, "TEST");
        assert_eq!(v[0].mass, 5100.0);
        assert!((v[0].wheel_radius - 0.53).abs() < 1e-6);
        assert_eq!(v[0].driving_wheels, [false, false, true, false, false, false, false, false]);
    }
    #[test]
    fn cabin_selection_matches_original_lookup() {
        for line in include_str!("../../../tools/emu/golden_cabin_selection.tsv").lines(){
            let p:Vec<_>=line.split('\t').collect();let mut data=sample();
            for (offset,value) in [(2302,p[0]),(2322,p[1])]{let bytes=value.as_bytes();data[HEADER_LEN+offset..HEADER_LEN+offset+bytes.len()].copy_from_slice(bytes);}
            let record=parse(&data).unwrap().remove(0);
            assert_eq!(record.prefix,p[0]);assert_eq!(format!("{}Cab",record.cabin),p[2]);
        }
    }
    #[test]
    fn fallback_header_matches_original_record_address(){
        for line in include_str!("../../../tools/emu/golden_cabin_fallback.tsv").lines(){
            let p:Vec<_>=line.split('\t').collect();let boundary=p[6].parse::<u32>().unwrap();
            let mut d=sample();
            // Header of the original shipped table has a 13-byte filename,
            // 43 records and seven category boundaries following the count.
            d[..4].copy_from_slice(&13u32.to_le_bytes());d[4..17].copy_from_slice(b".\\truck2.tch\0");
            d[21..25].copy_from_slice(&2416u32.to_le_bytes());d[25..29].copy_from_slice(&43u32.to_le_bytes());
            d[37..41].copy_from_slice(&boundary.to_le_bytes());
            assert_eq!(cabin_fallback_index(&d).unwrap(),boundary as usize+1);
        }
        let mut d=sample();d[..4].copy_from_slice(&u32::MAX.to_le_bytes());assert!(cabin_fallback_index(&d).is_err());
        assert!(cabin_fallback_index(&[]).is_err());
    }

    #[test]
    fn rejects_truncated() {
        let d = sample();
        assert!(parse(&d[..d.len() - 100]).is_err());
    }
}

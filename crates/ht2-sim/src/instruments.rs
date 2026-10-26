//! Cabin needle transforms from the cabin branch of `0x53d1f0`.
use crate::{mem::Mem,matrix};
pub fn cabin_needles(m:&mut Mem,bases:[u32;3],destinations:[u32;3],speed:f32,rev:f32,fuel_fraction:f32) {
    let angles=[rev*f32::from_bits(0x3dc104fb),speed.abs()*f32::from_bits(0x3d20d97c),fuel_fraction*f32::from_bits(0x4016cbe4)];
    for i in 0..3 {
        matrix::mat_rot_y(m,0x2ffd_4800,angles[i]);
        matrix::mat_mul(m,destinations[i],0x2ffd_4800,bases[i]);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn original_cabin_needle_transforms(){
        for (case,line) in include_str!("../../../tools/emu/golden_cabin_needles.tsv").lines().enumerate(){
            let p:Vec<_>=line.split('\t').collect();let mut m=Mem::new();let bases=[0x2a000000,0x2a000040,0x2a000080];let out=[0x2a000100,0x2a000140,0x2a000180];
            for i in 0..3 {for (j,v) in p[i+1].split(',').enumerate(){m.set_u32(bases[i]+j as u32*4,v.parse().unwrap());}}
            let v:Vec<f32>=p[0].split(',').map(|v|f32::from_bits(v.parse().unwrap())).collect();
            cabin_needles(&mut m,bases,out,v[0],v[1],v[2]);
            for i in 0..3 {assert_eq!((0..12).map(|j|m.u32(out[i]+j*4)).collect::<Vec<_>>(),p[i+4].split(',').map(|v|v.parse::<u32>().unwrap()).collect::<Vec<_>>(),"case {case} needle {i}");}
        }
    }
}

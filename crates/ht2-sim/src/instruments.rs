//! Cabin needle transforms from the cabin branch of `0x53d1f0`.
use crate::{mem::Mem,matrix};
#[derive(Debug,PartialEq,Eq)]
pub struct KeyCommand {pub node:u32,pub value:i32,pub digit:bool}
/// Key commands emitted by the cabin branch of `0x53d1f0`. Missing nodes
/// are skipped; odometer nodes are visited from +0x1338 down to +0x1328.
pub fn cabin_keys(m:&Mem,panel:u32,gear:i32,automatic:i32,park:i32,low_fuel:i32,digits:[i32;5])->Vec<KeyCommand> {
    let mut commands=Vec::new();
    for (offset,value) in [(0x1318,gear),(0x131c,i32::from(automatic==0)),(0x1320,park),(0x1324,low_fuel)] {
        let node=m.u32(panel+offset);if node!=0 {commands.push(KeyCommand{node,value,digit:false});}
    }
    for (i,value) in digits.into_iter().enumerate(){
        let node=m.u32(panel+0x1338-i as u32*4);if node!=0 {commands.push(KeyCommand{node,value,digit:true});}
    }
    commands
}
/// Steering-wheel matrix portion of `0x4decc0`, before cabin suspension motion.
pub fn cabin_steering(m:&mut Mem,base:u32,dst:u32,steering:f32) {
    let angle=(steering as f64*core::f64::consts::FRAC_PI_2) as f32;
    matrix::mat_rot_z(m,0x2ffd_4840,angle);
    matrix::mat_mul(m,dst,0x2ffd_4840,base);
}
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
    fn original_cabin_key_commands(){
        for line in include_str!("../../../tools/emu/golden_cabin_keys.tsv").lines(){
            let p:Vec<_>=line.split('\t').collect();let mut m=Mem::new();let panel=0x2a010000;
            for (i,v) in p[0].split(',').enumerate(){m.set_u32(panel+0x1318+i as u32*4,v.parse().unwrap());}
            let v:Vec<i32>=p[1].split(',').map(|v|v.parse().unwrap()).collect();
            let actual=cabin_keys(&m,panel,v[0],v[1],v[2],v[3],v[4..].try_into().unwrap());
            let expected=p[2].split(';').filter(|s|!s.is_empty()).map(|s|{let v:Vec<_>=s.split(',').collect();KeyCommand{node:v[0].parse().unwrap(),value:v[1].parse().unwrap(),digit:v[2]=="1"}}).collect::<Vec<_>>();
            assert_eq!(actual,expected);
        }
    }
    #[test]
    fn original_cabin_steering(){
        for line in include_str!("../../../tools/emu/golden_cabin_steering.tsv").lines(){
            let p:Vec<_>=line.split('\t').collect();let mut m=Mem::new();let base=0x2a000000;let out=base+0x100;
            for (i,v) in p[1].split(',').enumerate(){m.set_u32(base+i as u32*4,v.parse().unwrap());}
            cabin_steering(&mut m,base,out,f32::from_bits(p[0].parse().unwrap()));
            assert_eq!((0..12).map(|i|m.u32(out+i*4)).collect::<Vec<_>>(),p[2].split(',').map(|v|v.parse::<u32>().unwrap()).collect::<Vec<_>>());
        }
    }
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

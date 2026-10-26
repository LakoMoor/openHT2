//! Cabin needle transforms from the cabin branch of `0x53d1f0`.
use crate::{mem::Mem,matrix};
/// Rectangle passed to the scene by `0x532330`. Original table indices are
/// signed view modes; arithmetic preserves the original 32-bit wrapping.
pub fn view_rectangle(m:&Mem,scenario:u32)->[i32;4] {
    let surface=m.u32(scenario+0x24);let width=m.i32(surface+0x100);let height=m.i32(surface+0x104);
    let mode=m.i32(scenario+0x574);let shift=(mode as u32).wrapping_mul(4);
    let (x_offset,y_offset)=if mode==0 {(0x560,0x564)}else{(0x558,0x55c)};
    let mut x=m.i32(0x696bf8u32.wrapping_add(shift)).wrapping_add(m.i32(scenario+x_offset));
    let mut top=((height.wrapping_sub(480) as f64*0.2061855670103093
        +m.i32(0x696c0cu32.wrapping_add(shift)) as f64)+m.i32(scenario+y_offset) as f64).trunc() as i64 as i32;
    let mut bottom=m.i32(0x696c20u32.wrapping_add(shift)).wrapping_add(top);
    if width>=780 && height>=580 {top=top.wrapping_add(50);bottom=bottom.wrapping_add(50);}
    if x<0 {x=0;}
    if top<m.i32(0x672d7c){top=m.i32(0x672d7c);}
    if bottom<=top {bottom=top.wrapping_add(1);}
    if bottom>height {bottom=height;}
    if x<0 {x=0;}
    let right=if x<width || x.wrapping_add(1)<width {width}else{x.wrapping_add(1)};
    [x,top,right,bottom]
}
/// `0x4de990`: view transform before cabin suspension. Scenario matrices
/// +0x1d8 and +0x5f0 are supplied by their original owners.
pub fn cabin_view(m:&mut Mem,dst:u32,ignore_tilt:bool) {
    const S:u32=0x2ffd_4900;
    let (result,translation,rotation,tilt,temporary)=(S,S+48,S+96,S+144,S+192);
    matrix::put12(m,result,&matrix::IDENTITY);
    let scenario=m.u32(0x6d2078);
    if scenario==0 || m.u32(scenario+0x268)==0 {
        let bytes=m.read(result,48);m.write(dst,&bytes);return;
    }
    let vehicle=m.u32(scenario+0x268);
    let height=if m.u32(vehicle+0x51a8)!=0 {m.f32(vehicle+0x4f48)}else{0.};
    let up:[f32;3]=core::array::from_fn(|i|m.f32(0x702630+i as u32*4));
    matrix::mat_translation(m,translation,height*up[0],height*up[1],height*up[2]);
    if m.u32(0x6cec68)!=0 {
        let angle=-m.f32(0x696d10);
        matrix::put12(m,tilt,&matrix::IDENTITY);
        if !ignore_tilt {
            let factor=if (angle as f64).abs()<=core::f64::consts::FRAC_PI_2 {(angle as f64).cos()}else{0.};
            matrix::mat_rot_x(m,tilt,(m.f32(scenario+200) as f64*factor) as f32);
        }
        matrix::mat_rot_z(m,rotation,angle);
        matrix::mat_mul(m,temporary,tilt,rotation);
        matrix::mat_mul(m,result,temporary,translation);
    }else{
        let angle=match m.i32(scenario+0x574){1=>Some(0x3f060a92),2=>Some(0x3fc90fdb),-2=>Some(0xbfc90fdb),-1=>Some(0xbf3ba866),_=>None};
        if let Some(bits)=angle {
            matrix::mat_rot_z(m,rotation,f32::from_bits(bits));
            matrix::mat_mul(m,result,rotation,translation);
        }else {let bytes=m.read(translation,48);m.write(result,&bytes);}
        matrix::mat_rot_x(m,tilt,(height as f64 * -0.25) as f32);
        matrix::mat_mul(m,temporary,scenario+0x1d8,tilt);
        matrix::mat_mul(m,result,temporary,result);
    }
    matrix::mat_mul(m,dst,scenario+0x5f0,result);
}
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
    fn original_view_rectangles(){
        for line in include_str!("../../../tools/emu/golden_view_rectangle.tsv").lines(){
            let p:Vec<_>=line.split('\t').collect();let v:Vec<i32>=p[0].split(',').map(|s|s.parse().unwrap()).collect();let mut m=Mem::new();let s=0x2a000000;let surface=s+0x10000;
            m.set_u32(s+0x24,surface);m.set_i32(surface+0x100,v[0]);m.set_i32(surface+0x104,v[1]);m.set_i32(s+0x574,v[2]);
            for (o,value) in [0x558,0x55c,0x560,0x564].into_iter().zip(&v[3..7]){m.set_i32(s+o,*value);}
            for (address,value) in [0x696bf8u32,0x696c0c,0x696c20].into_iter().zip(&v[7..10]){m.set_i32(address.wrapping_add((v[2] as u32).wrapping_mul(4)),*value);}
            m.set_i32(0x672d7c,v[10]);
            assert_eq!(view_rectangle(&m,s).to_vec(),p[1].split(',').map(|s|s.parse::<i32>().unwrap()).collect::<Vec<_>>());
        }
    }
    #[test]
    fn original_cabin_view_transforms(){
        for (case,line) in include_str!("../../../tools/emu/golden_cabin_view.tsv").lines().enumerate(){
            let p:Vec<_>=line.split('\t').collect();let f:Vec<i64>=p[0].split(',').map(|s|s.parse().unwrap()).collect();let mut m=Mem::new();crate::globals::install_constants(&mut m);
            let s=0x2a000000;let v=s+0x10000;let out=v+0x10000;
            m.set_u32(0x6d2078,if f[0]!=0{s}else{0});m.set_u32(s+0x268,if f[1]!=0{v}else{0});m.set_u32(0x6cec68,f[2] as u32);
            m.set_i32(s+0x574,f[4] as i32);m.set_u32(v+0x51a8,f[5] as u32);m.set_u32(0x696d10,f[6] as u32);m.set_u32(v+0x4f48,f[7] as u32);m.set_u32(s+200,f[8] as u32);
            for (o,data) in [(0x1d8,p[1]),(0x5f0,p[2])]{for (i,value) in data.split(',').enumerate(){m.set_u32(s+o+i as u32*4,value.parse().unwrap());}}
            cabin_view(&mut m,out,f[3]!=0);
            assert_eq!((0..12).map(|i|m.u32(out+i*4)).collect::<Vec<_>>(),p[3].split(',').map(|v|v.parse::<u32>().unwrap()).collect::<Vec<_>>(),"case {case}");
        }
    }
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

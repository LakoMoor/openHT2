//! Matrix-stack and clipping state of `0x5ae0e0`. Device viewport/transform
//! uploads at the end of that function are handled by the rendering backend.
use crate::{mem::Mem,matrix,math};
pub fn setup(m:&mut Mem,scene:u32,model:u32,view:u32,software:bool) {
    const S:u32=0x2ffd_4000;
    let (scaled,scale,combined)=(S,S+48,S+96);
    m.set_u32(0x702e08,m.u32(0x702e08)|3);
    let bytes=m.read(model,48);m.write(scene+0x1070,&bytes);
    math::mat_inverse_scene(m,scene+0x10a0,scene+0x1070);
    for o in [0x2020,0x2024,0x2028,0x202c]{m.set_u32(scene+o,0);}
    let view=if software {
        let v=(1.0/m.f64(scene+0x2110)) as f32;
        matrix::put12(m,scale,&[v,0.,0.,0.,v,0.,0.,0.,v,0.,0.,0.]);
        matrix::mat_mul(m,scaled,view,scale);scaled
    }else{view};
    let bytes=m.read(view,48);m.write(0x702998,&bytes);
    matrix::mat_mul(m,combined,model,0x702998);
    let bytes=m.read(combined,48);m.write(scene+0x8f0,&bytes);
    // The original computes this product a second time before inversion.
    matrix::mat_mul(m,combined,model,0x702998);
    math::mat_inverse_scene(m,0x7029c8,combined);
    for (i,j) in [0,3,6,1,4,7,2,5,8,9,10,11].into_iter().enumerate(){m.set_u32(scene+0xcb0+i as u32*4,m.u32(0x7029c8+j*4));}
    m.set_u32(scene+0x2020,1);
    let bytes=m.read(0x702998,48);m.write(scene+0x10d0,&bytes);m.set_u32(scene+0x2024,1);
    matrix::put12(m,scene+0x1490,&matrix::IDENTITY);m.set_u32(scene+0x202c,1);
    for (src,dst) in [(0x7a0,0x7b0),(0x7a4,0x800),(0x7a8,0x850),(0x7ac,0x8a0)]{m.set_u32(scene+dst,m.u32(scene+src));}
    let distance=m.f64(scene+0x2148);
    let factor=((distance+distance)/((m.i32(scene+0x7a8)>>19) as f64*m.f32(0x702998) as f64)) as f32;
    m.set_f32(scene+0x6d8,factor);
    let mut blend=m.i32(scene+0x2108) as f64*0.01;
    if blend>1.0 {blend=1.0;}if blend<0.0 {blend=0.0;}
    m.set_f64(scene+0x2150,blend);m.set_u32(scene+0x2028,1);
    let serial=m.u32(0x67ef88).wrapping_add(1);m.set_u32(0x67ef88,serial);m.set_u32(scene+0x20fc,serial);
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn original_scene_setup_bits() {
        for line in include_str!("../../../tools/emu/golden_scene_setup.tsv").lines(){
            let p:Vec<_>=line.split('\t').collect();let c=0x2a000000;let a=c+0x10000;let b=a+0x100;let mut m=Mem::new();
            for (base,s) in [(a,p[1]),(b,p[2])]{for(i,v)in s.split(',').enumerate(){m.set_u32(base+i as u32*4,v.parse().unwrap());}}
            m.set_f64(c+0x2110,2.0);m.set_f64(c+0x2148,250.0);m.set_i32(c+0x2108,p[3].parse().unwrap());
            for(i,v)in [1,2,100<<19,4].iter().enumerate(){m.set_i32(c+0x7a0+i as u32*4,*v);}
            m.set_u32(0x67ef88,9);setup(&mut m,c,a,b,p[0]=="1");
            let offsets=[0x1070,0x10a0,0x8f0,0xcb0,0x10d0,0x1490];
            let mut bits=Vec::new();for o in offsets{bits.extend((0..12).map(|i|m.u32(c+o+i*4)));}
            bits.extend([0x2020,0x2024,0x2028,0x202c,0x7b0,0x800,0x850,0x8a0,0x6d8,0x2150,0x2154,0x20fc].map(|o|m.u32(c+o)));
            assert_eq!(bits,p[4].split(',').map(|s|s.parse().unwrap()).collect::<Vec<u32>>());
        }
    }
}

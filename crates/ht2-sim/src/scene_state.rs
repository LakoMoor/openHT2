//! Matrix-stack and clipping state of `0x5ae0e0`. Device viewport/transform
//! uploads at the end of that function are handled by the rendering backend.
use crate::{mem::Mem,matrix,math};
/// Complete transform lookup and matrix preparation of `0x5eb4b0`.
/// The surface owner supplies the original clip rectangle (`0x5b6710`).
pub fn prepare_camera_node(m:&mut Mem,camera:u32,scene:u32,clip:[i32;4]) {
    const WORLD:u32=0x2ffd_4500;
    matrix::put12(m,WORLD,&matrix::IDENTITY);
    matrix::node_world_transform(m,m.u32(camera+0x50),WORLD);
    prepare_camera(m,camera,WORLD,scene,clip);
}
/// Matrix/clip portion of `0x5eb4b0`. `world_transform` is the result of
/// the camera node's hierarchy lookup (`0x5f25c0`), supplied by its owner.
pub fn prepare_camera(m:&mut Mem,camera:u32,world_transform:u32,scene:u32,clip:[i32;4]) {
    const S:u32=0x2ffd_4300;
    let (model,rotation,translation,stereo,scaled)=(S,S+48,S+96,S+144,S+192);
    math::mat_inverse(m,model,world_transform);
    if m.u32(0x702e8c)&0x100!=0 {
        let mut shift=m.f64(scene+0x2130)*0.5;
        let mut angle=m.f64(scene+0x2138) as f32;
        if m.u32(scene+0x210c)==0 {shift=-shift;angle=-angle;}
        matrix::mat_rot_z(m,rotation,angle);
        matrix::put12(m,translation,&[1.,0.,0.,0.,1.,0.,0.,0.,1.,shift as f32,0.,0.]);
        matrix::mat_mul(m,stereo,translation,rotation);
        matrix::mat_apply(m,model,stereo);
    }
    let x=if clip[0]<0 {clip[2] as f64/clip[2].wrapping_sub(clip[0]) as f64}else{1.};
    let y=if clip[1]<0 {clip[3] as f64/clip[3].wrapping_sub(clip[1]) as f64}else{1.};
    let sign=if m.u32(camera+0x64)!=0 {-2.}else{2.};
    let sx=(sign/(m.f32(camera+0x58) as f64*x)) as f32;
    let sz=(2./(y*m.f32(camera+0x54) as f64)) as f32;
    matrix::put12(m,scaled,&[sx,0.,0.,0.,1.,0.,0.,0.,sz,0.,0.,0.]);
    setup(m,scene,model,scaled,m.u32(0x720908)==0);
}
/// Hardware view upload at the end of `0x5ae0e0`, in Direct3D row-vector layout.
/// The original rounds each 3x4 product before packing it into a 4x4 matrix.
pub fn device_view(m:&mut Mem,model:u32)->[f32;16] {
    const S:u32=0x2ffd_4100;
    let (rotation,scale,combined,result)=(S,S+48,S+96,S+144);
    matrix::mat_rot_x(m,rotation,f32::from_bits(0xbfc90fda));
    matrix::put12(m,scale,&[1.,0.,0.,0.,1.,0.,0.,0.,-1.,0.,0.,0.]);
    matrix::mat_mul(m,combined,model,rotation);
    matrix::mat_mul(m,result,combined,scale);
    let mut out=[0.;16];
    for row in 0..4 {for col in 0..3 {out[row*4+col]=m.f32(result+(row*3+col) as u32*4);}}
    out[15]=1.;out
}
/// Numeric commands emitted by `0x5adea0`. Matrices retain Direct3D's original
/// row-vector layout; a backend must explicitly convert its coordinate system.
pub struct ProjectionState {
    pub viewport:[u32;4],
    pub matrix:[f32;16],
}
pub fn projection(m:&mut Mem,scene:u32,clip:[i32;4])->ProjectionState {
    let x=(m.i32(scene+0x7a0)>>20).wrapping_add(clip[0]);
    let y=(m.i32(scene+0x7a4)>>20).wrapping_add(clip[1]);
    let half_w=(m.i32(scene+0x7a8)>>20).wrapping_add(1);
    let half_h=(m.i32(scene+0x7ac)>>20).wrapping_add(1);
    for (o,v) in [(0x2164,x),(0x2168,y),(0x216c,half_w),(0x2170,half_h)]{m.set_i32(scene+o,v);}
    let width=(half_w.wrapping_mul(2) as u32).min(clip[2].wrapping_sub(clip[0]) as u32);
    let height=(half_h.wrapping_mul(2) as u32).min(clip[3].wrapping_sub(clip[1]) as u32);
    let mut out=[0.0;16];
    let near=m.f64(scene+0x2110);
    if near<=0.0001 {out[0]=1.;out[5]=1.;out[10]=1.;out[15]=1.;}
    else {
        let p=scene+0x10a0+m.u32(scene+0x2024)*48;
        out[0]=m.f32(p);out[5]=m.f32(p+32);out[8]=m.f32(p+12);out[9]=m.f32(p+20);out[11]=1.;
        let far=m.f64(scene+0x2118);let range=far-near;
        out[10]=(far/range) as f32;out[14]=(-(near*far/range)) as f32;
    }
    ProjectionState{viewport:[x as u32,y as u32,width,height],matrix:out}
}
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
    fn original_camera_scene_preparation() {
        for (case,line) in include_str!("../../../tools/emu/golden_camera_scene.tsv").lines().enumerate(){
            let p:Vec<_>=line.split('\t').collect();let mut m=Mem::new();
            let (camera,scene,transform)=(0x2a000000,0x2a010000,0x2a020000);
            for (base,values) in [(transform,p[0]),(camera+0x54,p[1])]{
                for (i,v) in values.split(',').enumerate(){m.set_u32(base+i as u32*4,v.parse().unwrap());}
            }
            m.set_u32(camera+0x64,p[2].parse().unwrap());m.set_u32(0x702e8c,p[3].parse::<u32>().unwrap()*0x100);
            m.set_u32(scene+0x210c,p[4].parse().unwrap());m.set_f64(scene+0x2130,p[5].parse().unwrap());m.set_f64(scene+0x2138,p[6].parse().unwrap());
            m.set_u32(0x720908,1);m.set_f64(scene+0x2110,1.);m.set_f64(scene+0x2148,250.);m.set_i32(scene+0x7a8,100<<19);
            let clip=p[7].split(',').map(|v|v.parse::<i32>().unwrap()).collect::<Vec<_>>().try_into().unwrap();
            let bytes=m.read(transform,48);m.write(transform+0x40,&bytes);m.set_u32(transform+0x2c,0);m.set_u32(transform+0x38,1);m.set_u32(camera+0x50,transform);
            prepare_camera_node(&mut m,camera,scene,clip);
            for (base,expected) in [(scene+0x1070,p[8]),(scene+0x10d0,p[9])]{
                assert_eq!((0..12).map(|i|m.u32(base+i*4)).collect::<Vec<_>>(),expected.split(',').map(|v|v.parse::<u32>().unwrap()).collect::<Vec<_>>(),"case {case}");
            }
        }
    }
    #[test]
    fn original_hardware_view_uploads() {
        for line in include_str!("../../../tools/emu/golden_scene_device_view.tsv").lines(){
            let (input,expected)=line.split_once('\t').unwrap();
            let mut m=Mem::new();let model=0x2a000000;
            for (i,v) in input.split(',').enumerate(){m.set_u32(model+i as u32*4,v.parse().unwrap());}
            assert_eq!(device_view(&mut m,model).map(f32::to_bits).to_vec(),expected.split(',').map(|v|v.parse::<u32>().unwrap()).collect::<Vec<_>>());
        }
    }
    #[test]
    fn original_projection_commands() {
        for line in include_str!("../../../tools/emu/golden_scene_projection.tsv").lines(){
            let p:Vec<_>=line.split('\t').collect();let c=0x2a000000;let mut m=Mem::new();
            let nums=|s:&str|s.split(',').map(|s|s.parse::<u32>().unwrap()).collect::<Vec<_>>();
            for (i,v)in nums(p[0]).iter().enumerate(){m.set_u32(c+0x7a0+i as u32*4,*v);}
            m.set_f64(c+0x2110,p[1].parse().unwrap());m.set_f64(c+0x2118,p[2].parse().unwrap());m.set_u32(c+0x2024,1);
            for (i,v)in nums(p[3]).iter().enumerate(){m.set_u32(c+0x10d0+i as u32*4,*v);}
            let clip:Vec<i32>=p[4].split(',').map(|s|s.parse().unwrap()).collect();
            let result=projection(&mut m,c,clip.try_into().unwrap());
            assert_eq!(result.viewport.to_vec(),nums(p[5]));
            assert_eq!(result.matrix.map(|v|v.to_bits()).to_vec(),nums(p[6]));
            assert_eq!([0x2164,0x2168,0x216c,0x2170].map(|o|m.u32(c+o)).to_vec(),nums(p[7]));
        }
    }
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

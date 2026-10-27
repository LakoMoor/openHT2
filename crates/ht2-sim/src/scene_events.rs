//! Point-event traversal (`0x5dbec0` / `0x5dbf00`). Runtime scene owners
//! provide virtual child lookup, volume tests and the event callback.
use crate::{mem::Mem,math,matrix};
const POINT:u32=0x721298;
pub trait EventHooks {
    fn child(&mut self,m:&mut Mem,node:u32,index:i32)->u32;
    /// Native volume receiver is node+0x38 for branches, node+0x40 for events.
    fn contains(&mut self,m:&mut Mem,volume:u32,point:u32)->bool {contains(m,volume,point)}
    fn event(&mut self,m:&mut Mem,callback:u32,command:u32,user:u32);
}
/// `0x5f88d0`: point against an unconditional volume, sphere, cylinder or
/// plane. Outside-sphere/cylinder forms use strict comparisons. The cylinder
/// variants have different intermediate f32 stores in the original.
pub fn contains(m:&Mem,volume:u32,point:u32)->bool {
    contains_point(m,volume,std::array::from_fn(|i|m.f32(point+i as u32*4)))
}
/// Same predicate for a caller-owned point, without scratch-memory writes.
pub fn contains_point(m:&Mem,volume:u32,point:[f32;3])->bool {
    let kind=m.u32(volume);
    if kind==0 {return true;}
    if kind>5 {return false;}
    let data=m.u32(volume+8);
    let p=point.map(|x|x as f64);
    let a=std::array::from_fn::<_,8,_>(|i|m.f32(data+i as u32*4) as f64);
    if kind==5 {return ((p[2]*a[2]+p[1]*a[1])+p[0]*a[0])+a[3]>0.;}
    let d=std::array::from_fn::<_,3,_>(|i|(p[i]-a[i]) as f32 as f64);
    let length=|v:[f64;3]|((v[2]*v[2]+v[1]*v[1])+v[0]*v[0]).sqrt();
    if kind<=2 {let len=length(d);return if kind==1 {!(len>a[3])}else{len>a[3]};}
    let along=(a[5]*d[2]+a[4]*d[1])+a[3]*d[0];
    // Original tests C0 alone here, so unordered takes the cap-rejection path.
    if !(a[7]>=along.abs()) {return kind==4;}
    let radial=std::array::from_fn(|i|{
        let projection=if kind==3 && i==0 {along*a[3+i]}else{(along*a[3+i]) as f32 as f64};
        (d[i]-projection) as f32 as f64
    });
    let len=length(radial);
    if kind==3 {!(len>a[6])}else{len>a[6]}
}
pub fn query(m:&mut Mem,root:u32,point:[f32;3],callback:u32,user:u32,h:&mut impl EventHooks) {
    for (i,v) in point.into_iter().enumerate(){m.set_f32(POINT+i as u32*4,v);}
    m.set_u32(0x7216bc,callback);m.set_u32(0x7212b0,user);
    visit(m,root,h);
}
fn children(m:&mut Mem,node:u32,index:i32,h:&mut impl EventHooks) {
    let mut child=h.child(m,node,index);
    while child!=0 {visit(m,child,h);child=m.u32(child+8);}
}
fn local_point(m:&mut Mem,transform:u32) {
    if transform==0 {return;}
    const MATRIX:u32=0x2ffd_4d00;
    matrix::put12(m,MATRIX,&matrix::IDENTITY);
    matrix::node_world_transform(m,transform,MATRIX);
    math::mat_inverse_scene(m,MATRIX+48,MATRIX);
    matrix::transform_in_place(m,POINT,MATRIX+48);
}
pub fn visit(m:&mut Mem,node:u32,h:&mut impl EventHooks) {
    if node==0 {return;}
    match m.u32(node+0x10) {
        4=>{let saved=m.read(POINT,12);local_point(m,m.u32(node+0x3c));children(m,node,0,h);m.write(POINT,&saved);}
        5|15|21=>children(m,node,0,h),
        7=>{children(m,node,0,h);children(m,node,1,h);}
        8=>{let inside=h.contains(m,node+0x38,POINT);children(m,node,(!inside) as i32,h);}
        9=>{if h.contains(m,node+0x40,POINT){h.event(m,m.u32(0x7216bc),node+0x30,m.u32(0x7212b0));}}
        17=>{if m.rd8(node+0x28)&8==0 {let saved=m.read(POINT,12);let target=m.u32(node+0x40);local_point(m,m.u32(node+0x44));visit(m,target,h);m.write(POINT,&saved);}}
        18=>{let refer=m.u32(node+0x3c);let key=m.i32(if refer==0 {node+0x38}else{refer+0x38});children(m,node,key,h);}
        _=>{},
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_point_volumes() {
        for (case,line) in include_str!("../../../tools/emu/golden_event_volumes.tsv").lines().enumerate(){
            let p:Vec<_>=line.split('\t').collect();let mut m=Mem::new();
            let bytes=(0..p[1].len()).step_by(2).map(|i|u8::from_str_radix(&p[1][i..i+2],16).unwrap()).collect::<Vec<_>>();
            m.set_u32(0x1000,p[0].parse().unwrap());m.set_u32(0x1008,0x2000);m.write(0x2000,&bytes);
            assert_eq!(contains(&m,0x1000,0x2020),p[2]=="1","case {case}, kind {}",p[0]);
        }
    }
    struct Hooks(Vec<u32>,bool);
    impl EventHooks for Hooks {
        fn child(&mut self,m:&mut Mem,n:u32,i:i32)->u32 {if (0..2).contains(&i){m.u32(n+0x50+i as u32*4)}else{0}}
        fn contains(&mut self,m:&mut Mem,v:u32,p:u32)->bool {if self.1 {contains(m,v,p)}else{m.u32(v)!=0}}
        fn event(&mut self,m:&mut Mem,c:u32,n:u32,u:u32){self.0.extend([c,n,u,m.u32(POINT),m.u32(POINT+4),m.u32(POINT+8)]);m.set_f32(POINT,m.f32(POINT)+1.);}
    }
    #[test]
    fn native_event_traversal() {
        for (data,native) in [(include_str!("../../../tools/emu/golden_scene_events.tsv"),false),(include_str!("../../../tools/emu/golden_scene_events_native_volumes.tsv"),true)] {
        for (case,line) in data.lines().enumerate(){
            let p:Vec<_>=line.split('\t').collect();let mut m=Mem::new();
            let bytes=(0..p[0].len()).step_by(2).map(|i|u8::from_str_radix(&p[0][i..i+2],16).unwrap()).collect::<Vec<_>>();m.write(0x2a000000,&bytes);
            let mut h=Hooks(Vec::new(),native);query(&mut m,0x2a000000,[3.,4.,5.],123,456,&mut h);
            let expected=p[1].split(',').filter(|x|!x.is_empty()).map(|x|x.parse::<u32>().unwrap()).collect::<Vec<_>>();
            assert_eq!(h.0,expected,"trace {case}");
            let point=(0..3).map(|i|m.u32(POINT+i*4).to_string()).collect::<Vec<_>>().join(",");assert_eq!(point,p[2],"point {case}");
        }
        }
    }
}

//! Operations bridging scene vehicles and the trajectory AI.
use crate::mem::Mem;

/// `0x58f230`: disable one named movement bit; other values do nothing.
pub fn disable_movement(m:&mut Mem,body:u32,value:i32) {
    if matches!(value,1|2|4|8) {m.set_u32(body+0x4ac,m.u32(body+0x4ac)&!(value as u32));}
}
/// `0x58f290`: returns whether the original emits its invalid-detail diagnostic.
/// The write still occurs after that diagnostic returns.
pub fn collision_detail(m:&mut Mem,body:u32,value:i32)->bool {
    if matches!(value,0|1) {m.set_i32(body+0xf88,value);false}
    else {m.set_i32(body+0xf8c,value);!(0..=3).contains(&value)}
}
/// `0x58f2d0`: value 3 preserves +0xf88; all calls clear +0xf8c.
pub fn motion(m:&mut Mem,body:u32,value:i32) {
    if matches!(value,0|1|2) {m.set_i32(body+0xf88,value);}
    m.set_u32(body+0xf8c,0);
}

pub trait TrajectoryHooks {
    /// `0x54ca60`, on the complete vehicle object.
    fn mode(&mut self,m:&mut Mem,vehicle:u32,value:i32);
    /// `0x58f230`, on vehicle+0x10.
    fn movement(&mut self,m:&mut Mem,body:u32,value:i32) {disable_movement(m,body,value);}
    /// `0x58f290`, on vehicle+0x10.
    fn collision_detail(&mut self,m:&mut Mem,body:u32,value:i32) {
        assert!(!collision_detail(m,body,value),"invalid collision detail");
    }
    /// `0x57de90`; deliberately read twice, after the mode changes.
    fn world_transform(&mut self,m:&mut Mem,body:u32)->[f32;12];
    /// `0x43f690`, after copying the player's identity with 0x41d300.
    fn publish_flags(&mut self,m:&mut Mem,player:u32,flags:u32);
}

pub trait PhysicsHooks:TrajectoryHooks {
    /// Speed field of the snapshot queried by 0x43fc40.
    fn player_speed(&mut self,m:&mut Mem,player:u32)->f64;
    /// 0x437520 position followed by 0x439320 direction lookup.
    fn player_direction(&mut self,m:&mut Mem,player:u32)->[f32;3];
    /// Body virtual +0x90/+0x94, both called with the trailing argument zero.
    fn linear_velocity(&mut self,m:&mut Mem,body:u32,value:[f32;3]);
    fn angular_velocity(&mut self,m:&mut Mem,body:u32,value:[f32;3]);
    /// 0x58f2d0.
    fn motion(&mut self,m:&mut Mem,body:u32,value:i32) {motion(m,body,value);}
}

/// `0x4cf080`: restore physical velocity and activate the body. Snapshot and
/// route queries remain explicit boundaries, retaining the original f64 speed.
pub fn enter_physics(m:&mut Mem,vehicle:u32,player:u32,flags:u32,publish:bool,h:&mut impl PhysicsHooks) {
    let _world=h.world_transform(m,vehicle+0x10);
    // 0x57def0 is a side-effect-free body-state pointer query; its return is unused.
    let mut direction=std::array::from_fn(|i|m.f32(0x702620+i as u32*4));
    let mut speed=m.f32(vehicle+0x51e4) as f64;
    if player!=m.u32(0x68b990) {
        speed=h.player_speed(m,player);direction=h.player_direction(m,player);
        if publish {let value=m.u32(flags)|1;m.set_u32(flags,value);h.publish_flags(m,player,value);}
    }
    if speed>18. {speed=18.;}
    const INPUT:u32=0x2ffd_4b00;
    for (i,value) in direction.into_iter().enumerate(){m.set_f32(INPUT+i as u32*4,value);}
    crate::matrix::vec_unit(m,INPUT+16,INPUT);
    let velocity=std::array::from_fn(|i|(speed*m.f32(INPUT+16+i as u32*4) as f64) as f32);
    h.linear_velocity(m,vehicle+0x10,velocity);
    let zero=std::array::from_fn(|i|m.f32(0x702650+i as u32*4));
    h.angular_velocity(m,vehicle+0x10,zero);
    h.mode(m,vehicle,1);h.motion(m,vehicle+0x10,3);
}

/// Complete `0x4cef60` orchestration. Scene/AI owners implement the explicit
/// boundary operations; no synthetic physics-mode flags are substituted here.
pub fn leave_physics(m:&mut Mem,vehicle:u32,player:u32,flags:u32,h:&mut impl TrajectoryHooks) {
    h.mode(m,vehicle,2);
    h.movement(m,vehicle+0x10,1);
    h.collision_detail(m,vehicle+0x10,0);
    let saved=h.world_transform(m,vehicle+0x10);
    crate::matrix::put12(m,vehicle+0x50a0,&saved);
    let current=h.world_transform(m,vehicle+0x10);
    let local:[f64;3]=std::array::from_fn(|i|m.f32(vehicle+0x5230+i as u32*4) as f64);
    // x uses Y+Z+X; y/z use Y+X+Z. Keep the original accumulation order.
    for axis in 0..3 {
        let first=current[3+axis] as f64*local[1];
        let x=current[axis] as f64*local[0];let z=current[6+axis] as f64*local[2];
        let value=if axis==0 {(first+z)+x}else{(first+x)+z};
        m.set_f32(vehicle+0x50c4+axis as u32*4,(value+current[9+axis] as f64) as f32);
    }
    let value=m.u32(flags)&!1;m.set_u32(flags,value);
    if player!=m.u32(0x68b990) {h.publish_flags(m,player,value);}
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn original_body_mode_fields() {
        for (case,line) in include_str!("../../../tools/emu/golden_body_modes.tsv").lines().enumerate(){
            let p:Vec<i64>=line.split('\t').map(|v|v.parse().unwrap()).collect();
            let mut m=Mem::new();let body=0x2a000000;let offsets=[0x4ac,0xf88,0xf8c];
            for (offset,value) in offsets.into_iter().zip(&p[2..5]){m.set_u32(body+offset,*value as u32);}
            let diagnostic=match p[0] {
                0x58f230=>{disable_movement(&mut m,body,p[1] as i32);false},
                0x58f290=>collision_detail(&mut m,body,p[1] as i32),
                0x58f2d0=>{motion(&mut m,body,p[1] as i32);false},
                _=>unreachable!(),
            };
            assert_eq!(offsets.map(|o|m.u32(body+o)).to_vec(),p[5..8].iter().map(|v|*v as u32).collect::<Vec<_>>(),"case {case}");
            assert_eq!(i64::from(diagnostic),p[8],"case {case} diagnostic");
        }
    }
    struct Hooks {matrices:[[f32;12];2],reads:usize,trace:Vec<String>,speed:f64,direction:[f32;3]}
    impl TrajectoryHooks for Hooks {
        fn mode(&mut self,_:&mut Mem,v:u32,value:i32){assert_eq!(v,0x2a000000);self.trace.push(format!("mode:{value}"));}
        fn movement(&mut self,_:&mut Mem,v:u32,value:i32){assert_eq!(v,0x2a000010);self.trace.push(format!("move:{value}"));}
        fn collision_detail(&mut self,_:&mut Mem,v:u32,value:i32){assert_eq!(v,0x2a000010);self.trace.push(format!("collision:{value}"));}
        fn world_transform(&mut self,_:&mut Mem,v:u32)->[f32;12] {
            assert_eq!(v,0x2a000010);self.trace.push(format!("world:{}",self.reads));
            let result=self.matrices[self.reads];self.reads+=1;result
        }
        fn publish_flags(&mut self,_:&mut Mem,player:u32,flags:u32){self.trace.push(format!("flags:{player},{flags}"));}
    }
    impl PhysicsHooks for Hooks {
        fn player_speed(&mut self,_:&mut Mem,player:u32)->f64 {self.trace.push(format!("speed:{player}"));self.speed}
        fn player_direction(&mut self,_:&mut Mem,_:u32)->[f32;3] {self.trace.push("direction".into());self.direction}
        fn linear_velocity(&mut self,_:&mut Mem,body:u32,value:[f32;3]) {
            assert_eq!(body,0x2a000010);self.trace.push(format!("linear:{}",value.map(|v|v.to_bits().to_string()).join(",")));
        }
        fn angular_velocity(&mut self,_:&mut Mem,body:u32,value:[f32;3]) {
            assert_eq!(body,0x2a000010);self.trace.push(format!("angular:{}",value.map(|v|v.to_bits().to_string()).join(",")));
        }
        fn motion(&mut self,_:&mut Mem,body:u32,value:i32) {assert_eq!(body,0x2a000010);self.trace.push(format!("motion:{value}"));}
    }
    #[test]
    fn original_complete_enter_physics() {
        let parse=|s:&str|s.split(',').map(|v|f32::from_bits(v.parse().unwrap())).collect::<Vec<_>>();
        for (case,line) in include_str!("../../../tools/emu/golden_ai_enter_physics.tsv").lines().enumerate(){
            let p:Vec<_>=line.split('\t').collect();let mut m=Mem::new();let vehicle=0x2a000000;let flags=vehicle+0x10000;
            crate::globals::install_constants(&mut m);m.set_u32(0x68b990,0x77777777);
            m.set_u32(flags,p[2].parse().unwrap());m.set_u32(vehicle+0x51e4,p[3].parse().unwrap());
            let mut hooks=Hooks{matrices:[parse(p[6]).try_into().unwrap();2],reads:0,trace:vec![],speed:f64::from_bits(p[4].parse().unwrap()),direction:parse(p[5]).try_into().unwrap()};
            enter_physics(&mut m,vehicle,p[0].parse().unwrap(),flags,p[1]=="1",&mut hooks);
            assert_eq!(m.u32(flags),p[7].parse::<u32>().unwrap(),"case {case} flags");
            assert_eq!(hooks.trace.join(";"),p[8],"case {case} call order/velocity");
        }
    }
    #[test]
    fn original_complete_leave_physics() {
        let parse=|s:&str|s.split(',').map(|v|v.parse::<u32>().unwrap()).collect::<Vec<_>>();
        for (case,line) in include_str!("../../../tools/emu/golden_ai_leave_physics.tsv").lines().enumerate(){
            let p:Vec<_>=line.split('\t').collect();let mut m=Mem::new();let vehicle=0x2a000000;let flags=vehicle+0x10000;
            m.set_u32(0x68b990,0x77777777);m.set_u32(flags,p[1].parse().unwrap());
            for (i,value) in parse(p[2]).into_iter().enumerate(){m.set_u32(vehicle+0x5230+i as u32*4,value);}
            let mut hooks=Hooks{matrices:std::array::from_fn(|i|parse(p[3+i]).into_iter().map(f32::from_bits).collect::<Vec<_>>().try_into().unwrap()),reads:0,trace:vec![],speed:0.,direction:[0.;3]};
            leave_physics(&mut m,vehicle,p[0].parse().unwrap(),flags,&mut hooks);
            assert_eq!((0..15).map(|i|m.u32(vehicle+0x50a0+i*4)).collect::<Vec<_>>(),parse(p[5]),"case {case} pose");
            assert_eq!(m.u32(flags),p[6].parse::<u32>().unwrap(),"case {case} flags");
            assert_eq!(hooks.trace.join(";"),p[7],"case {case} call order");
        }
    }
}

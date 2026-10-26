//! Operations bridging scene vehicles and the trajectory AI.
use crate::mem::Mem;

/// State-changing part of `0x5812a0`. Returns the XY diagnostic predicate;
/// diagnostic formatting/output do not alter the body state.
pub fn body_linear_velocity(m:&mut Mem,body:u32,vector:u32)->bool {
    // Preserve sequential loads even if the caller's vector overlaps the body.
    for i in 0..3 {let value=m.u32(vector+i*4);m.set_u32(body+0x1a0+i*4,value);}
    let diagnostic=crate::carv::velocity_diagnostic(m,body+0x1a0,150.);
    for i in 0..3 {m.set_u32(body+0xce4+i*4,m.u32(body+0x1a0+i*4));}
    let speed=crate::math::vec_len(m,body+0x1a0) as f32;
    m.set_f32(body+0x378,speed);m.set_f32(body+0xd84,speed);
    diagnostic
}

trait DriverSetup {
    fn place(&mut self,m:&mut Mem,driver:u32,owner:u32,position:u32);
    fn linear(&mut self,m:&mut Mem,driver:u32,vector:u32);
    fn angular(&mut self,m:&mut Mem,driver:u32,vector:u32);
}
fn driver_mode_inner(m:&mut Mem,driver:u32,mode:i32,owner:u32,position:u32,linear:u32,angular:u32,h:&mut impl DriverSetup) {
    match mode {
        0|1=>m.set_i32(driver+0x2638,mode),
        2|3=>{
            h.place(m,driver,owner,position);h.linear(m,driver,linear);h.angular(m,driver,angular);
            m.set_i32(driver+0x2638,mode);
        }
        _=>{},
    }
}
/// `0x4fc930`: activate the physical driver using the existing original
/// placement and velocity setters. Repeated modes 2/3 still perform setup.
pub fn driver_mode(m:&mut Mem,world:&mut dyn crate::contact::World,driver:u32,mode:i32,owner:u32,position:u32,linear:u32,angular:u32) {
    struct Setup<'a>(&'a mut dyn crate::contact::World);
    impl DriverSetup for Setup<'_> {
        fn place(&mut self,m:&mut Mem,d:u32,o:u32,p:u32){crate::ctor::fn_004f1240(m,self.0,d,2,o,p);}
        fn linear(&mut self,m:&mut Mem,d:u32,v:u32){crate::ctor::fn_004f1dd0(m,d,v);}
        fn angular(&mut self,m:&mut Mem,d:u32,v:u32){crate::ctor::fn_004f1ef0(m,d,v);}
    }
    driver_mode_inner(m,driver,mode,owner,position,linear,angular,&mut Setup(world));
}

pub trait VehicleModeHooks {
    fn excessive_velocity(&mut self,m:&mut Mem,vector:u32,limit:f32)->bool {crate::carv::fn_004e28b0(m,vector,limit)!=0}
    fn clear_linear_velocity(&mut self,m:&mut Mem,body:u32) {let _=body_linear_velocity(m,body,0x702650);}
    fn check_velocity(&mut self,m:&mut Mem,vector:u32,limit:f32);
    /// `0x4fc930`, with arguments 3, vehicle+0x330, +0x204, +0x1b0, +0x1bc.
    fn start_physics(&mut self,m:&mut Mem,driver:u32,owner:u32,position:u32,linear:u32,angular:u32);
}
/// Complete mode orchestration `0x54ca60`; validation and driver activation
/// remain explicit subsystem boundaries.
pub fn vehicle_mode(m:&mut Mem,vehicle:u32,value:i32,h:&mut impl VehicleModeHooks) {
    if m.i32(vehicle+0x2aac)==1 && value!=1 {
        let driver=m.u32(vehicle+0x5460);m.set_u32(driver+0x2638,0);
    }
    if value==1 {
        if m.i32(vehicle+0x2aac)!=1 && (m.u32(vehicle+0x51ac)==0 || m.i32(vehicle+0xf98)==3) {
            if m.i32(vehicle+0x2aac)==2 && h.excessive_velocity(m,vehicle+0x1b0,100.) {
                h.clear_linear_velocity(m,vehicle+0x10);
            }
            h.check_velocity(m,vehicle+0x1b0,100.);
            let driver=m.u32(vehicle+0x5460);let owner=m.u32(vehicle+0x330);
            h.start_physics(m,driver,owner,vehicle+0x204,vehicle+0x1b0,vehicle+0x1bc);
            m.set_u32(vehicle+0x2aac,1);m.set_u32(vehicle+0x51ac,0);
        }
    }else if value==2 {
        for offset in [0x50d0,0x50e8] {
            let zero=m.read(0x702650,12);m.write(vehicle+offset,&zero);
        }
    }
    m.set_i32(vehicle+0x2aac,value);
}

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
    fn linear_velocity(&mut self,m:&mut Mem,body:u32,value:[f32;3]) {
        const VECTOR:u32=0x2ffd_4b30;
        for (i,v) in value.into_iter().enumerate(){m.set_f32(VECTOR+i as u32*4,v);}
        let _=body_linear_velocity(m,body,VECTOR);
    }
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
    fn original_body_linear_velocity_state() {
        for (case,line) in include_str!("../../../tools/emu/golden_body_velocity.tsv").lines().enumerate(){
            let p:Vec<_>=line.split('\t').collect();let mut m=Mem::new();let body=0x2a000000;let source=p[0].parse::<u32>().unwrap();
            for (i,value) in p[1].split(',').enumerate(){m.set_u32(source+i as u32*4,value.parse().unwrap());}
            let diagnostic=body_linear_velocity(&mut m,body,source);
            let offsets=[0x1a0,0x1a4,0x1a8,0xce4,0xce8,0xcec,0x378,0xd84];
            assert_eq!(offsets.map(|o|m.u32(body+o)).to_vec(),p[2].split(',').map(|v|v.parse::<u32>().unwrap()).collect::<Vec<_>>(),"case {case} state");
            assert_eq!(u32::from(diagnostic),p[3].parse::<u32>().unwrap(),"case {case} diagnostic");
        }
    }
    #[test]
    fn original_driver_activation_dispatch() {
        struct Setup(Vec<String>);
        impl DriverSetup for Setup {
            fn place(&mut self,m:&mut Mem,d:u32,o:u32,p:u32){assert_eq!(d,0x2a000000);self.0.push(format!("place:2,{o},{p}"));m.set_u32(d+0x2638,0x12340003);}
            fn linear(&mut self,m:&mut Mem,d:u32,v:u32){assert_eq!(d,0x2a000000);self.0.push(format!("linear:{v}"));m.set_u32(d+0x2638,0x12340001);}
            fn angular(&mut self,m:&mut Mem,d:u32,v:u32){assert_eq!(d,0x2a000000);self.0.push(format!("angular:{v}"));m.set_u32(d+0x2638,0x12340001);}
        }
        for (case,line) in include_str!("../../../tools/emu/golden_driver_mode.tsv").lines().enumerate(){
            let p:Vec<_>=line.split('\t').collect();let v:Vec<i64>=p[0].split(',').map(|x|x.parse().unwrap()).collect();
            let mut m=Mem::new();let driver=0x2a000000;m.set_u32(driver+0x2638,v[1] as u32);
            let mut setup=Setup(vec![]);
            driver_mode_inner(&mut m,driver,v[0] as i32,v[2] as u32,v[3] as u32,v[4] as u32,v[5] as u32,&mut setup);
            assert_eq!(m.u32(driver+0x2638),p[1].parse::<u32>().unwrap(),"case {case} mode");
            assert_eq!(setup.0.join(";"),p[2],"case {case} setup order");
        }
    }
    struct ModeHooks {excessive:bool,trace:Vec<String>}
    impl VehicleModeHooks for ModeHooks {
        fn excessive_velocity(&mut self,_:&mut Mem,v:u32,limit:f32)->bool {assert_eq!((v,limit),(0x2a0001b0,100.));self.trace.push("excessive".into());self.excessive}
        fn clear_linear_velocity(&mut self,m:&mut Mem,body:u32) {
            assert_eq!(body,0x2a000010);self.trace.push(format!("clear:{}",(0..3).map(|i|m.u32(0x702650+i*4).to_string()).collect::<Vec<_>>().join(",")));
        }
        fn check_velocity(&mut self,_:&mut Mem,v:u32,limit:f32) {assert_eq!((v,limit),(0x2a0001b0,100.));self.trace.push("check".into());}
        fn start_physics(&mut self,_:&mut Mem,driver:u32,owner:u32,p:u32,l:u32,a:u32) {
            assert_eq!(driver,0x2a010000);self.trace.push(format!("start:3,{owner},{p},{l},{a}"));
        }
    }
    #[test]
    fn original_vehicle_mode_orchestration() {
        for (case,line) in include_str!("../../../tools/emu/golden_vehicle_mode.tsv").lines().enumerate(){
            let p:Vec<_>=line.split('\t').collect();let values:Vec<i64>=p[0].split(',').map(|v|v.parse().unwrap()).collect();
            let mut m=Mem::new();let vehicle=0x2a000000;let driver=vehicle+0x10000;
            let offsets=[0x2aac,0x51ac,0x50d0,0x50d4,0x50d8,0x50e8,0x50ec,0x50f0];
            for (offset,value) in offsets.into_iter().zip(&values[5..]){m.set_u32(vehicle+offset,*value as u32);}
            m.set_u32(vehicle+0x5460,driver);m.set_u32(vehicle+0xf98,values[1] as u32);m.set_u32(vehicle+0x330,values[3] as u32);m.set_u32(driver+0x2638,values[4] as u32);
            for (i,value) in p[1].split(',').enumerate(){m.set_u32(0x702650+i as u32*4,value.parse().unwrap());}
            let mut h=ModeHooks{excessive:values[2]!=0,trace:vec![]};vehicle_mode(&mut m,vehicle,values[0] as i32,&mut h);
            let actual=offsets.into_iter().map(|o|m.u32(vehicle+o)).chain([m.u32(driver+0x2638)]).collect::<Vec<_>>();
            assert_eq!(actual,p[2].split(',').map(|v|v.parse::<u32>().unwrap()).collect::<Vec<_>>(),"case {case} state");
            assert_eq!(h.trace.join(";"),p[3],"case {case} calls");
        }
    }
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

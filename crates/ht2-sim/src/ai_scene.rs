//! Operations bridging scene vehicles and the trajectory AI.
use crate::mem::Mem;

pub trait ObstacleSourceHooks {
    /// Kind from the original 0x43fc40 snapshot query.
    fn player_kind(&mut self,m:&mut Mem,vehicle:u32)->i32;
    fn world_transform(&mut self,m:&mut Mem,body:u32)->[f32;12];
}
/// Command-29 preparation in `0x4ce300`, before queue rotation/allocation.
/// Returns the subcode and signed plane distance written to the new command.
pub fn prepare_obstacle_command(m:&mut Mem,command:u32,vehicle:u32,h:&mut impl ObstacleSourceHooks)->Option<(i32,f32)> {
    let state=obstacle_subcode(m,command,vehicle,h)?;
    Some((state,obstacle_distance(m,command,vehicle,h)))
}
fn obstacle_subcode(m:&mut Mem,command:u32,vehicle:u32,h:&mut impl ObstacleSourceHooks)->Option<i32> {
    if m.u32(command)!=29 {return None;}
    let controller=m.u32(0x696338);if controller==0 {return None;}
    if h.player_kind(m,vehicle)==5 && m.u32(vehicle+0x55a4)!=0 {return None;}
    let controller=m.u32(0x696338);
    let code=m.i32(command+4);let mut state=m.u32(controller+0x24);
    if code%10!=0 {state^=1;}
    match code/10 {1|3=>state^=1,4=>state=1,_=>{}}
    Some(state as i32)
}
fn obstacle_distance(m:&mut Mem,command:u32,vehicle:u32,h:&mut impl ObstacleSourceHooks)->f32 {
    let plane=m.u32(command+12);let mut distance=100.;
    if plane!=0 {
        let a:[f64;4]=std::array::from_fn(|i|m.f32(plane+i as u32*4) as f64);
        let transform=h.world_transform(m,vehicle+0x10);
        let raw=((transform[11] as f64*a[2]+transform[10] as f64*a[1])+transform[9] as f64*a[0])+a[3];
        distance=raw as f32;
        let mut positive=false;
        if !(raw>=0.) {
            let transform=h.world_transform(m,vehicle+0x10);
            let forward:[f64;3]=std::array::from_fn(|axis|{
                (m.f32(0x702628) as f64*transform[6+axis] as f64+m.f32(0x702624) as f64*transform[3+axis] as f64)
                    +m.f32(0x702620) as f64*transform[axis] as f64
            });
            positive=(forward[0]*a[0]+forward[1]*a[1])+forward[2]*a[2]>0.;
        }
        distance=if positive {distance.abs()}else{-distance.abs()};
    }
    distance
}

/// Complete command-29 branch of 0x4ce300. Other message types are not handled.
pub fn receive_obstacle_command(m:&mut Mem,command:u32,vehicle:u32,h:&mut impl ObstacleSourceHooks,heap:&mut impl CommandHeap)->bool {
    let Some(state)=obstacle_subcode(m,command,vehicle,h) else{return false};
    let temporary=heap.allocate(m,4);
    let distance=obstacle_distance(m,command,vehicle,h);
    queue_prepared_obstacle(m,vehicle,state,distance,temporary,heap);true
}

pub trait CommandHeap {
    fn allocate(&mut self,m:&mut Mem,bytes:u32)->u32;
    fn release(&mut self,m:&mut Mem,address:u32);
}

/// Queue rotation/allocation of the command-29 branch in `0x4ce300`.
/// The temporary payload allocation precedes the geometric preparation in the
/// original; callers which need that ordering can allocate it before preparing.
pub fn queue_prepared_obstacle(m:&mut Mem,vehicle:u32,subcode:i32,distance:f32,temporary:u32,heap:&mut impl CommandHeap) {
    m.set_f32(temporary,distance);
    let old=m.u32(vehicle+0x5574);
    if old!=0 {
        let data=m.u32(old+12);if data!=0 {heap.release(m,data);}
        heap.release(m,old);
    }
    m.set_u32(vehicle+0x5574,m.u32(vehicle+0x5570));
    let copy=heap.allocate(m,16);
    if copy!=0 {
        m.set_u32(copy,29);m.set_i32(copy+4,subcode);m.set_u32(copy+8,1);
        let payload=heap.allocate(m,4);m.set_u32(copy+12,payload);m.set_u32(payload,m.u32(temporary));
    }
    m.set_u32(vehicle+0x5570,copy);
    if temporary!=0 {heap.release(m,temporary);}
}
/// `0x4cfc30`: replace one of two owned commands with a deep copy.
/// Allocation/free remain explicit; Mem itself has a non-freeing bump allocator.
pub fn replace_command(m:&mut Mem,commands:u32,source:u32,slot:u32,heap:&mut impl CommandHeap) {
    if slot>=2 {return;}
    let target=commands+slot*4;let old=m.u32(target);
    if old!=0 {
        let data=m.u32(old+12);if data!=0 {heap.release(m,data);}
        heap.release(m,old);
    }
    let copy=if source==0 {0}else{
        let object=heap.allocate(m,16);
        if object!=0 {
            for offset in [0,4,8] {m.set_u32(object+offset,m.u32(source+offset));}
            let count=m.u32(object+8);let data=heap.allocate(m,count.wrapping_mul(4));
            m.set_u32(object+12,data);let input=m.u32(source+12);
            for i in 0..(count&0x3fffffff) {m.set_u32(data+i*4,m.u32(input+i*4));}
        }
        object
    };
    m.set_u32(target,copy);
}

pub trait ObstacleHooks {
    /// 0x4cfc30 with null command and slot 0, on the two-slot command owner.
    fn consume(&mut self,m:&mut Mem,commands:u32);
    /// 0x447180, after copying player identity with 0x41d300.
    fn publish(&mut self,m:&mut Mem,player:u32,clear:bool,distance:f64);
}

/// State writes of `0x447180`; caller supplies a validated AI player identity.
pub fn publish_obstacle(m:&mut Mem,player:u32,clear:bool,distance:f64) {
    m.wr8(player+0x3d0,u8::from(clear));m.set_f64(player+0x3d8,distance);
}
/// Command consumer and AI publisher using the original memory layouts.
/// Heap ownership is supplied by the application, not emulated by leaking frees.
pub struct OwnedObstacle<'a,H:CommandHeap> {pub heap:&'a mut H}
impl<H:CommandHeap> ObstacleHooks for OwnedObstacle<'_,H> {
    fn consume(&mut self,m:&mut Mem,commands:u32){replace_command(m,commands,0,0,self.heap);}
    fn publish(&mut self,m:&mut Mem,player:u32,clear:bool,distance:f64){publish_obstacle(m,player,clear,distance);}
}

/// Bridge a validated scene player ID to its actual trajectory-world player.
pub struct PlayerObstacle<'a,H:CommandHeap> {
    pub heap:&'a mut H,
    pub player_id:u32,
    pub player:&'a mut crate::world::AiPlayer,
}
impl<H:CommandHeap> ObstacleHooks for PlayerObstacle<'_,H> {
    fn consume(&mut self,m:&mut Mem,commands:u32){replace_command(m,commands,0,0,self.heap);}
    fn publish(&mut self,m:&mut Mem,player:u32,clear:bool,distance:f64){
        assert_eq!(player,self.player_id,"scene player identity mismatch");
        publish_obstacle(m,player,clear,distance);self.player.prescribe_obstacle(clear,distance);
    }
}
/// Command/obstacle tail `0x4cfb51..0x4cfbf7` of the common AI update.
pub fn update_obstacle_command(m:&mut Mem,vehicle:u32,player:u32,flags:u32,h:&mut impl ObstacleHooks) {
    let driver=m.u32(vehicle+0x5460);m.set_f64(driver+0x40b0,-1.);
    let command=m.u32(vehicle+0x5570);
    if m.u32(vehicle+0x5574)==0 && command!=0 && m.u32(command)==29 {
        let distance=m.f32(m.u32(command+12));
        if distance>0. && m.u32(command+4)==0 {
            let target=m.u32(vehicle+0x5460);m.set_f64(target+0x40b0,distance as f64);
        }
        h.consume(m,vehicle+0x5570);
    }
    if m.u32(flags)&1==0 {
        let distance=m.f64(driver+0x40b0);
        h.publish(m,player,!(distance>0.),distance);
    }
}

trait ModeDispatch {
    fn enter_needed(&mut self,m:&mut Mem,vehicle:u32)->bool;
    fn leave_needed(&mut self,m:&mut Mem,vehicle:u32)->bool;
    fn enter(&mut self,m:&mut Mem,vehicle:u32,player:u32,flags:u32);
    fn leave(&mut self,m:&mut Mem,vehicle:u32,player:u32,flags:u32);
}
fn update_physics_mode_inner(m:&mut Mem,vehicle:u32,player:u32,flags:u32,h:&mut impl ModeDispatch) {
    if m.u32(flags)&1!=0 {
        if h.leave_needed(m,vehicle){h.leave(m,vehicle,player,flags);}
    }else if h.enter_needed(m,vehicle){h.enter(m,vehicle,player,flags);}
    let driver=m.u32(vehicle+0x5460);
    m.set_u32(driver+0x4078,u32::from(m.u32(flags)&0x80==0));
}
/// Mode block `0x4cf7e8..0x4cf861` of the common AI vehicle update.
/// Call for AI vehicles after the earlier trajectory synchronization; this is
/// not a replacement for the full 0x4cf620 update or its human early return.
pub fn update_physics_mode(m:&mut Mem,vehicle:u32,player:u32,flags:u32,h:&mut impl PhysicsHooks) {
    struct Dispatch<'a,H>(&'a mut H);
    impl<H:PhysicsHooks> Dispatch<'_,H> {
        fn positions(&mut self,m:&mut Mem,v:u32)->([f32;3],[f32;3]) {
            let human=m.u32(m.u32(0x6d2078)+0x268);
            let a=self.0.world_transform(m,human+0x10);let b=self.0.world_transform(m,v+0x10);
            (a[9..].try_into().unwrap(),b[9..].try_into().unwrap())
        }
    }
    impl<H:PhysicsHooks> ModeDispatch for Dispatch<'_,H> {
        fn enter_needed(&mut self,m:&mut Mem,v:u32)->bool {let(a,b)=self.positions(m,v);crate::ai::fn_004cedd0(a,b)}
        fn leave_needed(&mut self,m:&mut Mem,v:u32)->bool {let(a,b)=self.positions(m,v);crate::ai::fn_004cee70(m,v,a,b)}
        fn enter(&mut self,m:&mut Mem,v:u32,p:u32,f:u32){enter_physics(m,v,p,f,true,self.0);}
        fn leave(&mut self,m:&mut Mem,v:u32,p:u32,f:u32){leave_physics(m,v,p,f,self.0);}
    }
    update_physics_mode_inner(m,vehicle,player,flags,&mut Dispatch(h));
}

/// State-changing part of `0x581430`. Components within +/-10 are copied;
/// larger finite vectors up to +/-10000 are normalized to length 10. Beyond
/// that bound the previous angular velocity is retained. Returns XY diagnostic.
pub fn body_angular_velocity(m:&mut Mem,body:u32,vector:u32)->bool {
    let mut diagnostic=false;
    if crate::carv::fn_004e28b0(m,vector,10.)==0 {
        for i in 0..3 {let value=m.u32(vector+i*4);m.set_u32(body+0x1ac+i*4,value);}
    }else if crate::carv::fn_004e28b0(m,vector,10000.)==0 {
        const UNIT:u32=0x2ffd_4b50;
        crate::matrix::vec_unit(m,UNIT,vector);
        for i in 0..3 {m.set_f32(body+0x1ac+i*4,(m.f32(UNIT+i*4) as f64*10.) as f32);}
    }else{diagnostic=crate::carv::velocity_diagnostic(m,vector,10000.);}
    let weighted:[f64;3]=std::array::from_fn(|i|(m.f32(body+0x1ac+i as u32*4) as f64*m.f32(body+0x194+i as u32*4) as f64) as f32 as f64);
    let momentum:[f32;3]=std::array::from_fn(|axis|{
        let f=|i:usize|m.f32(body+0x1f4+(i*3+axis) as u32*4) as f64;
        ((f(2)*weighted[2]+f(1)*weighted[1])+f(0)*weighted[0]) as f32
    });
    for (i,value) in momentum.into_iter().enumerate(){m.set_f32(body+0x1b8+i as u32*4,value);}
    for i in 0..6 {m.set_u32(body+0xcf0+i*4,m.u32(body+0x1ac+i*4));}
    diagnostic
}

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

/// Concrete mode owner using the ported physical driver and the caller's world.
/// Diagnostic events are retained for the application to display/log.
pub struct PhysicalMode<'a> {
    pub world:&'a mut dyn crate::contact::World,
    pub diagnostics:Vec<(u32,f32)>,
}
impl VehicleModeHooks for PhysicalMode<'_> {
    fn clear_linear_velocity(&mut self,m:&mut Mem,body:u32) {let _=body_linear_velocity(m,body,0x702650);}
    fn check_velocity(&mut self,m:&mut Mem,vector:u32,limit:f32) {
        if crate::carv::velocity_diagnostic(m,vector,limit){self.diagnostics.push((vector,limit));}
    }
    fn start_physics(&mut self,m:&mut Mem,driver:u32,owner:u32,position:u32,linear:u32,angular:u32) {
        driver_mode(m,self.world,driver,3,owner,position,linear,angular);
    }
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
    fn angular_velocity(&mut self,m:&mut Mem,body:u32,value:[f32;3]) {
        const VECTOR:u32=0x2ffd_4b40;
        for (i,v) in value.into_iter().enumerate(){m.set_f32(VECTOR+i as u32*4,v);}
        let _=body_angular_velocity(m,body,VECTOR);
    }
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
    fn original_complete_obstacle_delivery() {
        use std::{cell::RefCell,rc::Rc};
        struct Source {kind:i32,matrix:[f32;12],trace:Rc<RefCell<Vec<String>>>}
        impl ObstacleSourceHooks for Source {
            fn player_kind(&mut self,_:&mut Mem,v:u32)->i32{assert_eq!(v,0x2a010000);self.trace.borrow_mut().push("kind".into());self.kind}
            fn world_transform(&mut self,_:&mut Mem,b:u32)->[f32;12]{assert_eq!(b,0x2a010010);self.trace.borrow_mut().push("world".into());self.matrix}
        }
        struct Heap {fail:bool,calls:usize,trace:Rc<RefCell<Vec<String>>>}
        impl CommandHeap for Heap {
            fn allocate(&mut self,_:&mut Mem,bytes:u32)->u32{self.trace.borrow_mut().push(format!("alloc:{bytes}"));let a=[0x2a022000,if self.fail{0}else{0x2a022100},0x2a022200][self.calls];self.calls+=1;a}
            fn release(&mut self,_:&mut Mem,a:u32){self.trace.borrow_mut().push(format!("free:{a}"));}
        }
        let parse=|s:&str|s.split(',').map(|x|x.parse::<i64>().unwrap()).collect::<Vec<_>>();
        for (case,line) in include_str!("../../../tools/emu/golden_obstacle_receive.tsv").lines().enumerate(){
            let p:Vec<_>=line.split('\t').collect();let v=parse(p[0]);let mut m=Mem::new();let command=0x2a000000;let vehicle=command+0x10000;let controller=vehicle+0x10000;let plane=controller+0x1000;
            m.set_u32(command,29);m.set_u32(command+4,v[3] as u32);m.set_u32(command+12,if v[5]!=0{plane}else{0});m.set_u32(0x696338,if v[0]!=0{controller}else{0});m.set_u32(controller+0x24,v[4] as u32);m.set_u32(vehicle+0x55a4,v[2] as u32);
            m.set_f32(0x702624,1.);for (i,value) in parse(p[2]).into_iter().enumerate(){m.set_u32(plane+i as u32*4,value as u32);}
            m.set_u32(vehicle+0x5570,v[7] as u32);m.set_u32(vehicle+0x5574,v[8] as u32);m.set_u32(0x2a022400+12,0x2a022440);
            for (a,n) in [(0x2a022000,4),(0x2a022100,16),(0x2a022200,4)]{m.write(a,&vec![0xcd;n]);}
            let trace=Rc::new(RefCell::new(vec![]));let mut source=Source{kind:v[1] as i32,matrix:parse(p[1]).into_iter().map(|x|f32::from_bits(x as u32)).collect::<Vec<_>>().try_into().unwrap(),trace:trace.clone()};let mut heap=Heap{fail:v[6]!=0,calls:0,trace:trace.clone()};
            receive_obstacle_command(&mut m,command,vehicle,&mut source,&mut heap);
            let actual=[vehicle+0x5570,vehicle+0x5574,0x2a022000,0x2a022100,0x2a022104,0x2a022108,0x2a02210c,0x2a022200].map(|a|m.u32(a)).to_vec();
            assert_eq!(actual,parse(p[3]).into_iter().map(|x|x as u32).collect::<Vec<_>>(),"case {case} queue");assert_eq!(trace.borrow().join(";"),p[4],"case {case} order");
        }
    }
    #[test]
    fn original_obstacle_queue_rotation() {
        struct Heap {fail:bool,calls:usize,trace:Vec<String>}
        impl CommandHeap for Heap {
            fn allocate(&mut self,_:&mut Mem,bytes:u32)->u32 {self.calls+=1;self.trace.push(format!("alloc:{bytes}"));if self.calls==1 {if self.fail {0}else{0x2a040000}}else{0x2a040100}}
            fn release(&mut self,_:&mut Mem,address:u32){self.trace.push(format!("free:{address}"));}
        }
        for (case,line) in include_str!("../../../tools/emu/golden_obstacle_queue.tsv").lines().enumerate(){
            let p:Vec<_>=line.split('\t').collect();let v:Vec<u32>=p[0].split(',').map(|x|x.parse().unwrap()).collect();
            let mut m=Mem::new();let vehicle=0x2a000000;let temporary=vehicle+0x10000;let copy=vehicle+0x40000;
            m.set_u32(vehicle+0x5570,v[0]);m.set_u32(vehicle+0x5574,v[1]);m.set_u32(vehicle+0x31000+12,v[2]);
            m.write(copy,&[0xcd;16]);m.write(copy+0x100,&[0xcd;4]);
            let mut heap=Heap{fail:v[5]!=0,calls:0,trace:vec![]};
            queue_prepared_obstacle(&mut m,vehicle,v[3] as i32,f32::from_bits(v[4]),temporary,&mut heap);
            let actual=[vehicle+0x5570,vehicle+0x5574,temporary,copy,copy+4,copy+8,copy+12,copy+0x100].map(|a|m.u32(a)).to_vec();
            assert_eq!(actual,p[1].split(',').map(|x|x.parse::<u32>().unwrap()).collect::<Vec<_>>(),"case {case} queue");
            assert_eq!(heap.trace.join(";"),p[2],"case {case} allocation/free");
        }
    }
    #[test]
    fn original_obstacle_source_preparation() {
        struct Source {kind:i32,transform:[f32;12]}
        impl ObstacleSourceHooks for Source {
            fn player_kind(&mut self,_:&mut Mem,v:u32)->i32{assert_eq!(v,0x2a010000);self.kind}
            fn world_transform(&mut self,_:&mut Mem,b:u32)->[f32;12]{assert_eq!(b,0x2a010010);self.transform}
        }
        for (case,line) in include_str!("../../../tools/emu/golden_obstacle_source.tsv").lines().enumerate(){
            let p:Vec<_>=line.split('\t').collect();let v:Vec<i64>=p[0].split(',').map(|x|x.parse().unwrap()).collect();
            let parse=|s:&str|s.split(',').map(|x|x.parse::<u32>().unwrap()).collect::<Vec<_>>();
            let mut m=Mem::new();let c=0x2a000000;let vehicle=c+0x10000;let controller=vehicle+0x10000;let plane=controller+0x1000;
            m.set_u32(c,29);m.set_u32(c+4,v[3] as u32);m.set_u32(c+12,if v[5]!=0 {plane}else{0});
            m.set_u32(0x696338,if v[0]!=0 {controller}else{0});m.set_u32(controller+0x24,v[4] as u32);m.set_u32(vehicle+0x55a4,v[2] as u32);
            for (i,value) in parse(p[2]).into_iter().enumerate(){m.set_u32(plane+i as u32*4,value);}
            m.set_f32(0x702624,1.);
            let mut source=Source{kind:v[1] as i32,transform:parse(p[1]).into_iter().map(f32::from_bits).collect::<Vec<_>>().try_into().unwrap()};
            let actual=prepare_obstacle_command(&mut m,c,vehicle,&mut source).map(|(state,distance)|vec![state as u32,distance.to_bits()]);
            assert_eq!(actual,if p[3].is_empty(){None}else{Some(parse(p[3]))},"case {case}");
        }
    }
    #[test]
    fn original_obstacle_publication_writes() {
        let bytes=|s:&str|(0..s.len()).step_by(2).map(|i|u8::from_str_radix(&s[i..i+2],16).unwrap()).collect::<Vec<_>>();
        for line in include_str!("../../../tools/emu/golden_obstacle_publish.tsv").lines(){
            let p:Vec<_>=line.split('\t').collect();let mut m=Mem::new();let player=0x2a000000;
            m.write(player+0x3d0,&bytes(p[2]));publish_obstacle(&mut m,player,p[0]=="1",f64::from_bits(p[1].parse().unwrap()));
            assert_eq!(m.read(player+0x3d0,24),bytes(p[3]));
        }
    }
    #[test]
    fn owned_obstacle_copy_consume_and_publish_chain() {
        struct Heap {freed:Vec<u32>}
        impl CommandHeap for Heap {
            fn allocate(&mut self,m:&mut Mem,bytes:u32)->u32 {m.alloc(bytes)}
            fn release(&mut self,_:&mut Mem,address:u32){self.freed.push(address);}
        }
        let mut m=Mem::new();let vehicle=0x2a000000;let driver=vehicle+0x10000;let player=vehicle+0x20000;
        let flags=vehicle+0x30000;let source=vehicle+0x40000;let data=source+0x100;
        m.set_u32(vehicle+0x5460,driver);
        for (offset,value) in [(0,29),(4,0),(8,1),(12,data)]{m.set_u32(source+offset,value);}
        m.set_f32(data,12.5);let mut heap=Heap{freed:vec![]};
        replace_command(&mut m,vehicle+0x5570,source,0,&mut heap);
        let owned=m.u32(vehicle+0x5570);let payload=m.u32(owned+12);
        assert_ne!(owned,source);assert_ne!(payload,data);
        m.set_f32(data,999.); // External caller's buffer is no longer authoritative.
        update_obstacle_command(&mut m,vehicle,player,flags,&mut OwnedObstacle{heap:&mut heap});
        assert_eq!(m.f64(driver+0x40b0),12.5);assert_eq!(m.f64(player+0x3d8),12.5);
        assert_eq!(m.rd8(player+0x3d0),0);assert_eq!(m.u32(vehicle+0x5570),0);
        assert_eq!(heap.freed,vec![payload,owned]);
        update_obstacle_command(&mut m,vehicle,player,flags,&mut OwnedObstacle{heap:&mut heap});
        assert_eq!(m.f64(player+0x3d8),-1.);assert_eq!(m.rd8(player+0x3d0),1);
        assert_eq!(heap.freed.len(),2); // The consumed command cannot be freed twice.
    }
    #[test]
    fn original_owned_command_replacement() {
        struct Heap {outer_fail:bool,inner_fail:bool,calls:usize,trace:Vec<String>}
        impl CommandHeap for Heap {
            fn allocate(&mut self,_:&mut Mem,bytes:u32)->u32 {
                self.calls+=1;self.trace.push(format!("alloc:{bytes}"));
                if self.calls==1 {if self.outer_fail {0}else{0x2a003000}}else if self.inner_fail {0}else{0x2a004000}
            }
            fn release(&mut self,_:&mut Mem,address:u32){self.trace.push(format!("free:{address}"));}
        }
        for (case,line) in include_str!("../../../tools/emu/golden_ai_commands.tsv").lines().enumerate(){
            let p:Vec<_>=line.split('\t').collect();let v:Vec<u32>=p[0].split(',').map(|x|x.parse().unwrap()).collect();
            let mut m=Mem::new();let owner=0x2a000000;let old=owner+0x1000;let source=owner+0x2000;let input=source+0x100;
            for slot in 0..2 {m.set_u32(owner+slot*4,if v[1]!=0 {old}else{0});}
            m.set_u32(old+12,if v[2]!=0 {old+0x100}else{0});
            for (i,value) in [v[6],v[7],v[8],input].into_iter().enumerate(){m.set_u32(source+i as u32*4,value);}
            for (i,value) in p[1].split(',').enumerate(){m.set_u32(input+i as u32*4,value.parse().unwrap());}
            m.write(owner+0x3000,&[0xcd;16]);m.write(owner+0x4000,&[0xcd;32]);
            let mut heap=Heap{outer_fail:v[4]!=0,inner_fail:v[5]!=0,calls:0,trace:vec![]};
            replace_command(&mut m,owner,if v[3]!=0 {source}else{0},v[0],&mut heap);
            let actual=(0..2).map(|i|m.u32(owner+i*4)).chain((0..4).map(|i|m.u32(owner+0x3000+i*4))).chain((0..8).map(|i|m.u32(owner+0x4000+i*4))).collect::<Vec<_>>();
            assert_eq!(actual,p[2].split(',').map(|x|x.parse::<u32>().unwrap()).collect::<Vec<_>>(),"case {case} copy");
            assert_eq!(heap.trace.join(";"),p[3],"case {case} heap order");
        }
    }
    #[test]
    fn original_obstacle_command_tail() {
        struct Hooks(Vec<String>);
        impl ObstacleHooks for Hooks {
            fn consume(&mut self,m:&mut Mem,commands:u32){assert_eq!(commands,0x2a005570);self.0.push("consume".into());m.set_u32(commands,0);}
            fn publish(&mut self,_:&mut Mem,player:u32,clear:bool,distance:f64){let bits=distance.to_bits();self.0.push(format!("publish:{player},{},{},{}",u32::from(clear),bits as u32,(bits>>32) as u32));}
        }
        for (case,line) in include_str!("../../../tools/emu/golden_ai_obstacle_command.tsv").lines().enumerate(){
            let p:Vec<_>=line.split('\t').collect();let v:Vec<i64>=p[0].split(',').map(|x|x.parse().unwrap()).collect();
            let mut m=Mem::new();let vehicle=0x2a000000;let driver=vehicle+0x10000;let flags=vehicle+0x20000;let command=vehicle+0x30000;let data=command+0x100;
            m.set_u32(vehicle+0x5460,driver);m.set_u32(vehicle+0x5570,if v[0]!=0 {command}else{0});m.set_u32(vehicle+0x5574,v[1] as u32);
            m.set_u32(command,v[2] as u32);m.set_u32(command+4,v[3] as u32);m.set_u32(command+12,data);m.set_u32(data,v[4] as u32);m.set_u32(flags,v[5] as u32);
            let mut h=Hooks(vec![]);update_obstacle_command(&mut m,vehicle,0x12345678,flags,&mut h);
            assert_eq!(vec![m.f64(driver+0x40b0).to_bits(),m.u32(vehicle+0x5570) as u64],p[1].split(',').map(|x|x.parse::<u64>().unwrap()).collect::<Vec<_>>(),"case {case} state");
            assert_eq!(h.0.join(";"),p[2],"case {case} calls");
        }
    }
    #[test]
    fn original_mode_dispatch_block() {
        struct Dispatch {enter:bool,leave:bool,flags:u32,driver:u32,trace:Vec<String>}
        impl ModeDispatch for Dispatch {
            fn enter_needed(&mut self,_:&mut Mem,v:u32)->bool {assert_eq!(v,0x2a000000);self.trace.push("enter?".into());self.enter}
            fn leave_needed(&mut self,_:&mut Mem,v:u32)->bool {assert_eq!(v,0x2a000000);self.trace.push("leave?".into());self.leave}
            fn enter(&mut self,m:&mut Mem,v:u32,p:u32,f:u32){self.trace.push(format!("enter:{v},{p},{f},1"));m.set_u32(f,self.flags);m.set_u32(v+0x5460,self.driver);}
            fn leave(&mut self,m:&mut Mem,v:u32,p:u32,f:u32){self.trace.push(format!("leave:{v},{p},{f}"));m.set_u32(f,self.flags);m.set_u32(v+0x5460,self.driver);}
        }
        for (case,line) in include_str!("../../../tools/emu/golden_ai_mode_dispatch.tsv").lines().enumerate(){
            let p:Vec<_>=line.split('\t').collect();let v:Vec<u32>=p[0].split(',').map(|x|x.parse().unwrap()).collect();
            let mut m=Mem::new();let vehicle=0x2a000000;let flags=vehicle+0x10000;let drivers=[vehicle+0x30000,vehicle+0x40000];
            m.set_u32(flags,v[0]);m.set_u32(vehicle+0x5460,drivers[0]);
            for (d,value) in drivers.into_iter().zip(&v[5..]){m.set_u32(d+0x4078,*value);}
            let mut h=Dispatch{enter:v[1]!=0,leave:v[2]!=0,flags:v[3],driver:v[4],trace:vec![]};
            update_physics_mode_inner(&mut m,vehicle,0x12345678,flags,&mut h);
            assert_eq!([m.u32(flags),m.u32(vehicle+0x5460),m.u32(drivers[0]+0x4078),m.u32(drivers[1]+0x4078)].to_vec(),p[1].split(',').map(|x|x.parse::<u32>().unwrap()).collect::<Vec<_>>(),"case {case}");
            assert_eq!(h.trace.join(";"),p[2],"case {case} calls");
        }
    }
    #[test]
    fn original_body_angular_velocity_state() {
        for (case,line) in include_str!("../../../tools/emu/golden_body_angular_velocity.tsv").lines().enumerate(){
            let p:Vec<_>=line.split('\t').collect();let mut m=Mem::new();let body=0x2a000000;let source=p[0].parse::<u32>().unwrap();
            for (field,index) in [(body+0x1ac,1),(body+0x194,2),(body+0x1f4,3),(source,4)] {
                for (i,value) in p[index].split(',').enumerate(){m.set_u32(field+i as u32*4,value.parse().unwrap());}
            }
            let diagnostic=body_angular_velocity(&mut m,body,source);
            let actual=(0..6).map(|i|m.u32(body+0x1ac+i*4)).chain((0..6).map(|i|m.u32(body+0xcf0+i*4))).collect::<Vec<_>>();
            let expected=p[5].split(',').map(|v|v.parse::<u32>().unwrap()).collect::<Vec<_>>();
            for (i,(a,b)) in actual.into_iter().zip(expected).enumerate(){assert!(a==b || f32::from_bits(a).is_nan() && f32::from_bits(b).is_nan(),"case {case} word {i}: {a} != {b}");}
            assert_eq!(u32::from(diagnostic),p[6].parse::<u32>().unwrap(),"case {case} diagnostic");
        }
    }
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

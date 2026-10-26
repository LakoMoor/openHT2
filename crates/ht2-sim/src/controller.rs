//! Original scenario-controller reset. Resource destruction and the base reset
//! remain explicit callbacks, so a caller cannot silently skip their side effects.
use crate::mem::Mem;
/// `0x5dbd30`: construct the scenario camera transform, including the added
/// heading offset at +0xb88, then invert it for scene traversal.
pub fn camera_matrix(m:&mut Mem,c:u32,destination:u32) {
    const TEMP:u32=0x2ffd_3000;
    let (x,y,z)=(m.f64(c+0xb58),m.f64(c+0xb60),m.f64(c+0xb68));
    let (heading,pitch,roll)=(m.f64(c+0xb70)+m.f64(c+0xb88),m.f64(c+0xb78),m.f64(c+0xb80));
    crate::matrix::mat_from_euler(m,TEMP,x,y,z,heading,pitch,roll);
    crate::math::mat_inverse(m,destination,TEMP);
}
pub trait ScenePassHooks {
    fn overlay(&mut self,m:&mut Mem,object:u32);
    fn profile(&mut self,begin:bool);
    fn prepare_scene(&mut self,m:&mut Mem,scene:u32,phase:u32);
    fn transform_scene(&mut self,m:&mut Mem,scene:u32);
    fn update_object(&mut self,m:&mut Mem,object:u32,mode:u32,matrix:&[u8;48],scene:u32);
    fn render(&mut self,m:&mut Mem,controller:u32);
}
/// `0x5df6f0`, controller slot 0x64: ordinary and optional reflected scene pass.
/// Camera and reflected matrices use ported original mathematics directly;
/// object/render virtual methods remain explicit effects.
pub fn scene_pass(m:&mut Mem,c:u32,audio:bool,h:&mut impl ScenePassHooks) {
    let overlay=m.u32(c+0xb2c);
    if overlay!=0 {h.overlay(m,overlay);}
    if m.u32(c+0x1e8)==0 {return;}
    const CAMERA:u32=0x2ffd_3040;
    camera_matrix(m,c,CAMERA);
    let matrix:[u8;48]=m.read(CAMERA,48).try_into().unwrap();h.profile(true);
    if m.u32(0x702e8c)&0x100!=0 {h.prepare_scene(m,m.u32(c+0x1d0),m.u32(c+0x224));}
    for offset in [0,4,8,12] {let scene=m.u32(c+0x1d0);m.set_u32(scene+0x2120+offset,m.u32(c+0xb90+offset));}
    let camera=m.u32(c+0x1e8);
    if camera!=0 && m.u32(camera+0x4c)!=0 {
        let scene=m.u32(c+0x1d0);h.transform_scene(m,scene);
        let camera=m.u32(c+0x1e8);let object=if camera==0 {0}else{m.u32(camera+0x4c)};
        h.update_object(m,object,if audio{1}else{5},&matrix,m.u32(c+0x1d0));
    }
    h.render(m,c);
    let overlay=m.u32(c+0xb2c);
    if overlay!=0 {
        let scene=m.u32(c+0x1d0);m.set_u32(scene+0x21dc,1);
        let scene=m.u32(c+0x1d0);
        crate::math::mat_inverse(m,scene+0x2174,overlay+0x398);
        h.render(m,c);
        let scene=m.u32(c+0x1d0);m.set_u32(scene+0x21dc,0);
    }
    h.profile(false);
}
/// Runtime for the original recurring internal tasks. Effects are supplied by
/// the game controller; task completion schedules its next tick internally.
pub trait InternalEffects {
    fn call(&mut self,m:&mut Mem,controller:u32,slot:u32);
}
pub struct InternalScheduler {
    pub memory: Mem,
    pub manager: u32,
    pub controller: u32,
    free: Vec<u32>,
}
struct InternalRunner<'a,H> { effects:&'a mut H,free:&'a mut Vec<u32> }
impl<H:InternalEffects> TaskHooks for InternalRunner<'_,H> {
    fn call(&mut self,m:&mut Mem,c:u32,s:u32){self.effects.call(m,c,s);}
    fn complete(&mut self,m:&mut Mem,task:u32){
        let manager=m.u32(task+12);
        if manager==0{return;}
        let r=self.free.pop().unwrap_or_else(||m.alloc(24));m.write(r,&[0;24]);
        m.set_u32(r+12,task);m.set_u32(r+16,1);
        if !enqueue(m,manager,r){self.free.push(r);}
    }
}
impl<H:InternalEffects> RequestHooks for InternalRunner<'_,H> {
    fn run(&mut self,m:&mut Mem,task:u32){execute_task(m,task,self);}
    fn remove_task(&mut self,_:&mut Mem,_:u32,_:u32){unreachable!("internal runtime creates only run/activate requests");}
    fn destroy(&mut self,_:&mut Mem,object:u32){self.free.push(object);}
}
impl<H:InternalEffects> SchedulerHooks for InternalRunner<'_,H> {
    fn execute(&mut self,m:&mut Mem,c:u32,r:u32){execute_request(m,c,r,self);}
}
impl InternalScheduler {
    pub fn new(flags:u32,extra:bool)->Self {
        let mut memory=Mem::new();let manager=memory.alloc(32);let controller=memory.alloc(0x2100);
        for id in internal_tasks(flags,extra) {
            let task=memory.alloc(40);memory.set_u32(task+12,manager);memory.set_i32(task+20,id);
            memory.set_u32(task+32,controller);memory.set_i32(task+36,id);
            let r=memory.alloc(24);memory.set_u32(r+12,task);memory.set_u32(r+20,2);
            assert!(enqueue(&mut memory,manager,r));
        }
        Self {memory,manager,controller,free:vec![]}
    }
    pub fn tick(&mut self,effects:&mut impl InternalEffects) {
        let mut runner=InternalRunner{effects,free:&mut self.free};
        scheduler_tick(&mut self.memory,self.manager,&mut runner);
    }
}
pub trait FrameBeginHooks {
    fn clear_frame(&mut self,m:&mut Mem,scene:u32);
    fn copy_palette(&mut self,m:&mut Mem,destination:u32,source:u32);
}
/// Complete `0x5de600` handler, with clock/rendering services supplied by caller.
pub fn begin_frame(m:&mut Mem,c:u32,milliseconds:f64,overhead_ms:f64,h:&mut impl FrameBeginHooks) {
    begin_frame_time(m,c,milliseconds,overhead_ms);
    h.clear_frame(m,m.u32(c+0x1d0));
    for offset in [0,4,8,12] {
        let scene=m.u32(c+0x1d0);m.set_u32(scene+0x2130+offset,m.u32(c+0xbf8+offset));
    }
    let scene=m.u32(c+0x1d0);let player=m.u32(c+0x1d4);
    m.set_u32(scene+0x2104,m.u32(player+0x94));
    let player=m.u32(c+0x1d4);m.set_u32(player+0x94,m.u32(player+0x94).wrapping_add(1));
    m.set_u32(0x702e90,0);
    let player=m.u32(c+0x1d4);let table=m.u32(player+0x64);let index=m.u32(player+0x8c);
    h.copy_palette(m,c+0xc18,m.u32(table+index*4));
    let index=m.u32(m.u32(c+0x1d4)+0x8c);
    if m.u32(c+0x220)!=index {m.set_u32(c+0x220,index);m.set_u32(c+0xc14,1);}
    m.set_u32(0x702e8c,m.u32(c+0xbcc));
}
/// Timing part of `0x5de600`, with the original double-precision x87 mode.
/// Accumulation deliberately precedes invalid-step rejection.
pub fn begin_frame_time(m:&mut Mem,c:u32,milliseconds:f64,overhead_ms:f64) {
    let now=milliseconds*0.001;
    let elapsed=now-m.f64(c+0x250);m.set_f64(c+0xba0,elapsed);
    let fixed=m.i32(c+0xbd8);
    let mut dt=if fixed>0 {f64::from(fixed)*0.001}else{
        let dt=(elapsed-m.f64(c+0x258))-overhead_ms*0.001;
        if dt>0.3 && fixed==0 {0.05}else{dt}
    };
    m.set_f64(c+0xb98,m.f64(c+0xb98)+dt);
    if dt<0.0 || dt>100.0 {dt=0.0;}
    m.set_f64(c+0xb90,dt);m.set_f64(c+0x258,0.0);m.set_f64(c+0x250,now);
}
pub trait FrameEndHooks {
    fn apply_palette(&mut self,m:&mut Mem,controller:u32);
    fn refresh_surface(&mut self,m:&mut Mem,controller:u32);
    fn finish_frame(&mut self,m:&mut Mem,controller:u32);
    fn update_audio(&mut self,m:&mut Mem);
    /// Original optional view_key recording, bracketed by pause-clock updates.
    fn record_camera(&mut self,m:&mut Mem,controller:u32);
}
/// `0x5de7d0`, internal task 0: palette refresh, frame completion, audio and
/// optional camera recording. Recording itself remains an explicit subsystem.
pub fn finish_frame(m:&mut Mem,c:u32,audio:bool,record_camera:bool,h:&mut impl FrameEndHooks) {
    if m.u32(c+0xc14)!=0 {
        h.apply_palette(m,c);h.refresh_surface(m,c);m.set_u32(c+0xc14,0);
    }
    h.finish_frame(m,c);
    if audio {h.update_audio(m);}
    if record_camera {h.record_camera(m,c);}
}
pub trait SchedulerHooks {
    /// Execute and detach the request; callbacks may enqueue further work.
    fn execute(&mut self,m:&mut Mem,manager:u32,request:u32);
}
pub trait RequestHooks {
    fn run(&mut self,m:&mut Mem,task:u32);
    fn remove_task(&mut self,m:&mut Mem,manager:u32,task:u32);
    fn destroy(&mut self,m:&mut Mem,object:u32);
}
/// Core execution of `0x5d38c0`. Debug allocator validation/logging is not
/// reproduced here; callers own valid task/request records.
pub fn execute_request(m:&mut Mem,c:u32,r:u32,h:&mut impl RequestHooks) {
    let task=m.u32(r+12);if task==0 {return;}
    m.set_u32(task+24,m.u32(r+16));
    match m.u32(r+20) {
        0|2=>{
            let ordinary=m.u32(r+20)==0;
            if ordinary {m.set_u32(0x720118,task);}else{m.set_u32(task+16,0);}
            h.run(m,task);
            if ordinary {m.set_u32(0x720118,0);}
            // Original re-reads the request's task after callbacks.
            let task=m.u32(r+12);m.set_u32(task+16,m.u32(task+16).wrapping_add(1));
            if m.u32(c+12)==r {m.set_u32(c+12,m.u32(r+8));}
            else if m.u32(c+8)==r {m.set_u32(c+8,m.u32(r+8));}
            else {unlink(m,r);h.destroy(m,r);return;}
            let next=m.u32(r+8);if next!=0 {m.set_u32(next+4,0);}
            h.destroy(m,r);
        }
        _=>{h.remove_task(m,c,task);h.destroy(m,task);}
    }
}
/// `0x5d3850`: drain tick requests first, then real-time requests. Re-read each
/// head and the pointed-to clock after every callback; advance the tick last.
pub fn scheduler_tick(m:&mut Mem,manager:u32,h:&mut impl SchedulerHooks) {
    loop {
        let request=m.u32(manager+12);
        if request==0 || m.i32(request+16)>m.i32(manager) {break;}
        h.execute(m,manager,request);
    }
    loop {
        let request=m.u32(manager+8);
        if request==0 {break;}
        let clock=m.u32(manager+24);
        let now=if clock==0 {0}else{(m.f64(clock)*1000.0).trunc() as i32};
        if m.i32(request+16)>now {break;}
        h.execute(m,manager,request);
    }
    m.set_u32(manager,m.u32(manager).wrapping_add(1));
}
/// `0x5d3660`: queue a task request at a relative tick. Requests with the same
/// task, absolute tick and kind are discarded. Equal ticks use descending task
/// priority. The head is retained on equal priority; later equal entries are
/// inserted before the existing request (the original's asymmetric tie rule).
pub fn enqueue(m:&mut Mem,manager:u32,request:u32)->bool {
    if request==0 || m.u32(request+12)==0 {return false;}
    let time=m.u32(request+16).wrapping_add(m.u32(manager));m.set_u32(request+16,time);
    let task=m.u32(request+12);let kind=m.u32(request+20);
    let mut node=m.u32(manager+12);
    while node!=0 {
        if m.u32(node+12)==task && m.u32(node+16)==time && m.u32(node+20)==kind {return false;}
        node=m.u32(node+8);
    }
    let priority=m.i32(task+20);let mut previous=0;node=m.u32(manager+12);
    if node!=0 && (m.i32(node+16)<time as i32 ||
        (m.u32(node+16)==time && priority<=m.i32(m.u32(node+12)+20))) {
        previous=node;node=m.u32(node+8);
    }
    while node!=0 && (m.i32(node+16)<time as i32 ||
        (m.u32(node+16)==time && m.i32(m.u32(node+12)+20)>priority)) {
        previous=node;node=m.u32(node+8);
    }
    m.set_u32(request+4,previous);m.set_u32(request+8,node);
    if node!=0 {m.set_u32(node+4,request);}
    if previous==0 {m.set_u32(manager+12,request);}else{m.set_u32(previous+8,request);}
    true
}
/// `0x5de0a0`: creation order of the controller's internal tasks. Registration
/// and activation are separate from execution by the task scheduler.
pub fn internal_tasks(flags:u32,extra:bool)->Vec<i32> {
    let mut tasks=vec![400,0,20,30,10];
    if flags&0x100!=0 {tasks.extend([21,31,11]);}
    if extra {tasks.push(40);}
    tasks
}
pub trait TaskHooks {
    fn call(&mut self,m:&mut Mem,controller:u32,slot:u32);
    fn complete(&mut self,m:&mut Mem,task:u32);
}
/// `0x5de430`: dispatch and complete an internal task, including unknown IDs.
pub fn execute_task(m:&mut Mem,task:u32,h:&mut impl TaskHooks) {
    let c=m.u32(task+0x20);let id=m.i32(task+0x24);
    let slot=match id {
        0=>Some(0xb8),400=>Some(0xb4),40=>Some(0xac),
        10|11=>{m.set_u32(c+0x224,u32::from(id==11));Some(0x6c)},
        20|21=>{m.set_u32(c+0x224,u32::from(id==21));Some(0x64)},
        30|31=>{m.set_u32(c+0x224,u32::from(id==31));Some(0x68)},
        _=>None,
    };
    if let Some(slot)=slot {h.call(m,c,slot);}
    h.complete(m,task);
}
pub trait CameraResetHooks {
    fn prepare(&mut self,m:&mut Mem,transform:u32);
    fn update(&mut self,m:&mut Mem,transform:u32);
    fn derive_projection(&mut self,m:&mut Mem,camera:u32);
}
pub trait SceneResetHooks: CameraResetHooks {
    fn destroy_object(&mut self,m:&mut Mem,object:u32);
}
/// `0x5d21b0`: unlink without destroying. List roots share the node layout.
fn unlink(m:&mut Mem,p:u32) {
    let previous=m.u32(p+4);let next=m.u32(p+8);
    if previous!=0 {m.set_u32(previous+8,next);}
    if next!=0 {m.set_u32(next+4,previous);}
    m.set_u32(p+4,0);m.set_u32(p+8,0);
}
/// `0x5c4910`: clear ten typed object lists, then pending scene objects, then
/// restore the camera. Typed traversal caches next before calling a destructor;
/// pending traversal reloads the root each time, as in `0x5c33a0`.
pub fn reset_scene(m:&mut Mem,c:u32,h:&mut impl SceneResetHooks) {
    for k in 0..10 {
        let root=c+0x1f28+k*0x20;
        let mut object=m.u32(root+8);
        while object!=0 {
            let next=m.u32(object+8);unlink(m,object);h.destroy_object(m,object);object=next;
        }
    }
    let root=c+0x206c;
    while m.u32(root+8)!=0 {
        let object=m.u32(root+8);unlink(m,object);h.destroy_object(m,object);
    }
    reset_camera(m,c,h);
}
/// `0x5e02f0`: restore scenario camera and both interpolation endpoints.
pub fn reset_camera(m:&mut Mem,c:u32,h:&mut impl CameraResetHooks) {
    let camera=m.u32(c+0x1e8);
    if camera!=0 {
        h.prepare(m,c+0x28c);
        let camera=m.u32(c+0x1e8);
        m.set_f32(camera+0x58,m.f64(c+0xbe8) as f32);
        m.set_f32(camera+0x54,m.f64(c+0xbf0) as f32);
        h.update(m,c+0x28c);
        let camera=m.u32(c+0x1e8);
        let projection=m.u32(c+0x2bc);
        if projection==0 {h.derive_projection(m,camera);}
        else {m.set_u32(camera+0x4c,projection);}
    }
    for (source,dest) in [(0xb58,0x238),(0xb60,0x23c),(0xb68,0x240)] {
        let v=m.f64(c+source) as f32;
        m.set_f32(c+dest,v);m.set_f32(c+dest+12,v);
    }
}
pub trait BaseResetHooks {
    fn clear_resources(&mut self,m:&mut Mem,address:u32);
    fn milliseconds(&mut self) -> f64;
    fn stop_audio(&mut self,m:&mut Mem);
    fn clear_messages(&mut self,m:&mut Mem,address:u32);
    fn virtual_call(&mut self,m:&mut Mem,controller:u32,slot:u32);
}
/// `0x5e03c0`: base scenario reset. The caller supplies the installed audio flag.
pub fn reset_base(m:&mut Mem,c:u32,audio_enabled:bool,h:&mut impl BaseResetHooks) {
    h.clear_resources(m,c+0x2c0);
    let linked=m.u32(c+0x1e4);
    if linked!=0 {m.set_u32(linked+8,0);}
    m.set_u32(c+0xb98,0);m.set_u32(c+0xb9c,0);
    let time=h.milliseconds()*0.001;
    m.set_f64(c+0xb90,0.1);m.set_f64(c+0xba0,0.1);
    m.set_u32(c+0x258,0);m.set_f64(c+0x250,time);
    for o in [0xb48,0xb38,0xb40,0xb50,0xb88,0x25c,0xb4c,0xb3c,0xb44,0xb54,0xb8c,0xba8,0x1cc,0x1c8] {
        m.set_u32(c+o,0);
    }
    m.set_u32(c+0x220,u32::MAX);
    let playback=m.u32(c+0x1d4);m.set_u32(playback+0x8c,0);
    if audio_enabled {h.stop_audio(m);}
    m.set_u32(c+0xbac,0);
    h.clear_messages(m,c+0xbb0);
    h.virtual_call(m,c,0x7c);h.virtual_call(m,c,0x78);
    m.set_u32(c+0xbd0,0);
}
pub trait ResetHooks {
    fn destroy_command(&mut self, m: &mut Mem, command: u32);
    fn reset_base(&mut self, m: &mut Mem, controller: u32);
}
/// `0x5c4940`: pop queued commands in reverse order, then reset the base controller.
pub fn reset(m: &mut Mem, controller: u32, hooks: &mut impl ResetHooks) {
    while m.i32(controller + 0x1ef0) > 0 {
        let count = m.i32(controller + 0x1ef0) - 1;
        m.set_i32(controller + 0x1ef0, count);
        let command = m.u32(controller + 0xf50 + count as u32 * 4);
        if command != 0 { hooks.destroy_command(m,command); }
    }
    for (offset,value) in [(0x1f20,0),(0x1f08,1),(0x1f0c,1),(0x1f24,0)] {
        m.set_u32(controller+offset,value);
    }
    hooks.reset_base(m,controller);
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn camera_matrix_matches_original() {
        for line in include_str!("../../../tools/emu/golden_controller_camera_matrix.tsv").lines() {
            let p:Vec<_>=line.split('\t').collect();let mut m=Mem::new();let c=0x2a000000;let out=c+0x10000;
            for (o,v) in [0xb58,0xb60,0xb68,0xb70,0xb78,0xb80,0xb88].iter().zip(p[0].split(',')){m.set_f64(c+o,v.parse().unwrap());}
            camera_matrix(&mut m,c,out);
            let bits:Vec<u32>=p[1].split(',').map(|s|s.parse().unwrap()).collect();
            assert_eq!((0..12).map(|i|m.u32(out+i*4)).collect::<Vec<_>>(),bits);
        }
    }
    struct ScenePass(Vec<u32>);
    impl ScenePassHooks for ScenePass {
        fn overlay(&mut self,_:&mut Mem,_:u32){self.0.push(1);}
        fn profile(&mut self,b:bool){self.0.push(if b{2}else{8});}
        fn prepare_scene(&mut self,_:&mut Mem,_:u32,p:u32){self.0.extend([3,p]);}
        fn transform_scene(&mut self,_:&mut Mem,_:u32){self.0.push(4);}
        fn update_object(&mut self,_:&mut Mem,_:u32,mode:u32,_:&[u8;48],_:u32){self.0.extend([5,mode]);}
        fn render(&mut self,m:&mut Mem,c:u32){self.0.extend([6,m.u32(m.u32(c+0x1d0)+0x21dc)]);}
    }
    #[test]
    fn scene_pass_matches_original() {
        for line in include_str!("../../../tools/emu/golden_controller_scene_pass.tsv").lines() {
            let p:Vec<_>=line.split('\t').collect();let mut m=Mem::new();let c=0x2a000000;let scene=c+0x10000;let cam=c+0x20000;
            m.set_u32(c+0x1d0,scene);m.set_u32(c+0x1e8,if p[0]=="1"{cam}else{0});m.set_u32(cam+0x4c,if p[1]=="1"{c+0x30000}else{0});
            m.set_u32(c+0xb2c,if p[2]=="1"{c+0x50000}else{0});m.set_u32(0x702e8c,p[3].parse().unwrap());m.set_u32(c+0x224,9);
            let variant=p[8].parse::<u32>().unwrap() as f32;
            for (i,v) in [variant+1.,0.5f32,0.,0.,3.,0.25,0.,0.,4.,variant,2.,3.].iter().enumerate(){m.set_f32(c+0x50398+i as u32*4,*v);}
            for i in 0..4 {m.set_u32(c+0xb90+i*4,100+i);}
            let mut h=ScenePass(vec![]);scene_pass(&mut m,c,p[4]=="1",&mut h);
            let nums=|s:&str|s.split(',').filter(|s|!s.is_empty()).map(|s|s.parse().unwrap()).collect::<Vec<u32>>();
            assert_eq!(h.0,nums(p[5]));assert_eq!([0x2120,0x2124,0x2128,0x212c,0x21dc].map(|o|m.u32(scene+o)).to_vec(),nums(p[6]));
            assert_eq!((0..12).map(|i|m.u32(scene+0x2174+i*4)).collect::<Vec<_>>(),nums(p[7]));
        }
    }
    struct Effects(Vec<u32>);
    impl InternalEffects for Effects {
        fn call(&mut self,m:&mut Mem,c:u32,s:u32){self.0.extend([s,m.u32(c+0x224)]);}
    }
    #[test]
    fn recurring_runtime_matches_original() {
        let mut current=None;let mut runtime=InternalScheduler::new(0,false);let mut e=Effects(vec![]);
        for line in include_str!("../../../tools/emu/golden_internal_runtime.tsv").lines() {
            let p:Vec<_>=line.split('\t').collect();let mode=(p[0].parse::<u32>().unwrap(),p[1]=="1");
            if current!=Some(mode){runtime=InternalScheduler::new(mode.0,mode.1);current=Some(mode);}
            e.0.clear();runtime.tick(&mut e);
            assert_eq!(runtime.memory.u32(runtime.manager),p[2].parse::<u32>().unwrap()+1);
            assert_eq!(e.0,p[3].split(',').map(|s|s.parse().unwrap()).collect::<Vec<u32>>());
            assert!(runtime.free.len()<=1);
        }
    }
    struct Begin;
    impl FrameBeginHooks for Begin {
        fn clear_frame(&mut self,_:&mut Mem,_:u32){}
        fn copy_palette(&mut self,_:&mut Mem,_:u32,_:u32){}
    }
    #[test]
    fn frame_time_matches_original_bits() {
        for (i,line) in include_str!("../../../tools/emu/golden_controller_frame_time.tsv").lines().enumerate() {
            let p:Vec<_>=line.split('\t').collect();let mut m=Mem::new();let c=0x2a000000;
            let values:Vec<f64>=p[0].split(',').map(|s|s.parse().unwrap()).collect();
            m.set_f64(c+0x250,values[1]);m.set_f64(c+0x258,values[2]);m.set_f64(c+0xb98,values[4]);m.set_i32(c+0xbd8,p[1].parse().unwrap());
            let scene=c+0x20000;let player=c+0x30000;let table=c+0x40000;
            m.set_u32(c+0x1d0,scene);m.set_u32(c+0x1d4,player);m.set_u32(player+0x64,table);
            m.set_u32(player+0x8c,i as u32%2);m.set_u32(player+0x94,i as u32);m.set_u32(c+0xbcc,123);
            for k in 0..4 {m.set_u32(c+0xbf8+k*4,100+k);}
            begin_frame(&mut m,c,values[0],values[3],&mut Begin);
            let bits:Vec<u64>=p[2].split(',').map(|s|s.parse().unwrap()).collect();
            assert_eq!([0x250,0x258,0xb90,0xb98,0xba0].map(|o|m.f64(c+o).to_bits()).to_vec(),bits);
            let expected:Vec<u32>=p[3].split(',').map(|s|s.parse().unwrap()).collect();
            assert_eq!([scene+0x2130,scene+0x2134,scene+0x2138,scene+0x213c,scene+0x2104,player+0x94,c+0x220,c+0xc14,0x702e8c,0x702e90].map(|a|m.u32(a)).to_vec(),expected);
        }
    }
    struct FrameEnd(Vec<u32>);
    impl FrameEndHooks for FrameEnd {
        fn apply_palette(&mut self,_:&mut Mem,_:u32){self.0.push(1);}
        fn refresh_surface(&mut self,_:&mut Mem,_:u32){self.0.push(2);}
        fn finish_frame(&mut self,_:&mut Mem,_:u32){self.0.push(3);}
        fn update_audio(&mut self,_:&mut Mem){self.0.push(4);}
        fn record_camera(&mut self,_:&mut Mem,_:u32){self.0.push(5);}
    }
    #[test]
    fn frame_end_order_matches_original() {
        for line in include_str!("../../../tools/emu/golden_controller_frame_end.tsv").lines() {
            let p:Vec<_>=line.split('\t').collect();let mut m=Mem::new();let c=0x2a000000;
            m.set_u32(c+0xc14,p[0].parse().unwrap());let mut h=FrameEnd(vec![]);
            finish_frame(&mut m,c,p[1]=="1",false,&mut h);
            assert_eq!(m.u32(c+0xc14),0);
            assert_eq!(h.0,p[2].split(',').map(|s|s.parse().unwrap()).collect::<Vec<u32>>());
        }
    }
    struct Requests(Vec<u32>);
    impl RequestHooks for Requests {
        fn run(&mut self,m:&mut Mem,t:u32){self.0.extend([t,m.u32(t+16),m.u32(0x720118)]);}
        fn remove_task(&mut self,_:&mut Mem,_:u32,t:u32){self.0.extend([2,t]);}
        fn destroy(&mut self,_:&mut Mem,t:u32){self.0.extend([3,t]);}
    }
    #[test]
    fn request_execution_matches_original() {
        for line in include_str!("../../../tools/emu/golden_scheduler_request.tsv").lines() {
            let p:Vec<_>=line.split('\t').collect();let c=0x2a000000;let r=c+0x1000;let t=c+0x2000;let mut m=Mem::new();
            m.set_u32(r+12,t);m.set_u32(r+16,123);m.set_u32(r+20,p[0].parse().unwrap());m.set_u32(t+16,p[1].parse().unwrap());m.set_u32(c+12,r);
            let mut h=Requests(vec![]);execute_request(&mut m,c,r,&mut h);
            let nums=|s:&str|s.split(',').filter(|s|!s.is_empty()).map(|s|s.parse().unwrap()).collect::<Vec<u32>>();
            assert_eq!(h.0,nums(p[2]));
            assert_eq!(vec![m.u32(t+16),m.u32(t+24),m.u32(c+12),m.u32(0x720118)],nums(p[3]));
        }
    }
    struct TickHooks(Vec<u32>);
    impl SchedulerHooks for TickHooks {
        fn execute(&mut self,m:&mut Mem,c:u32,p:u32){
            self.0.push(p);let root=if m.u32(c+12)==p {c+12}else{c+8};
            let next=m.u32(p+8);m.set_u32(root,next);if next!=0 {m.set_u32(next+4,0);}
        }
    }
    #[test]
    fn scheduler_tick_matches_original() {
        for line in include_str!("../../../tools/emu/golden_scheduler_tick.tsv").lines() {
            let p:Vec<_>=line.split('\t').collect();let c=0x2a000000;let mut m=Mem::new();
            m.set_u32(c,p[0].parse().unwrap());
            if p[1]!="none" {m.set_u32(c+24,c+0x100);m.set_f64(c+0x100,p[1].parse().unwrap());}
            for (k,s) in p[2..4].iter().enumerate() {
                let times:Vec<i32>=s.split(',').filter(|s|!s.is_empty()).map(|s|s.parse().unwrap()).collect();
                let base=c+0x1000+k as u32*0x1000;
                if !times.is_empty() {m.set_u32(c+if k==0{12}else{8},base);}
                for (i,t) in times.iter().enumerate(){let a=base+i as u32*32;m.set_i32(a+16,*t);m.set_u32(a+8,if i+1<times.len(){a+32}else{0});}
            }
            let mut h=TickHooks(vec![]);scheduler_tick(&mut m,c,&mut h);
            let nums=|s:&str|s.split(',').filter(|s|!s.is_empty()).map(|s|s.parse().unwrap()).collect::<Vec<u32>>();
            assert_eq!(h.0,nums(p[4]));assert_eq!(vec![m.u32(c),m.u32(c+12),m.u32(c+8)],nums(p[5]));
        }
    }
    #[test]
    fn scheduler_enqueue_matches_original() {
        for line in include_str!("../../../tools/emu/golden_scheduler_enqueue.tsv").lines() {
            let p:Vec<_>=line.split('\t').collect();let c=0x2a000000;let mut m=Mem::new();m.set_u32(c,p[0].parse().unwrap());
            let mut rejected=Vec::new();
            for (i,s) in p[1].split(';').enumerate() {
                let v:Vec<u32>=s.split(',').map(|s|s.parse().unwrap()).collect();let task=c+0x1000+v[0]*0x100;let req=c+0x10000+i as u32*0x20;
                m.set_u32(task+20,v[1]);m.set_u32(req+12,task);m.set_u32(req+16,v[2]);m.set_u32(req+20,v[3]);
                if !enqueue(&mut m,c,req){rejected.push(i as u32);}
            }
            let mut got=Vec::new();let mut node=m.u32(c+12);let mut previous=0;
            while node!=0 {assert_eq!(m.u32(node+4),previous);got.push((node-c-0x10000)/0x20);previous=node;node=m.u32(node+8);}
            let nums=|s:&str|s.split(',').filter(|s|!s.is_empty()).map(|s|s.parse().unwrap()).collect::<Vec<u32>>();
            assert_eq!(got,nums(p[2]));assert_eq!(rejected,nums(p[3]));
        }
    }
    struct Tasks(Vec<u32>);
    impl TaskHooks for Tasks {
        fn call(&mut self,_:&mut Mem,_:u32,s:u32){self.0.push(s);}
        fn complete(&mut self,_:&mut Mem,_:u32){self.0.push(1);}
    }
    #[test]
    fn internal_dispatch_matches_original() {
        assert_eq!(internal_tasks(0,false),[400,0,20,30,10]);
        assert_eq!(internal_tasks(0,true),[400,0,20,30,10,40]);
        assert_eq!(internal_tasks(0x100,false),[400,0,20,30,10,21,31,11]);
        assert_eq!(internal_tasks(0x100,true),[400,0,20,30,10,21,31,11,40]);
        for line in include_str!("../../../tools/emu/golden_controller_tasks.tsv").lines() {
            let p:Vec<_>=line.split('\t').collect();let mut m=Mem::new();let task=0x10000000;let c=task+0x1000;
            m.set_u32(task+0x20,c);m.set_i32(task+0x24,p[0].parse().unwrap());m.set_u32(c+0x224,p[1].parse().unwrap());
            let mut h=Tasks(vec![]);execute_task(&mut m,task,&mut h);
            assert_eq!(m.u32(c+0x224),p[2].parse::<u32>().unwrap());
            assert_eq!(h.0,p[3].split(',').map(|s|s.parse().unwrap()).collect::<Vec<u32>>());
        }
    }
    impl SceneResetHooks for CameraHooks {
        fn destroy_object(&mut self,m:&mut Mem,p:u32){assert_eq!(m.u32(p+4),0);assert_eq!(m.u32(p+8),0);self.0.push(p);}
    }
    #[test]
    fn scene_lists_match_original() {
        for line in include_str!("../../../tools/emu/golden_controller_scene.tsv").lines() {
            let p:Vec<_>=line.split('\t').collect();let c=0x2a000000;let mut m=Mem::new();
            let counts:Vec<u32>=p[0].split(',').map(|s|s.parse().unwrap()).collect();
            let mut at=c+0x4000;
            for (k,&count) in counts.iter().enumerate() {
                let root=if k<10 {c+0x1f28+k as u32*0x20}else{c+0x206c};
                let mut previous=root;
                for _ in 0..count {m.set_u32(previous+8,at);m.set_u32(at+4,previous);previous=at;at+=0x20;}
            }
            let mut h=CameraHooks(vec![]);reset_scene(&mut m,c,&mut h);
            let expected:Vec<u32>=p[1].split(',').filter(|s|!s.is_empty()).map(|s|s.parse().unwrap()).collect();
            assert_eq!(h.0,expected);
            for k in 0..10 {assert_eq!(m.u32(c+0x1f30+k*0x20),0);}
            assert_eq!(m.u32(c+0x2074),0);
        }
    }
    struct CameraHooks(Vec<u32>);
    impl CameraResetHooks for CameraHooks {
        fn prepare(&mut self,_:&mut Mem,a:u32){self.0.push(a);}
        fn update(&mut self,_:&mut Mem,a:u32){self.0.push(a+1);}
        fn derive_projection(&mut self,_:&mut Mem,a:u32){self.0.push(a);}
    }
    #[test]
    fn camera_fields_match_original() {
        for line in include_str!("../../../tools/emu/golden_controller_camera.tsv").lines() {
            let p:Vec<_>=line.split('\t').collect();let c=0x2a000000;let mut m=Mem::new();
            m.set_u32(c+0x1e8,if p[0]=="1"{c+0x2000}else{0});m.set_u32(c+0x2bc,p[1].parse().unwrap());
            for (o,v) in [0xbe8,0xbf0,0xb58,0xb60,0xb68].iter().zip(p[2].split(',')) {m.set_f64(c+o,v.parse().unwrap());}
            let mut h=CameraHooks(vec![]);reset_camera(&mut m,c,&mut h);
            let fields=[0x2058,0x2054,0x204c,0x238,0x23c,0x240,0x244,0x248,0x24c].map(|o|m.u32(c+o)).to_vec();
            let nums=|s:&str|s.split(',').filter(|s|!s.is_empty()).map(|s|s.parse::<u32>().unwrap()).collect::<Vec<_>>();
            assert_eq!(fields,nums(p[3]));assert_eq!(h.0,nums(p[4]));
        }
    }
    struct BaseHooks { time:f64,events:Vec<u32> }
    impl BaseResetHooks for BaseHooks {
        fn clear_resources(&mut self,_:&mut Mem,a:u32){self.events.push(a);}
        fn milliseconds(&mut self)->f64{self.time}
        fn stop_audio(&mut self,_:&mut Mem){self.events.push(1);}
        fn clear_messages(&mut self,_:&mut Mem,a:u32){self.events.push(a);}
        fn virtual_call(&mut self,_:&mut Mem,_:u32,s:u32){self.events.push(s);}
    }
    #[test]
    fn base_fields_match_original() {
        for line in include_str!("../../../tools/emu/golden_controller_base.tsv").lines() {
            let p:Vec<_>=line.split('\t').collect();let c=0x2a000000;
            let mut m=Mem::new();m.write(c,&vec![0x55;0xc00]);
            m.set_u32(c+0x1e4,if p[1]=="1" {c+0x2000}else{0});m.set_u32(c+0x1d4,c+0x3000);
            m.set_u32(c+0x2008,0x12345678);m.set_u32(c+0x308c,0x87654321);
            let mut h=BaseHooks{time:p[0].parse().unwrap(),events:vec![]};
            reset_base(&mut m,c,p[2]=="1",&mut h);
            let offsets=[0x250,0x254,0x258,0x25c,0xb90,0xb94,0xba0,0xba4,0xb98,0xb9c,0x220,0xb48,0xb38,0xb40,0xb50,0xb88,0xb4c,0xb3c,0xb44,0xb54,0xb8c,0xba8,0x1cc,0x1c8,0xbac,0xbd0,0x2008,0x308c];
            let expected:Vec<u32>=p[3].split(',').map(|s|s.parse().unwrap()).collect();
            assert_eq!(offsets.map(|o|m.u32(c+o)).to_vec(),expected);
            assert_eq!(h.events,p[4].split(',').map(|s|s.parse().unwrap()).collect::<Vec<u32>>());
        }
    }
    struct Hooks(Vec<u32>);
    impl ResetHooks for Hooks {
        fn destroy_command(&mut self,_:&mut Mem,p:u32){self.0.push(p);}
        fn reset_base(&mut self,_:&mut Mem,_:u32){self.0.push(0xffffffff);}
    }
    #[test]
    fn original_reset_order_and_fields() {
        for line in include_str!("../../../tools/emu/golden_controller_reset.tsv").lines() {
            let p:Vec<_>=line.split('\t').collect();
            let nums=|s:&str|s.split(',').filter(|s|!s.is_empty()).map(|s|s.parse::<u32>().unwrap()).collect::<Vec<_>>();
            let commands=nums(p[0]);let mut m=Mem::new();let c=0x10000000;
            m.set_u32(c+0x1ef0,commands.len() as u32);
            for (i,p) in commands.iter().enumerate(){m.set_u32(c+0xf50+i as u32*4,*p);}
            let mut h=Hooks(vec![]);reset(&mut m,c,&mut h);
            assert_eq!(h.0,nums(p[1]));
            assert_eq!([0x1ef0,0x1f20,0x1f08,0x1f0c,0x1f24].map(|o|m.u32(c+o)).to_vec(),nums(p[2]));
        }
    }
}

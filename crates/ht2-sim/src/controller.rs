//! Original scenario-controller reset. Resource destruction and the base reset
//! remain explicit callbacks, so a caller cannot silently skip their side effects.
use crate::mem::Mem;
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

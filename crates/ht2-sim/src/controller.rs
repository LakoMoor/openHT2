//! Original scenario-controller reset. Resource destruction and the base reset
//! remain explicit callbacks, so a caller cannot silently skip their side effects.
use crate::mem::Mem;
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

//! Original scenario-controller reset. Resource destruction and the base reset
//! remain explicit callbacks, so a caller cannot silently skip their side effects.
use crate::mem::Mem;
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

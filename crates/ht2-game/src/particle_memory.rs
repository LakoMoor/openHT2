//! Application CRT boundary for particle names and geometry arrays.
//! Reuses the owned heap; addresses need not match the original CRT.
use ht2_sim::{mem::Mem,ai_scene::CommandHeap};
#[derive(Default)]
pub struct ParticleMemory {
    heap:crate::event_heap::EventHeap,
    live:std::collections::HashSet<u32>,
}
impl ParticleMemory {
    pub fn allocate_zeroed(&mut self,m:&mut Mem,bytes:u32)->u32 {
        let p=self.heap.allocate(m,bytes);assert!(self.live.insert(p));m.write(p,&vec![0;bytes as usize]);p
    }
    pub fn release(&mut self,m:&mut Mem,p:u32) {
        if p==0 {return;}
        assert!(self.live.remove(&p),"particle free without ownership");self.heap.release(m,p);
    }
    pub fn live_blocks(&self)->usize {self.live.len()}
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn real_particle_lifecycle_releases_and_reuses_application_memory() {
        let mut m=Mem::new();let mut heap=ParticleMemory::default();let manager=m.alloc(0x1b4);let sort=m.alloc(0x40);let slots=m.alloc(4);let pool=m.alloc(16);let obj=m.alloc(0xb0);let clock=m.alloc(0xc00);let point=m.alloc(24);let scratch=m.alloc(32);
        m.set_u32(manager+0x1b0,1);m.set_u32(manager+0x20,pool);m.set_u32(sort+0x30,slots);m.set_u32(sort+0x34,1);m.set_u32(pool+4,obj);m.set_u32(obj+16,pool);
        m.set_u32(0x6f6a8c,pool);m.set_u32(0x6f6a68,sort);m.set_u32(0x696224,clock);m.set_u32(0x696208,1);m.set_u32(0x695bf0,0xffffffff);m.write(clock+0xb90,&0.1f64.to_le_bytes());
        let mut previous_name=None;
        for _ in 0..3 {
            assert_eq!(ht2_sim::instruments::emit_complete_scene_particle(&mut m,point,point+12,scratch,|m,size|heap.allocate_zeroed(m,size)).unwrap(),Some(obj));
            assert_eq!(heap.live_blocks(),6);let name=m.u32(obj+0x20);if let Some(old)=previous_name {assert_eq!(name,old);}previous_name=Some(name);
            for _ in 0..12 {ht2_sim::instruments::tick_scene_particle_pools(&mut m,manager,scratch,|m,p|heap.release(m,p));}
            assert_eq!(heap.live_blocks(),0);assert_eq!(m.u32(pool+8),0);assert_eq!(m.u32(pool+4),obj);assert_eq!(m.u32(slots),0);
        }
    }
}

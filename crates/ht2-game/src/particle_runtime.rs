//! Owned application runtime for the verified scene-particle class.
//! Fixed capacity and materials are supplied by the caller, not game defaults.
use ht2_sim::{mem::Mem,instruments as sim};
pub struct ParticleRuntime {
    memory:crate::particle_memory::ParticleMemory,
    manager:u32,sort:u32,pool:u32,clock:u32,materials:u32,frames:u32,scratch:u32,input:u32,
}
impl ParticleRuntime {
    pub fn new(m:&mut Mem,capacity:u32,materials:&[u32])->Self {
        assert!(!materials.is_empty());
        assert!(materials.len()<=25,"materials exceeds original smoke table capacity");
        let manager=m.alloc(0x1b4);let sort=m.alloc(0x40);let slots=m.alloc(4);let pool=m.alloc(16);let clock=m.alloc(0xc00);let material_table=m.alloc(materials.len() as u32*4);let scratch=m.alloc(32);let input=m.alloc(24);
        m.set_u32(manager+0x1b0,1);m.set_u32(manager+0x20,pool);m.set_u32(sort+0x30,slots);m.set_u32(sort+0x34,1);
        let mut previous=0;
        for _ in 0..capacity {let obj=m.alloc(0xb0);m.set_u32(obj+16,pool);m.set_u32(obj+8,previous);if previous!=0 {m.set_u32(previous+12,obj);}previous=obj;}
        m.set_u32(pool+4,previous);for (i,c) in materials.iter().enumerate(){m.set_u32(material_table+i as u32*4,*c);}
        Self{memory:Default::default(),manager,sort,pool,clock,materials:material_table,frames:materials.len() as u32,scratch,input}
    }
    /// Connect the original smoke enumeration to application material lookup.
    /// Resource parsing supplies pointers to the original material record layout.
    pub fn from_material_lookup(m:&mut Mem,capacity:u32,mut lookup:impl FnMut(&mut Mem,&str)->u32)->Result<Self,&'static str> {
        let mut found=0;
        let count=sim::load_smoke_materials(m,|m,name|{
            if found==25 {return 0;}
            let material=lookup(m,name);if material!=0 {found+=1;}material
        })?;
        let materials:Vec<u32>=(0..count).map(|i|m.u32(0x695bf0+i*4)).collect();
        Ok(Self::new(m,capacity,&materials))
    }
    pub fn emit(&mut self,m:&mut Mem,point:[f32;3],direction:[f32;3])->Result<bool,&'static str> {
        m.set_u32(0x6f6a8c,self.pool);m.set_u32(0x6f6a68,self.sort);m.set_u32(0x696224,self.clock);m.set_u32(0x696208,self.frames);
        // Original emission uses this global materials address, so publish the caller's materials.
        for i in 0..self.frames {m.set_u32(0x695bf0+i*4,m.u32(self.materials+i*4));}
        for (i,x) in point.into_iter().chain(direction).enumerate(){m.set_f32(self.input+i as u32*4,x);}
        Ok(sim::emit_complete_scene_particle(m,self.input,self.input+12,self.scratch,|m,n|self.memory.allocate_zeroed(m,n))?.is_some())
    }
    pub fn tick(&mut self,m:&mut Mem,delta:f64) {
        m.write(self.clock+0xb90,&delta.to_le_bytes());sim::tick_scene_particle_pools(m,self.manager,self.scratch,|m,p|self.memory.release(m,p));
    }
    pub fn clear(&mut self,m:&mut Mem) {
        let mut obj=m.u32(self.pool+8);while obj!=0 {sim::mark_particle_for_removal(m,obj);obj=m.u32(obj+8);}
        sim::cleanup_scene_particle_pools(m,self.manager,|m,p|self.memory.release(m,p));
    }
    pub fn active_count(&self,m:&Mem)->usize {let mut n=0;let mut obj=m.u32(self.pool+8);while obj!=0 {n+=1;obj=m.u32(obj+8);}n}
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn capacity_expiry_clear_and_reuse(){
        let mut m=Mem::new();let mut runtime=ParticleRuntime::new(&mut m,2,&[0xffffffff]);
        for _ in 0..3 {
            assert!(runtime.emit(&mut m,[1.,2.,3.],[0.,0.,1.]).unwrap());assert!(runtime.emit(&mut m,[4.,5.,6.],[1.,0.,0.]).unwrap());assert!(!runtime.emit(&mut m,[0.;3],[0.;3]).unwrap());assert_eq!(runtime.active_count(&m),2);
            for _ in 0..12 {runtime.tick(&mut m,0.1);}assert_eq!(runtime.active_count(&m),0);assert_eq!(runtime.memory.live_blocks(),0);
            assert!(runtime.emit(&mut m,[0.;3],[0.;3]).unwrap());runtime.clear(&mut m);assert_eq!(runtime.active_count(&m),0);assert_eq!(runtime.memory.live_blocks(),0);
        }
    }
    #[test]
    fn material_lookup_drives_particle_animation() {
        let mut m=Mem::new();let first=m.alloc(0x60);let second=m.alloc(0x60);let mut names=vec![];
        let mut runtime=ParticleRuntime::from_material_lookup(&mut m,1,|_,name|{names.push(name.to_owned());match name {"msmoke00"=>first,"msmoke01"=>second,_=>0}}).unwrap();
        assert_eq!(names,["msmoke00","msmoke01","msmoke02"]);
        assert_eq!(m.f32(first+0x58),1.0);assert_eq!(m.u32(second+0x20)&0x40220,0x40220);
        runtime.emit(&mut m,[0.;3],[1.,0.,0.]).unwrap();let object=m.u32(runtime.pool+8);let geometry=m.u32(object+0x48);
        runtime.tick(&mut m,0.4);assert_eq!(m.u32(geometry+0x24),first);
        runtime.tick(&mut m,0.4);assert_eq!(m.u32(geometry+0x24),second);
        runtime.clear(&mut m);assert_eq!(runtime.memory.live_blocks(),0);
    }

}

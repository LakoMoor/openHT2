//! Mixed-class effect manager: the six particle classes of `aeffects.cpp` in one `Mem`.
//! Pool capacities and the effect option scalars (`0x69603c`, `0x696048`, `0x696054`) and the sky
//! object behind the dust gate are STAND-INS: their owners are not ported.
use ht2_sim::{globals,instruments as sim,mem::Mem};
use std::collections::HashMap;

const SLOT:u32=0xe0;
/// Pool address globals of the original, in manager order.
const POOLS:[u32;6]=[0x6f6a7c,0x6f6a80,0x6f6a88,0x6f6a8c,0x6f6a98,0x6f6a9c];

/// One particle as a camera-facing quad.
#[derive(Debug,Clone,PartialEq)]
pub struct Quad {pub position:[f32;3],pub width:f32,pub height:f32,pub material:String}

pub struct ParticleWorld {
    memory:crate::particle_memory::ParticleMemory,
    manager:u32,pools:[u32;6],clock:u32,scratch:u32,input:u32,
    names:HashMap<u32,String>,
}
impl ParticleWorld {
    /// `available` says which material names exist in the resource archive.
    pub fn new(m:&mut Mem,capacity:u32,available:impl Fn(&str)->bool)->Result<Self,&'static str> {
        globals::install_constants(m);
        let manager=m.alloc(0x1b4);let sort=m.alloc(0x40);let slots=m.alloc(4);let clock=m.alloc(0xc00);let scratch=m.alloc(0x200);let input=m.alloc(48);
        m.set_u32(sort+0x30,slots);m.set_u32(sort+0x34,1);m.set_u32(0x6f6a68,sort);m.set_u32(0x696224,clock);m.set_u32(manager+0x1b0,6);
        let mut pools=[0;6];
        for (i,address) in POOLS.into_iter().enumerate() {
            let pool=m.alloc(16);let mut previous=0;
            for _ in 0..capacity {let obj=m.alloc(SLOT);m.set_u32(obj+16,pool);m.set_u32(obj+8,previous);if previous!=0 {m.set_u32(previous+12,obj);}previous=obj;}
            m.set_u32(pool+4,previous);m.set_u32(address,pool);m.set_u32(manager+0x20+i as u32*4,pool);pools[i]=pool;
        }
        let mut names=HashMap::new();
        for which in [sim::EffectMaterials::Smoke,sim::EffectMaterials::Fume,sim::EffectMaterials::Soot,sim::EffectMaterials::Dust] {
            sim::load_effect_materials(m,which,|m,name|{if !available(name) {return 0;}let p=m.alloc(0x60);names.insert(p,name.to_owned());p})?;
        }
        // STAND-INS: effect option scalars and a dim sky so the dust gate of 0x4bb010 passes.
        m.set_f32(0x69603c,1.0);m.set_f32(0x696048,0.0);m.set_f32(0x696054,1.0);m.set_u32(0x671700,200);
        let sky=m.alloc(0x240);m.set_f32(sky+0x220,0.5);m.set_f64(sky+0x78,0.0);m.set_f64(sky+0x68,0.0);m.set_u32(0x6f33ac,sky);
        Ok(Self{memory:Default::default(),manager,pools,clock,scratch,input,names})
    }
    fn vec(&mut self,m:&mut Mem,slot:u32,v:[f32;3])->u32 {
        let a=self.input+slot*12;for (i,x) in v.into_iter().enumerate() {m.set_f32(a+i as u32*4,x);}a
    }
    fn construct_root(&mut self,m:&mut Mem,object:u32,args:[u32;11]) {
        let scratch=self.scratch+0x100;let memory=&mut self.memory;
        let mut owner=sim::AllocatedParticleOwner{allocate:|m:&mut Mem,n|memory.allocate_zeroed(m,n),error:None};
        sim::construct_particle_root(m,object,args,scratch,&mut owner).expect("particle allocation");
    }
    /// `0x4bb050`: wheel dust. Returns whether a particle was made.
    pub fn emit_dust(&mut self,m:&mut Mem,position:[f32;3],direction:[f32;3],intensity:f32)->bool {
        let (p,d)=(self.vec(m,0,position),self.vec(m,1,direction));let mut made=None;
        sim::emit_wheel_dust(m,p,d,intensity,|m|globals::rand(m),|m|sim::dust_gate(m),|m,_,pool|sim::allocate_effect_object(m,pool),
            |_,r|made=Some(r));
        let Some(r)=made else {return false};
        let args=sim::DustParticleArgs{clock:m.u32(0x696224),position:p,velocity:d,size_a:r.size_a,size_b:r.size_b,table:r.material_table,frames:m.u32(0x696218),kind:0,lifetime:f32::from_bits(r.tail)};
        let scratch=self.scratch;
        sim::construct_dust_particle(m,r.object,args,scratch,|m|globals::rand(m),|m,o,a|self.construct_root(m,o,a));
        true
    }
    /// `0x4ba660` (exhaust smoke) for intensity `p`.
    pub fn emit_exhaust(&mut self,m:&mut Mem,position:[f32;3],direction:[f32;3],p:f32) {
        let (pp,d)=(self.vec(m,0,position),self.vec(m,1,direction));let dir_slot=self.input+24;let scratch=self.scratch;
        // The allocation handshake (0x6f4bc8..) is consumed by the root constructor, so each
        // particle must be constructed before the next allocation, exactly as in the original.
        sim::emit_exhaust(m,pp,d,p,|m|globals::rand(m),|m,_,pool|sim::allocate_effect_object(m,pool),|m,r|{
            for (i,x) in r.direction.into_iter().enumerate() {m.set_f32(dir_slot+i as u32*4,x);}
            let lifetime=f32::from_bits(r.tail[0]);
            match r.kind {
                sim::ExhaustKind::Smoke=>sim::construct_smoke_particle(m,r.object,r.clock,pp,dir_slot,r.table,r.extra,lifetime,|m|globals::rand(m),|m,o,a|self.construct_root(m,o,a)),
                sim::ExhaustKind::Fume=>sim::construct_fume_particle(m,r.object,r.clock,pp,dir_slot,r.table,r.extra,lifetime,|m|globals::rand(m),|m,o,a|self.construct_root(m,o,a)),
                sim::ExhaustKind::Soot=>sim::construct_soot_particle(m,r.object,r.clock,pp,dir_slot,r.table,r.extra,lifetime,f32::from_bits(r.tail[1]),scratch,|m|globals::rand(m),|m,o,a|self.construct_root(m,o,a)),
            }
        });
    }
    /// `0x4baf90`: flash of the given size.
    pub fn emit_flash(&mut self,m:&mut Mem,position:[f32;3],size:f32) {
        let p=self.vec(m,0,position);let mut made=None;
        sim::emit_flash(m,p,size,|m,_,pool|sim::allocate_effect_object(m,pool),|_,r|made=Some(r));
        if let Some(r)=made {sim::construct_flash_particle(m,r.object,r.clock,p,r.size,r.table,r.frames,|m,o,a|self.construct_root(m,o,a));}
    }
    /// Advance every class by `dt`, then free the particles marked for removal.
    pub fn tick(&mut self,m:&mut Mem,dt:f64) {
        m.write(self.clock+0xb90,&dt.to_le_bytes());
        let scratch=self.scratch;
        sim::update_effect_pools(m,self.manager,|m,obj|{
            let remove=|m:&mut Mem,o:u32|sim::mark_particle_for_removal(m,o);
            let life=|m:&mut Mem,o:u32|sim::update_particle_lifecycle(m,o,remove);
            let size=|m:&mut Mem,o:u32|sim::update_particle_size(m,o,life);
            match m.u32(obj) {
                0x650dd4=>sim::tick_scene_particle(m,obj,scratch),
                0x650f34=>sim::update_dust_particle(m,obj,scratch,life),
                0x650c78=>sim::update_smoke_particle(m,obj,scratch,size),
                0x650d80=>sim::update_fume_particle(m,obj,scratch,size),
                0x650cd8=>sim::update_soot_particle(m,obj,scratch,size),
                0x650ed8=>sim::update_flash_particle(m,obj,|m|globals::rand(m),life),
                other=>panic!("unknown particle class {other:#x}"),
            }
        });
        let memory=&mut self.memory;
        sim::cleanup_effect_pools(m,self.manager,|m,obj|{
            if m.u32(obj)==0x650f34 {sim::delete_dust_particle(m,obj,|m,p|memory.release(m,p));}
            else{sim::delete_scene_particle(m,obj,|m,p|memory.release(m,p));}
        });
    }
    pub fn live_count(&self,m:&Mem)->usize {
        self.pools.iter().map(|&pool|{let mut n=0;let mut o=m.u32(pool+8);while o!=0 {n+=1;o=m.u32(o+8);}n}).sum()
    }
    pub fn live_blocks(&self)->usize {self.memory.live_blocks()}
    /// Every live particle as a quad (STAND-IN billboard: size from the current vertex arrays).
    pub fn quads(&self,m:&Mem)->Vec<Quad> {
        let mut out=Vec::new();
        for &pool in &self.pools {
            let mut o=m.u32(pool+8);
            while o!=0 {
                let g=m.u32(o+0x48);let (x,z)=(m.u32(g+0x10),m.u32(g+0x14));
                if let Some(name)=self.names.get(&m.u32(g+0x24)) {
                    out.push(Quad{position:[m.f32(o+0x28),m.f32(o+0x2c),m.f32(o+0x30)],width:m.f32(x+4)-m.f32(x),height:m.f32(z+8)-m.f32(z),material:name.clone()});
                }
                o=m.u32(o+8);
            }
        }
        out
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn world(m:&mut Mem)->ParticleWorld {ParticleWorld::new(m,8,|n|n.starts_with("msmoke0")||n.starts_with("mvapor0")||n.starts_with("burn0")||n.starts_with("dust0")).unwrap()}
    #[test]
    fn every_class_is_created_updated_and_freed() {
        let mut m=Mem::new();let mut w=world(&mut m);
        m.set_u32(globals::RAND_SEED,12345);
        for i in 0..60 {
            w.emit_exhaust(&mut m,[i as f32,0.,0.],[0.,1.,0.2],1.0);
            w.emit_flash(&mut m,[0.,i as f32,0.],1.0);
            w.emit_dust(&mut m,[0.,0.,i as f32],[0.,0.3,1.],1.0);
            for pool in w.pools {let mut o=m.u32(pool+8);while o!=0 {assert_ne!(m.u32(o),0x650334,"step {i}: live particle {o:#x} was never constructed");o=m.u32(o+8);}}
            w.tick(&mut m,0.05);
            for q in w.quads(&m) {assert!(q.position.iter().all(|x|x.is_finite()),"{q:?}");}
        }
        assert!(w.live_count(&m)>0);
        for _ in 0..200 {w.tick(&mut m,0.1);}
        assert_eq!(w.live_count(&m),0);assert_eq!(w.live_blocks(),0);
        assert_eq!(m.u32(0x6f6a6c),0,"live dust counter must return to zero");
    }
}

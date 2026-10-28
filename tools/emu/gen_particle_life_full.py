"""Full root particle ctor, real bounds, controlled allocation/identities/manager."""
import pathlib,random,struct
from emu import Emu,OBJ_BASE
r=random.Random(0x59a780);e=Emu();rows=[];freed=[];heap=[0];manager=OBJ_BASE+0x10000;vt=manager+0x1000
e.set_u32(manager,vt);e.set_u32(vt+0x14,0x593270);e.set_u32(manager+0x30,manager+0x2000);e.set_u32(manager+0x34,1)

def alloc(em):
 p=heap[0];heap[0]+=0x100;e.write(p,bytes(0x100));return p
e.hook_ret(0x632643,alloc)
e.hook_ret(0x63266c,lambda em:freed.append(em.arg(0)) or 0)
for case in range(40):
 e.set_u32(manager+0x2000,0);heap[0]=OBJ_BASE+0x20000;e.write(OBJ_BASE,bytes(0xb0));color=r.getrandbits(32);e.set_u32(OBJ_BASE+0x1000,color)
 e.set_u32(0x6f6a68,manager);raw=struct.pack('<6f',*[r.uniform(-10,10) for _ in range(6)]);e.write(OBJ_BASE+0x1100,raw)
 args=[0,OBJ_BASE+0x1100,OBJ_BASE+0x110c,struct.unpack('<I',struct.pack('<f',r.uniform(0.1,5)))[0],OBJ_BASE+0x1000,1]
 pool=OBJ_BASE+0x14000;e.set_u32(OBJ_BASE+16,pool);e.set_u32(pool+4,OBJ_BASE);e.set_u32(pool+8,0);e.set_u32(0x6f6a8c,pool);e.set_u32(0x696224,OBJ_BASE+0x4000);e.set_u32(0x696208,args[5]);e.set_u32(0x695bf0,color)
 e.call(0x4bac90,[args[1],args[2],0,1],'iiii')
 tick_manager=OBJ_BASE+0x15000;e.set_u32(tick_manager+0x1b0,1);e.set_u32(tick_manager+0x20,pool);e.write(OBJ_BASE+0x4000+0xb90,struct.pack('<d',.1));ticks=case%12+1;freed.clear()
 for _ in range(ticks):
  try:e.call(0x5796e0,this=tick_manager)
  except Exception:
   from unicorn.x86_const import UC_X86_REG_EIP
   print(case,_,hex(e.mu.reg_read(UC_X86_REG_EIP)));raise
 rows.append(','.join(map(str,args+[color]))+'\t'+raw.hex()+'\t'+e.read(OBJ_BASE,0xb0).hex()+'\t'+e.read(OBJ_BASE+0x20104,40).hex()+'\t'+str(ticks)+'\t'+e.read(pool+4,8).hex()+'\t'+e.read(manager+0x2000,4).hex()+'\t'+','.join(map(str,freed)))
pathlib.Path(__file__).with_name('golden_particle_life_full.tsv').write_text('\n'.join(rows)+'\n');print('200 complete particle chains captured')

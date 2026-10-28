"""Full root particle ctor, real bounds, controlled allocation/identities/manager."""
import pathlib,random,struct
from emu import Emu,OBJ_BASE
r=random.Random(0x59a780);e=Emu();rows=[];heap=[0];manager=OBJ_BASE+0x10000;vt=manager+0x1000
e.set_u32(manager,vt);e.set_u32(vt+0x14,0x593270);e.set_u32(manager+0x30,manager+0x2000);e.set_u32(manager+0x34,1)

def alloc(em):
 p=heap[0];heap[0]+=0x100;e.write(p,bytes(0x100));return p
e.hook_ret(0x632643,alloc)
for case in range(200):
 e.set_u32(manager+0x2000,0);heap[0]=OBJ_BASE+0x20000;e.write(OBJ_BASE,bytes(0xb0));color=r.getrandbits(32);e.set_u32(OBJ_BASE+0x1000,color)
 e.set_u32(0x6f6a68,manager);raw=struct.pack('<6f',*[r.uniform(-10,10) for _ in range(6)]);e.write(OBJ_BASE+0x1100,raw)
 args=[0,OBJ_BASE+0x1100,OBJ_BASE+0x110c,struct.unpack('<I',struct.pack('<f',r.uniform(0.1,5)))[0],OBJ_BASE+0x1000,r.randrange(1,1000)]
 pool=OBJ_BASE+0x14000;e.set_u32(pool+4,OBJ_BASE);e.set_u32(pool+8,0);e.set_u32(0x6f6a8c,pool);e.set_u32(0x696224,0);e.set_u32(0x696208,args[5]);e.set_u32(0x695bf0,color)
 e.call(0x4bac90,[args[1],args[2],0,1],'iiii')
 rows.append(','.join(map(str,args+[color]))+'\t'+raw.hex()+'\t'+e.read(OBJ_BASE,0xb0).hex()+'\t'+e.read(OBJ_BASE+0x20104,40).hex())
pathlib.Path(__file__).with_name('golden_particle_emit_full.tsv').write_text('\n'.join(rows)+'\n');print('200 complete particle chains captured')

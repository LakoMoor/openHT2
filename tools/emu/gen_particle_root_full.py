"""Full root particle ctor, real bounds, controlled allocation/identities/manager."""
import pathlib,random,struct
from emu import Emu,OBJ_BASE
r=random.Random(0x59a780);e=Emu();rows=[];heap=[0];manager=OBJ_BASE+0x10000;vt=manager+0x1000
e.set_u32(manager,vt);e.set_u32(vt+0x14,0x541000)
e.hook_thiscall(0x579310,lambda em:em.ecx,0);e.hook_thiscall(0x5fa6f0,lambda em:em.ecx,1);e.hook_thiscall(0x61dac3,lambda em:0,5);e.hook_thiscall(0x541000,lambda em:0,2)
def alloc(em):
 p=heap[0];heap[0]+=0x100;e.write(p,bytes(0x100));return p
e.hook_ret(0x632643,alloc)
for case in range(200):
 heap[0]=OBJ_BASE+0x20000;e.write(OBJ_BASE,bytes(0xb0));color=r.getrandbits(32);e.set_u32(OBJ_BASE+0x1000,color)
 args=[0,manager]+[struct.unpack('<I',struct.pack('<f',r.uniform(-5,5)))[0] for _ in range(5)]+[OBJ_BASE+0x1000,1,0,0]
 e.call(0x59a780,args,'i'*11,this=OBJ_BASE)
 rows.append(','.join(map(str,args+[color]))+'\t'+e.read(OBJ_BASE,0x84).hex()+'\t'+e.read(OBJ_BASE+0x20004,40).hex())
pathlib.Path(__file__).with_name('golden_particle_root_full.tsv').write_text('\n'.join(rows)+'\n');print('200 full root constructors captured')

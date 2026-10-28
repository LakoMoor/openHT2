"""0x59ab10 with root particle constructor replaced."""
import pathlib,random
from emu import Emu,OBJ_BASE
r=random.Random(0x59ab10);e=Emu();rows=[];seen=[None];point=OBJ_BASE+0x1000
def root(em):seen[0]=[em.arg(i) for i in range(11)];return em.ecx
e.hook_thiscall(0x59a780,root,11)
for case in range(300):
 args=[r.getrandbits(32) for _ in range(9)];args[1]=point;pool=r.getrandbits(32);raw=bytes(r.getrandbits(8) for _ in range(12));e.write(point,raw);e.write(OBJ_BASE,bytes(0x94));e.set_u32(0x6f6a68,pool);e.call(0x59ab10,args,'i'*9,this=OBJ_BASE)
 rows.append(','.join(map(str,args+[pool]))+'\t'+raw.hex()+'\t'+','.join(map(str,seen[0]))+'\t'+e.read(OBJ_BASE,0x94).hex())
pathlib.Path(__file__).with_name('golden_particle_base.tsv').write_text('\n'.join(rows)+'\n');print('300 particle base constructors captured')

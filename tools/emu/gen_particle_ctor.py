"""Particle constructor 0x59c420 with base constructor boundary controlled."""
import pathlib,random,struct
from emu import Emu,OBJ_BASE
r=random.Random(0x59c420);e=Emu();captured=[None];flags=[0];rows=[];point=OBJ_BASE+0x1000;direction=point+12

def base(em):
 captured[0]=[em.arg(i) for i in range(9)];em.set_u32(em.ecx+0x80,flags[0]);return em.ecx
e.hook_thiscall(0x59ab10,base,9)
for case in range(300):
 raw=bytes(r.getrandbits(8) for _ in range(24));texture=r.getrandbits(32);scale=r.uniform(0,5);config=r.getrandbits(32);count=r.randrange(1,10000);flags[0]=r.getrandbits(32)
 e.write(point,raw);e.write(OBJ_BASE,bytes(0xb0));e.call(0x59c420,[texture,point,direction,scale,config,count],'iiifii',this=OBJ_BASE)
 rows.append(','.join(map(str,[texture,struct.unpack('<I',struct.pack('<f',scale))[0],config,count,flags[0]]))+'\t'+raw.hex()+'\t'+','.join(map(str,captured[0]))+'\t'+e.read(OBJ_BASE,0xb0).hex())
pathlib.Path(__file__).with_name('golden_particle_ctor.tsv').write_text('\n'.join(rows)+'\n');print('300 particle constructors captured')

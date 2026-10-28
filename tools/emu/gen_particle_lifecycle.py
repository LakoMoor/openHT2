"""Native particle lifecycle; deletion observed, valid positive frame count."""
import pathlib,random,struct
from emu import Emu,OBJ_BASE
r=random.Random(0x59aa10);e=Emu();rows=[];deleted=[0];clock=OBJ_BASE+0x1000;colors=clock+0x2000;g=colors+0x2000
def remove(em):deleted[0]+=1;return 0
e.hook_thiscall(0x579370,remove,0)
for case in range(400):
 age=r.uniform(0,10);period=r.uniform(.1,2);duration=r.uniform(0,10);count=r.randrange(1,50);offset=r.randrange(0,10);flags=case%2;counter=r.getrandbits(32);dt=r.uniform(0,.2)
 raw=struct.pack('<3f5I',age,period,duration,count,offset,flags,counter,0);e.write(OBJ_BASE,bytes(0x84));e.set_u32(OBJ_BASE+0x58,clock);e.set_u32(OBJ_BASE+0x48,g);e.set_u32(OBJ_BASE+0x78,colors)
 for a,v in [(0x68,age),(0x60,period),(0x64,duration)]:e.write(OBJ_BASE+a,struct.pack('<f',v))
 for a,v in [(0x70,count),(0x74,offset),(0x80,flags),(0x7c,counter)]:e.set_u32(OBJ_BASE+a,v)
 for i in range(1000):e.set_u32(colors+i*4,(i*1234567)&0xffffffff)
 e.write(clock+0xb90,struct.pack('<d',dt));deleted[0]=0;e.call(0x59aa10,this=OBJ_BASE)
 rows.append(raw.hex()+'\t'+struct.pack('<d',dt).hex()+'\t'+e.read(OBJ_BASE+0x68,24).hex()+'\t'+e.read(g+0x24,4).hex()+'\t'+str(deleted[0]))
pathlib.Path(__file__).with_name('golden_particle_lifecycle.tsv').write_text('\n'.join(rows)+'\n');print('400 particle lifecycle steps captured')

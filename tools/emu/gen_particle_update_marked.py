"""Complete particle update chain, only deletion replaced."""
import pathlib,random,struct
from emu import Emu,OBJ_BASE
r=random.Random(0x59c510);e=Emu();rows=[];deleted=[0];clock=OBJ_BASE+0x1000;g=clock+0x1000;x=g+0x1000;colors=x+0x1000
def remove(em):deleted[0]+=1;return 0

for case in range(300):
 e.write(OBJ_BASE,bytes(0xb0));e.write(g,bytes(40));e.set_u32(OBJ_BASE+0x58,clock);e.set_u32(OBJ_BASE+0x48,g);e.set_u32(OBJ_BASE+0x78,colors);e.set_u32(g+0x10,x);e.set_u32(g+0x14,x+16)
 for i in range(1000):e.set_u32(colors+i*4,i*1234567&0xffffffff)
 for offset in [0x4c,0x50,0x54,0xa0,0xa4,0xa8,0x84,0x88,0x8c,0x90]:e.write(OBJ_BASE+offset,struct.pack('<f',r.uniform(-10,10)))
 for offset,value in [(0xac,r.choice([0.,3.,15.])),(0x60,r.uniform(.1,2)),(0x64,r.uniform(.1,5)),(0x68,r.uniform(0,7))]:e.write(OBJ_BASE+offset,struct.pack('<f',value))
 e.set_u32(OBJ_BASE+0x70,50);e.set_u32(OBJ_BASE+0x80,case%2);dt=struct.pack('<d',r.uniform(0,.3));e.write(clock+0xb90,dt);before=e.read(OBJ_BASE,0xb0);deleted[0]=0;e.call(0x59c510,this=OBJ_BASE)
 rows.append(before.hex()+'\t'+dt.hex()+'\t'+e.read(OBJ_BASE,0xb0).hex()+'\t'+e.read(x,32).hex()+'\t'+e.read(g+0x24,4).hex())
pathlib.Path(__file__).with_name('golden_particle_update_marked.tsv').write_text('\n'.join(rows)+'\n');print('300 full particle updates captured')

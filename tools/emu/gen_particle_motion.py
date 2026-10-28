"""Complete 0x59c510 motion, common update 0x59ac90 replaced."""
import pathlib,random,struct
from emu import Emu,OBJ_BASE
r=random.Random(0x59c510);e=Emu();rows=[];clock=OBJ_BASE+0x1000;calls=[0]
def update(em):calls[0]+=1;return 0
e.hook_thiscall(0x59ac90,update,0)
for case in range(400):
 values=[r.uniform(-10,10) for _ in range(6)]+[r.choice([0.,3.,15.])]
 if case%5==0:values[3:5]=[0.,0.]
 raw=struct.pack('<7f',*values);dt=struct.pack('<d',r.uniform(0,2));e.write(OBJ_BASE,bytes(0xb0));e.set_u32(OBJ_BASE+0x58,clock);e.write(OBJ_BASE+0x4c,raw[:12]);e.write(OBJ_BASE+0xa0,raw[12:]);e.write(clock+0xb90,dt);calls[0]=0;e.call(0x59c510,this=OBJ_BASE);assert calls[0]==1
 rows.append(raw.hex()+'\t'+dt.hex()+'\t'+e.read(OBJ_BASE,0xb0).hex())
pathlib.Path(__file__).with_name('golden_particle_motion.tsv').write_text('\n'.join(rows)+'\n');print('400 complete particle motions captured')

"""Native 0x59ac90 size interpolation; lifecycle tail replaced."""
import pathlib,random,struct
from emu import Emu,OBJ_BASE
r=random.Random(0x59ac90);e=Emu();rows=[];geometry=OBJ_BASE+0x1000;x=geometry+0x1000;z=x+16
e.set_u32(OBJ_BASE+0x48,geometry);e.set_u32(geometry+0x10,x);e.set_u32(geometry+0x14,z);e.hook_thiscall(0x59aa10,lambda em:0,0)
for case in range(400):
 values=[r.uniform(0.1,10),r.uniform(-5,15)]+[r.uniform(-10,10) for _ in range(4)];raw=struct.pack('<6f',*values);e.write(OBJ_BASE+0x64,raw[:8]);e.write(OBJ_BASE+0x84,raw[8:]);e.call(0x59ac90,this=OBJ_BASE)
 rows.append(raw.hex()+'\t'+e.read(x,32).hex())
pathlib.Path(__file__).with_name('golden_particle_size.tsv').write_text('\n'.join(rows)+'\n');print('400 particle size updates captured')

"""Real geometry bounds and merges in native texture attachment."""
import pathlib,random,struct
from emu import Emu,OBJ_BASE
r=random.Random(0x5fa640);e=Emu();rows=[];geometry=OBJ_BASE+0x1000
for case in range(300):
 count=case%5;origin=struct.pack('<3f',*[r.uniform(-10,10) for _ in range(3)]);vertices=struct.pack('<32f',*[r.uniform(-10,10) for _ in range(32)])
 e.write(OBJ_BASE,bytes(0x44));e.write(OBJ_BASE+0x38,origin)
 for i in range(count):
  descriptor=geometry+i*40;x=OBJ_BASE+0x2000+i*0x100;z=x+16;e.write(descriptor,bytes(40));e.write(descriptor+4,struct.pack('<H',4));e.set_u32(descriptor+0x10,x);e.set_u32(descriptor+0x14,z);e.write(x,vertices[i*32:i*32+32])
 e.call(0x5fa4f0,[geometry,count],'ii',this=OBJ_BASE);e.call(0x5fa5f0,this=OBJ_BASE)
 rows.append(str(count)+'\t'+origin.hex()+'\t'+vertices.hex()+'\t'+e.read(OBJ_BASE,0x44).hex())
pathlib.Path(__file__).with_name('golden_particle_attach_full.tsv').write_text('\n'.join(rows)+'\n');print('300 full attachments captured')

"""Native 0x4c5000 on nonzero descriptor memory."""
import pathlib,random
from emu import Emu,OBJ_BASE
r=random.Random(0x4c5000);e=Emu();rows=[]
for case in range(200):
 raw=bytes(r.getrandbits(8) for _ in range(40));e.write(OBJ_BASE,raw);e.call(0x4c5000,this=OBJ_BASE);rows.append(raw.hex()+'\t'+e.read(OBJ_BASE,40).hex())
pathlib.Path(__file__).with_name('golden_particle_descriptor_ctor.tsv').write_text('\n'.join(rows)+'\n');print('200 descriptor constructors captured')

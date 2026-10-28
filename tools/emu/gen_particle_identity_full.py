"""Real particle base identity and texture-node constructor chain."""
import pathlib,random
from emu import Emu,OBJ_BASE
r=random.Random(0x579310);e=Emu();rows=[];name=OBJ_BASE+0x1000;allocation=name+0x1000;e.hook_ret(0x632643,lambda em:allocation)
for case in range(200):
 raw=bytes(r.getrandbits(8) for _ in range(0xb0));globals_=[r.getrandbits(32) for _ in range(4)];value=bytes(r.randrange(1,128) for _ in range(case%20));e.write(name,value+b'\0');e.write(OBJ_BASE,raw)
 for i,v in enumerate(globals_):e.set_u32(0x6f4bc8+i*4,v)
 e.call(0x579310,this=OBJ_BASE);e.call(0x5fa6f0,[name],'i',this=OBJ_BASE+0x14)
 rows.append(raw.hex()+'\t'+','.join(map(str,globals_))+'\t'+value.hex()+'\t'+e.read(OBJ_BASE,0xb0).hex())
pathlib.Path(__file__).with_name('golden_particle_identity_full.tsv').write_text('\n'.join(rows)+'\n');print('200 particle identities captured')

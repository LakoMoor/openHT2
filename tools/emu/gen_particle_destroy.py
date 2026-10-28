"""Native root destructor with external detach/resource boundaries observed."""
import pathlib,random
from emu import Emu,OBJ_BASE
r=random.Random(0x59aa90);e=Emu();rows=[];trace=[]
for addr in [0x5e48f0,0x5ef780,0x5fa720]:
 def hook(em,addr=addr):trace.append([addr,em.ecx]);return 0
 e.hook_thiscall(addr,hook,0)
for case in range(300):
 raw=bytes(r.getrandbits(8) for _ in range(0xb0));e.write(OBJ_BASE,raw);trace.clear();e.call(0x5a2a30,this=OBJ_BASE)
 rows.append(raw.hex()+'\t'+e.read(OBJ_BASE,0xb0).hex()+'\t'+','.join(str(v) for pair in trace for v in pair))
pathlib.Path(__file__).with_name('golden_particle_destroy.tsv').write_text('\n'.join(rows)+'\n');print('300 particle destructors captured')

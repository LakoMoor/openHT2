"""Full particle destructor; only memory free observed."""
import pathlib,random,struct
from emu import Emu,OBJ_BASE
r=random.Random(0x59aa90);e=Emu();rows=[];trace=[]
def free(em):trace.append(em.arg(0));return 0
e.hook_ret(0x63266c,free)
for case in range(200):
 count=case%5;geometry=OBJ_BASE+0x1004 if count else 0;raw=bytearray(r.getrandbits(8) for _ in range(0xb0));node=OBJ_BASE+0x14
 for offset,value in [(0x18,0),(0x1c,0),(0x40,0),(0x48,geometry)]:struct.pack_into('<I',raw,offset,value)
 e.write(OBJ_BASE,raw);payload=bytes(r.getrandbits(8) for _ in range(count*40));e.write(OBJ_BASE+0x1000,struct.pack('<I',count)+payload);trace.clear();e.call(0x5a2a30,this=OBJ_BASE)
 rows.append(raw.hex()+'\t'+payload.hex()+'\t'+e.read(OBJ_BASE,0xb0).hex()+'\t'+','.join(map(str,trace)))
pathlib.Path(__file__).with_name('golden_particle_destroy_full.tsv').write_text('\n'.join(rows)+'\n');print('200 full particle destructors captured')

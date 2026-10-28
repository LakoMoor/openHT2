"""Native scalar deleting particle destructor, full recycle; only free observed."""
import pathlib,random,struct
from emu import Emu,OBJ_BASE
r=random.Random(0x5a2060);e=Emu();rows=[];pool=OBJ_BASE+0x200;free_obj=OBJ_BASE+0x300
e.hook_ret(0x63266c,lambda em:0)
for case in range(200):
 raw=bytearray(r.getrandbits(8) for _ in range(0x400))
 def put(o,v):struct.pack_into('<I',raw,o,v)
 for o,v in [(4,1),(8,0),(12,0),(16,pool),(0x18,0),(0x1c,0),(0x40,0),(0x48,0),(0x204,free_obj if case%2 else 0),(0x208,OBJ_BASE),(0x308,0),(0x30c,0)]:put(o,v)
 e.write(OBJ_BASE,raw);e.call(0x5a2060,[1],'i',this=OBJ_BASE)
 rows.append(raw.hex()+'\t'+e.read(OBJ_BASE,0x400).hex())
pathlib.Path(__file__).with_name('golden_particle_delete.tsv').write_text('\n'.join(rows)+'\n');print('200 particle deleting destructors captured')

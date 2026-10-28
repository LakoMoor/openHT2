"""Full 0x5fa6f0 node constructor; name allocator controlled."""
import pathlib,random
from emu import Emu,OBJ_BASE
r=random.Random(0x5fa6f0);e=Emu();rows=[];name=OBJ_BASE+0x1000;allocation=name+0x1000;seen=[]
def alloc(em):seen.append(em.arg(0));return allocation
e.hook_ret(0x632643,alloc)
for case in range(200):
 raw=bytes(r.getrandbits(8) for _ in range(0x44));value=bytes(r.randrange(1,128) for _ in range(case%20));pointer=0 if case%7==0 else name;e.write(name,value+b'\0');e.write(OBJ_BASE,raw);seen.clear();e.call(0x5fa6f0,[pointer],'i',this=OBJ_BASE)
 rows.append(raw.hex()+'\t'+value.hex()+'\t'+str(pointer)+'\t'+e.read(OBJ_BASE,0x44).hex()+'\t'+','.join(map(str,seen)))
pathlib.Path(__file__).with_name('golden_texture_node_ctor.tsv').write_text('\n'.join(rows)+'\n');print('200 texture node constructors captured')

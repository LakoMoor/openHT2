"""Original scenario scene cleanup 0x5c4910, with real unlinking and observed destruction."""
import pathlib,random
from emu import Emu,OBJ_BASE,STUB
r=random.Random(0x5c4910);e=Emu();c=OBJ_BASE;v=c+0x3000;events=[]
e.set_u32(v,STUB+0x400)
def destroy(q):
 assert q.arg(0)==1
 assert q.read(q.ecx+4,8)==bytes(8)
 events.append(q.ecx);return 0
e.hook_thiscall(STUB+0x400,destroy,1);rows=[]
for i in range(160):
 e.write(c,bytes(0x2800));e.write(c+0x4000,bytes(0x4000));counts=[r.randrange(8) for _ in range(11)];at=c+0x4000
 for k,count in enumerate(counts):
  root=c+0x1f28+k*0x20 if k<10 else c+0x206c;previous=root
  for _ in range(count):e.set_u32(previous+8,at);e.set_u32(at,v);e.set_u32(at+4,previous);previous=at;at+=0x20
 events.clear();e.call(0x5c4910,this=c)
 for k in range(11):assert e.read((c+0x1f28+k*0x20 if k<10 else c+0x206c)+8,4)==bytes(4)
 rows.append(','.join(map(str,counts))+'\t'+','.join(map(str,events)))
pathlib.Path(__file__).with_name('golden_controller_scene.tsv').write_text('\n'.join(rows)+'\n');print('160 scene resets captured')

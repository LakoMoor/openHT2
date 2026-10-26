"""Original tick/real-time queue draining 0x5d3850 with observed request callbacks."""
import pathlib,struct,random
from emu import Emu,OBJ_BASE
r=random.Random(0x5d3850);e=Emu();c=OBJ_BASE;seen=[]
def run(q):
 p=q.arg(0);seen.append(p);root=c+12 if struct.unpack('<I',q.read(c+12,4))[0]==p else c+8
 nxt=struct.unpack('<I',q.read(p+8,4))[0];q.set_u32(root,nxt)
 if nxt:q.set_u32(nxt+4,0)
 return 0
e.hook_thiscall(0x5d38c0,run,1);rows=[]
for case in range(200):
 e.write(c,bytes(0x4000));tick=r.randrange(30);clock=None if case%5==0 else r.randrange(-20,40)/1000
 e.set_u32(c,tick)
 if clock is not None:e.set_u32(c+24,c+0x100);e.write(c+0x100,struct.pack('<d',clock))
 queues=[sorted(r.randrange(-20,40) for _ in range(r.randrange(16))) for _ in range(2)]
 for k,times in enumerate(queues):
  base=c+0x1000+k*0x1000
  if times:e.set_u32(c+(12 if k==0 else 8),base)
  for i,t in enumerate(times):a=base+i*32;e.set_u32(a+16,t);e.set_u32(a+8,a+32 if i+1<len(times) else 0)
 seen.clear();e.call(0x5d3850,this=c)
 fields=[struct.unpack('<I',e.read(c+o,4))[0] for o in [0,12,8]]
 rows.append(f'{tick}\t'+('none' if clock is None else repr(clock))+'\t'+'\t'.join(','.join(map(str,q)) for q in queues)+'\t'+','.join(map(str,seen))+'\t'+','.join(map(str,fields)))
pathlib.Path(__file__).with_name('golden_scheduler_tick.tsv').write_text('\n'.join(rows)+'\n');print('200 scheduler ticks captured')

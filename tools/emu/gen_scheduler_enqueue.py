"""Original 0x5d3660 ordering and duplicate rejection."""
import pathlib,random,struct
from emu import Emu,OBJ_BASE,STUB
r=random.Random(0x5d3660);e=Emu();c=OBJ_BASE;v=c+0x9000;rejected=[];e.set_u32(v,STUB+0x400)
e.hook_thiscall(STUB+0x400,lambda q:rejected.append((q.ecx-c-0x10000)//0x20) or 0,1)
rows=[]
for case in range(180):
 e.write(c,bytes(0x20000));e.set_u32(v,STUB+0x400);tick=r.randrange(30);e.set_u32(c,tick);priorities=[r.randrange(8) for _ in range(6)];requests=[];rejected.clear()
 for i in range(35):
  t=r.randrange(6);delay=r.randrange(5);kind=r.randrange(3);requests.append((t,priorities[t],delay,kind));task=c+0x1000+t*0x100;req=c+0x10000+i*0x20
  e.set_u32(task+20,priorities[t]);e.set_u32(req,v);e.set_u32(req+12,task);e.set_u32(req+16,delay);e.set_u32(req+20,kind);e.call(0x5d3660,[req],'i',this=c)
 order=[];node=struct.unpack('<I',e.read(c+12,4))[0]
 while node:order.append((node-c-0x10000)//0x20);node=struct.unpack('<I',e.read(node+8,4))[0]
 rows.append(f'{tick}\t'+';'.join(','.join(map(str,p)) for p in requests)+'\t'+','.join(map(str,order))+'\t'+','.join(map(str,rejected)))
pathlib.Path(__file__).with_name('golden_scheduler_enqueue.tsv').write_text('\n'.join(rows)+'\n');print('180 scheduler queues captured')

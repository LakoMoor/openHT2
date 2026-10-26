"""Original request execution, with task effects observed rather than substituted state."""
import pathlib,struct
from emu import Emu,OBJ_BASE,STUB
E=Emu();c=OBJ_BASE;r=c+0x1000;t=c+0x2000;v=c+0x3000;events=[]
E.set_u32(v,STUB+0x400);E.set_u32(v+20,STUB+0x410)
E.hook_thiscall(0x5d2b20,lambda q:1,1)
def run(q):events.extend([q.ecx,struct.unpack('<I',q.read(t+16,4))[0],struct.unpack('<I',q.read(0x720118,4))[0]]);return 0
E.hook_ret(STUB+0x410,run)
E.hook_thiscall(STUB+0x400,lambda q:events.extend([3,q.ecx]) or 0,1)
E.hook_thiscall(0x5d35b0,lambda q:events.extend([2,q.arg(0)]) or 0,1)
rows=[]
for kind in [0,1,2,3,0xffffffff]:
 for count in [0,1,5,0xffffffff]:
  E.write(c,bytes(0x100));E.write(r,bytes(32));E.write(t,bytes(48));E.set_u32(r,v);E.set_u32(t,v)
  E.set_u32(r+12,t);E.set_u32(r+16,123);E.set_u32(r+20,kind);E.set_u32(t+16,count);E.set_u32(c+12,r);E.set_u32(0x720118,0)
  events.clear();E.call(0x5d38c0,[r],'i',this=c)
  fields=[struct.unpack('<I',E.read(a,4))[0] for a in [t+16,t+24,c+12,0x720118]]
  rows.append(f'{kind}\t{count}\t'+','.join(map(str,events))+'\t'+','.join(map(str,fields)))
pathlib.Path(__file__).with_name('golden_scheduler_request.tsv').write_text('\n'.join(rows)+'\n');print('20 request executions captured')

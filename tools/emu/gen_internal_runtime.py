"""End-to-end original scheduler -> internal task -> next-tick recurrence."""
import pathlib,struct
from emu import Emu,OBJ_BASE,STUB
rows=[]
for flags in [0,256]:
 for extra in [0,1]:
  e=Emu();manager=OBJ_BASE;c=manager+0x1000;v=manager+0x4000;free=[];state={'next':manager+0x10000};events=[]
  def allocate(q):
   a=free.pop() if free else state['next']
   if not free and a==state['next']:state['next']+=32
   q.write(a,bytes(24));return a
  e.hook_ret(0x5d3060,allocate);e.hook_thiscall(0x5d2b20,lambda q:1,1)
  e.hook_thiscall(0x5d30c0,lambda q:free.append(q.ecx) or 0,1)
  e.set_u32(c,v)
  for k,slot in enumerate([0x64,0x68,0x6c,0xac,0xb4,0xb8]):
   a=STUB+0x400+k*16;e.set_u32(v+slot,a)
   e.hook_ret(a,lambda q,s=slot:events.extend([s,struct.unpack('<I',q.read(c+0x224,4))[0]]) or 0)
  ids=[400,0,20,30,10]+([21,31,11] if flags else [])+([40] if extra else [])
  for k,id in enumerate(ids):
   task=manager+0x5000+k*64;r=manager+0x8000+k*32;e.set_u32(task,0x652518);e.set_u32(task+12,manager);e.set_u32(task+20,id);e.set_u32(task+32,c);e.set_u32(task+36,id)
   e.set_u32(r,0x651eb4);e.set_u32(r+12,task);e.set_u32(r+20,2);e.call(0x5d3660,[r],'i',this=manager)
  for tick in range(50):
   events.clear();e.call(0x5d3850,this=manager)
   assert struct.unpack('<I',e.read(manager,4))[0]==tick+1
   rows.append(f'{flags}\t{extra}\t{tick}\t'+','.join(map(str,events)))
pathlib.Path(__file__).with_name('golden_internal_runtime.tsv').write_text('\n'.join(rows)+'\n');print('200 consecutive runtime ticks captured')

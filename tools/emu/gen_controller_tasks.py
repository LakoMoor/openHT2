"""Original internal controller task dispatcher 0x5de430."""
import pathlib,struct
from emu import Emu,OBJ_BASE,STUB
E=Emu();t=OBJ_BASE;c=t+0x1000;v=t+0x2000;events=[]
E.set_u32(t+0x20,c);E.set_u32(c,v)
for k,slot in enumerate([0x64,0x68,0x6c,0xac,0xb4,0xb8]):
 a=STUB+0x400+k*16;E.set_u32(v+slot,a);E.hook_ret(a,lambda e,s=slot:events.append(s) or 0)
E.hook_thiscall(0x5d32f0,lambda e:events.append(e.arg(0)) or 0,1)
rows=[]
for id in [-1,0,1,9,10,11,12,19,20,21,22,29,30,31,32,39,40,41,399,400,401]:
 for old in [0,1,2,0xffffffff]:
  E.set_u32(t+0x24,id);E.set_u32(c+0x224,old);events.clear();E.call(0x5de430,this=t)
  value=struct.unpack('<I',E.read(c+0x224,4))[0]
  rows.append(f'{id}\t{old}\t{value}\t'+','.join(map(str,events)))
pathlib.Path(__file__).with_name('golden_controller_tasks.tsv').write_text('\n'.join(rows)+'\n');print('84 task dispatches captured')

"""Original end-frame task 0x5de7d0, camera recording disabled."""
import pathlib,struct
from emu import Emu,OBJ_BASE
E=Emu();c=OBJ_BASE;config=c+0x2000;events=[];E.set_u32(0x721750,config);E.set_u32(0x721764,0)
for addr,event in [(0x5ba3d0,1),(0x5aea50,3),(0x5fd160,4)]:
 E.hook_ret(addr,lambda e,v=event:events.append(v) or 0)
E.set_u32(c+0x1d4,c+0x3000);E.set_u32(c+0x3064,c+0x4000)
E.hook_thiscall(0x5b64e0,lambda e:events.append(2) or 0,1)
rows=[]
for dirty in [0,1,2,0xffffffff]:
 for audio in [0,1]:
  E.set_u32(c+0xc14,dirty);E.set_u32(config+0x158,audio);events.clear();E.call(0x5de7d0,this=c)
  assert E.read(c+0xc14,4)==bytes(4)
  rows.append(f'{dirty}\t{audio}\t'+','.join(map(str,events)))
pathlib.Path(__file__).with_name('golden_controller_frame_end.tsv').write_text('\n'.join(rows)+'\n');print('8 frame-end branches captured')



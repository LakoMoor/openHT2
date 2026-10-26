"""Original task registration order in 0x5de0a0."""
import sys,struct
sys.path.insert(0,'tools/emu')
from emu import Emu,OBJ_BASE
E=Emu();E.install_crt_hooks();c=OBJ_BASE;seen=[]
E.hook_thiscall(0x5d2540,lambda e:0,2);E.hook_ret(0x5d27c0,lambda e:0)
E.hook_thiscall(0x5d31e0,lambda e:e.ecx,2)
E.hook_thiscall(0x5d3570,lambda e:seen.append(('add',struct.unpack('<I',e.read(e.arg(0)+0x24,4))[0])) or 0,1)
E.hook_thiscall(0x5d33d0,lambda e:seen.append(('activate',struct.unpack('<I',e.read(e.ecx+0x24,4))[0])) or 0,1)
for flag in [0,256]:
 for extra in [0,1]:
  E.set_u32(0x702e8c,flag);E.set_u32(c+0xbe4,extra);seen.clear();E.call(0x5de0a0,this=c)
  expected=[400,0,20,30,10]+([21,31,11] if flag else [])+([40] if extra else [])
  assert seen==[(a,id) for id in expected for a in ['add','activate']],seen
  print(flag,extra,expected)


"""Original slot 0x64 scene-pass branch order with synthetic camera matrices."""
import pathlib,struct
from emu import Emu,OBJ_BASE,STUB
E=Emu();c=OBJ_BASE;scene=c+0x10000;cam=c+0x20000;obj=c+0x30000;v=c+0x40000;overlay=c+0x50000;config=c+0x60000;events=[]
u=lambda a:struct.unpack('<I',E.read(a,4))[0]
E.set_u32(0x721750,config);E.set_u32(c+0x1d0,scene);E.set_u32(c+0x1e8,cam);E.set_u32(obj,v);E.set_u32(v+0x38,STUB+0x400);E.set_u32(v+0x30,STUB+0x410);E.set_u32(overlay,v)
E.hook_ret(0x5b6190,lambda q:0);E.hook_ret(0x5b6180,lambda q:0);E.hook_ret(STUB+0x410,lambda q:events.append(1) or 0)
def matrix(q):q.write(q.arg(0),bytes(range(48)));return q.arg(0)
E.hook_thiscall(0x5dbd30,matrix,1);E.hook_ret(0x5d2fa0,lambda q:events.append(2) or 0);E.hook_ret(0x5d2fc0,lambda q:events.append(8) or 0)
E.hook_thiscall(0x5adc70,lambda q:events.extend([3,q.arg(0)]) or 0,1)
E.hook_thiscall(0x5ae0e0,lambda q:events.append(4) or 0,2)
E.hook_thiscall(STUB+0x400,lambda q:events.extend([5,q.arg(0)]) or 0,4)
E.hook_thiscall(0x5eb280,lambda q:events.extend([6,u(scene+0x21dc)]) or 0,4)
def reflect(q):events.append(7);q.write(q.arg(0),bytes(reversed(range(48))));return q.arg(0)
E.hook_ret(0x5ac2e0,reflect);rows=[]
for present in [0,1]:
 for target in [0,1]:
  for mirror in [0,1]:
   for flag in [0,256]:
    for audio in [0,1]:
     E.set_u32(c+0x1e8,cam if present else 0);E.set_u32(cam+0x4c,obj if target else 0);E.set_u32(c+0xb2c,overlay if mirror else 0);E.set_u32(0x702e8c,flag);E.set_u32(config+0x158,audio);E.set_u32(c+0x224,9);E.write(scene+0x2120,bytes(16));E.write(scene+0x2174,bytes(48));E.set_u32(scene+0x21dc,0)
     for i in range(4):E.set_u32(c+0xb90+i*4,100+i)
     events.clear();E.call(0x5df6f0,this=c)
     fields=[u(scene+0x2120+i*4) for i in range(4)]+[u(scene+0x21dc)]
     rows.append('\t'.join(map(str,[present,target,mirror,flag,audio]))+'\t'+','.join(map(str,events))+'\t'+','.join(map(str,fields)))
pathlib.Path(__file__).with_name('golden_controller_scene_pass.tsv').write_text('\n'.join(rows)+'\n');print('32 scene pass branches captured')

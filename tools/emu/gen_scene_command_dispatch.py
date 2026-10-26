"""Complete non-29 branches of native 0x4ce300; native player-flag write."""
import pathlib,random,struct
from emu import Emu,OBJ_BASE
e=Emu();command=OBJ_BASE;vehicle=command+0x10000;data=vehicle+0x10000;player=data+0x1000
e.set_u32(vehicle+0x509c,player);e.set_u32(command+12,data);trace=[];modifier=0;z=0.
e.hook_ret(0x4566c0,lambda q:0)
def query_fill(q):
    trace.append('query');e.set_u32(q.arg(1)+0x120,modifier);return 0
e.hook_ret(0x476930,query_fill)
def world(q):
    trace.append('world');dst=q.arg(0);e.write(dst,struct.pack('<12f',1,0,0,0,1,0,0,0,1,0,0,z));return dst
e.hook_thiscall(0x57de90,world,1)
r=random.Random(0x4ce33e);rows=[]
for case in range(400):
    kind=r.choice([0,16,16,16,3264,99]);modifier=case%2
    height=r.choice([-1.,0.,1.,10.,float('nan'),r.uniform(-20,20)])
    z=r.choice([height,height-1.,height+1.,float('nan'),r.uniform(-20,20)])
    raw=struct.pack('<2f',height,z);height,z=struct.unpack('<2f',raw);flags=r.getrandbits(32)
    e.set_u32(command,kind);e.write(data,raw[:4]);e.set_u32(player+0x60,flags);trace.clear()
    e.call(0x4ce300,[command,vehicle],'ii')
    rows.append('\t'.join(map(str,[kind,modifier,*struct.unpack('<2I',raw),flags,int.from_bytes(e.read(player+0x60,4),'little')]))+'\t'+';'.join(trace))
pathlib.Path(__file__).with_name('golden_scene_command_dispatch.tsv').write_text('\n'.join(rows)+'\n')
print('400 native scene command dispatches captured')

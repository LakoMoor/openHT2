"""Actual complete 0x4cef60; capture its scene/AI boundary calls."""
import pathlib,random,struct
from emu import Emu,OBJ_BASE

e=Emu();vehicle=OBJ_BASE;flags=vehicle+0x10000;trace=[];matrices=[];reads=0
e.set_u32(0x68b990,0x77777777)
for addr,label in [(0x54ca60,'mode'),(0x58f230,'move'),(0x58f290,'collision')]:
    e.hook_thiscall(addr,lambda q,label=label:trace.append(f'{label}:{q.arg(0)}') or 0,1)
def world(q):
    global reads
    dst=q.arg(0);q.write(dst,matrices[reads]);trace.append(f'world:{reads}');reads+=1;return dst
e.hook_thiscall(0x57de90,world,1)
def identity(q):
    q.set_u32(q.ecx,int.from_bytes(q.read(q.arg(0),4),'little'));return q.ecx
e.hook_thiscall(0x41d300,identity,1)
e.hook_ret(0x43f690,lambda q:trace.append(f'flags:{q.arg(0)},{q.arg(1)}') or 0)
r=random.Random(0x4cef60);rows=[]
for case in range(300):
    player=0x77777777 if case%3==0 else 0x12345678
    initial=r.getrandbits(32);local=[r.uniform(-10,10) for _ in range(3)]
    matrices=[struct.pack('<12f',*[r.uniform(-100,100) for _ in range(12)]) for _ in range(2)]
    if case%5==0:matrices[1]=matrices[0]
    local=struct.pack('<3f',*local)
    e.write(vehicle+0x5230,local);e.set_u32(flags,initial);trace.clear();reads=0
    e.call(0x4cef60,[vehicle,player,flags],'iii')
    result=list(struct.unpack('<15I',e.read(vehicle+0x50a0,60)))
    rows.append('\t'.join([str(player),str(initial),','.join(map(str,struct.unpack('<3I',local))),*[','.join(map(str,struct.unpack('<12I',m))) for m in matrices],','.join(map(str,result)),str(int.from_bytes(e.read(flags,4),'little')),';'.join(trace)]))
pathlib.Path(__file__).with_name('golden_ai_leave_physics.tsv').write_text('\n'.join(rows)+'\n')
print('300 complete leave-physics operations captured')

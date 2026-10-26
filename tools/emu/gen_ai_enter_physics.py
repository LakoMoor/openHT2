"""Actual complete 0x4cf080, with scene/snapshot/route boundary hooks.

Original vector normalization, f64 speed clamp and velocity products execute.
"""
import pathlib,random,struct
from emu import Emu,OBJ_BASE,STUB

e=Emu();vehicle=OBJ_BASE;body=vehicle+0x10;flags=vehicle+0x10000;vtable=vehicle+0x50000
e.set_u32(0x68b990,0x77777777);e.set_u32(body,vtable)
e.write(0x702620,struct.pack('<3f',0,1,0));e.write(0x702650,b'\0'*12)
trace=[];speed=0.;direction=[];matrix=b''
def world(q):
    dst=q.arg(0);q.write(dst,matrix);trace.append('world:0');return dst
e.hook_thiscall(0x57de90,world,1)
def identity(q):
    q.set_u32(q.ecx,int.from_bytes(q.read(q.arg(0),4),'little'));return q.ecx
e.hook_thiscall(0x41d300,identity,1)
def snapshot(q):
    q.write(q.arg(1)+0xa0,struct.pack('<d',speed));trace.append(f'speed:{q.arg(0)}');return 0
e.hook_ret(0x43fc40,snapshot)
def place(q):
    q.write(q.arg(0),b'\0'*32);return q.arg(0)
e.hook_ret(0x437520,place)
def heading(q):
    q.write(q.arg(0),struct.pack('<3f',*direction));trace.append('direction');return q.arg(0)
e.hook_ret(0x439320,heading)
e.hook_ret(0x43f690,lambda q:trace.append(f'flags:{q.arg(0)},{q.arg(1)}') or 0)
for addr,label in [(0x54ca60,'mode'),(0x58f2d0,'motion')]:
    e.hook_thiscall(addr,lambda q,label=label:trace.append(f'{label}:{q.arg(0)}') or 0,1)
for offset,stub,label in [(0x90,STUB+0x700,'linear'),(0x94,STUB+0x720,'angular')]:
    e.set_u32(vtable+offset,stub)
    def capture(q,label=label):
        assert q.arg(1)==0
        trace.append(label+':'+','.join(map(str,struct.unpack('<3I',q.read(q.arg(0),12)))));return 0
    e.hook_thiscall(stub,capture,2)
r=random.Random(0x4cf080);rows=[]
for case in range(300):
    player=0x77777777 if case%3==0 else 0x12345678;publish=case%2;initial=r.getrandbits(32)
    speed=r.choice([-20.,0.,0.123456789,17.99999999,18.,18.00000001,25.,r.uniform(-20,30)])
    fallback=r.uniform(-30,30);fallback=struct.unpack('<f',struct.pack('<f',fallback))[0]
    direction=[r.uniform(-3,3) for _ in range(3)] if case%7 else [0.,0.,0.]
    direction=list(struct.unpack('<3f',struct.pack('<3f',*direction)))
    matrix=struct.pack('<12f',*[r.uniform(-100,100) for _ in range(12)])
    e.write(vehicle+0x51e4,struct.pack('<f',fallback));e.set_u32(flags,initial);trace.clear()
    e.call(0x4cf080,[vehicle,player,flags,publish],'iiii')
    rows.append('\t'.join([str(player),str(publish),str(initial),str(int.from_bytes(struct.pack('<f',fallback),'little')),str(int.from_bytes(struct.pack('<d',speed),'little')),','.join(map(str,struct.unpack('<3I',struct.pack('<3f',*direction)))),','.join(map(str,struct.unpack('<12I',matrix))),str(int.from_bytes(e.read(flags,4),'little')),';'.join(trace)]))
pathlib.Path(__file__).with_name('golden_ai_enter_physics.tsv').write_text('\n'.join(rows)+'\n')
print('300 complete enter-physics operations captured')

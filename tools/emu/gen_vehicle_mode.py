"""Complete native 0x54ca60 mode orchestration with explicit boundary hooks."""
import pathlib,random,struct
from emu import Emu,OBJ_BASE,STUB
e=Emu();vehicle=OBJ_BASE;driver=vehicle+0x10000;vtable=vehicle+0x20000
e.set_u32(vehicle+0x5460,driver);e.set_u32(vehicle+0x10,vtable);e.set_u32(vtable+0x90,STUB+0x700)
trace=[];excessive=0
e.hook_ret(0x4e28b0,lambda q:trace.append('excessive') or excessive)
e.hook_ret(0x4e2821,lambda q:trace.append('check') or 0)
def clear(q):
    assert q.arg(1)==0
    trace.append('clear:'+','.join(map(str,struct.unpack('<3I',q.read(q.arg(0),12)))));return 0
e.hook_thiscall(STUB+0x700,clear,2)
def start(q):
    assert q.ecx==driver
    trace.append('start:'+','.join(str(q.arg(i)) for i in range(5)));return 0
e.hook_thiscall(0x4fc930,start,5)
r=random.Random(0x54ca60);rows=[]
offsets=[0x2aac,0x51ac,0x50d0,0x50d4,0x50d8,0x50e8,0x50ec,0x50f0]
for case in range(400):
    old=r.choice([-1,0,1,2,3]);value=r.choice([-1,0,1,1,2,3]);pending=r.choice([0,1,2]);bodymode=r.choice([0,1,2,3]);excessive=case%2
    initial=[old,pending]+[r.getrandbits(32) for _ in range(6)]
    zero=[r.getrandbits(32) for _ in range(3)];owner=r.getrandbits(32);active=r.getrandbits(32)
    for o,v in zip(offsets,initial):e.set_u32(vehicle+o,v)
    e.set_u32(vehicle+0xf98,bodymode);e.set_u32(vehicle+0x330,owner);e.set_u32(driver+0x2638,active)
    e.write(0x702650,struct.pack('<3I',*zero));trace.clear();e.call(0x54ca60,[value],'i',this=vehicle)
    output=[int.from_bytes(e.read(vehicle+o,4),'little') for o in offsets]+[int.from_bytes(e.read(driver+0x2638,4),'little')]
    rows.append('\t'.join([','.join(map(str,[value,bodymode,excessive,owner,active]+initial)),','.join(map(str,zero)),','.join(map(str,output)),';'.join(trace)]))
pathlib.Path(__file__).with_name('golden_vehicle_mode.tsv').write_text('\n'.join(rows)+'\n')
print('400 native vehicle-mode operations captured')

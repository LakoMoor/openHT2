"""Complete native command-29 branch: queries, arithmetic and queue rotation."""
import pathlib,random,struct
from emu import Emu,OBJ_BASE
e=Emu();command=OBJ_BASE;vehicle=command+0x10000;controller=vehicle+0x10000;plane=controller+0x1000
temporary=controller+0x2000;copy=temporary+0x100;payload=copy+0x100;old0=payload+0x100;old1=old0+0x100
kind=0;matrix=b'';trace=[];allocs=0;fail=False
e.write(0x702620,struct.pack('<3f',0,1,0))
def alloc(q):
    global allocs
    allocs+=1;trace.append(f'alloc:{q.arg(0)}');return [temporary,0 if fail else copy,payload][allocs-1]
e.hook_ret(0x632643,alloc);e.hook_ret(0x63266c,lambda q:trace.append(f'free:{q.arg(0)}') or 0)
def identity(q):q.set_u32(q.ecx,int.from_bytes(q.read(q.arg(0),4),'little'));return q.ecx
e.hook_thiscall(0x41d300,identity,1)
def snapshot(q):q.set_u32(q.arg(1)+8,kind);trace.append('kind');return 0
e.hook_ret(0x43fc40,snapshot)
def world(q):q.write(q.arg(0),matrix);trace.append('world');return q.arg(0)
e.hook_thiscall(0x57de90,world,1)
r=random.Random(0x4ce6a2);rows=[]
def bits(b):return ','.join(map(str,struct.unpack('<'+'I'*(len(b)//4),b)))
for case in range(300):
    present=case%17!=0;kind=r.choice([1,2,5,8]);special=case%2;code=r.choice([-11,-1,0,1,10,11,30,40,50]);state=r.getrandbits(2);has_plane=case%7!=0;fail=case%13==0
    matrix=struct.pack('<12f',*[r.uniform(-3,3) for _ in range(12)]);equation=struct.pack('<4f',*[r.uniform(-5,5) for _ in range(4)])
    slots=[old0 if case%2 else 0,old1 if case%3 else 0]
    e.set_u32(command,29);e.set_u32(command+4,code);e.set_u32(command+12,plane if has_plane else 0)
    e.set_u32(0x696338,controller if present else 0);e.set_u32(controller+0x24,state);e.set_u32(vehicle+0x55a4,special);e.write(plane,equation)
    e.set_u32(vehicle+0x5570,slots[0]);e.set_u32(vehicle+0x5574,slots[1]);e.set_u32(old1+12,old1+0x40)
    e.write(copy,b'\xcd'*16);e.write(payload,b'\xcd'*4);e.write(temporary,b'\xcd'*4);allocs=0;trace.clear()
    e.call(0x4ce300,[command,vehicle],'ii')
    output=[int.from_bytes(e.read(a,4),'little') for a in [vehicle+0x5570,vehicle+0x5574,temporary,copy,copy+4,copy+8,copy+12,payload]]
    rows.append('\t'.join([','.join(map(str,[int(present),kind,special,code,state,int(has_plane),int(fail)]+slots)),bits(matrix),bits(equation),','.join(map(str,output)),';'.join(trace)]))
pathlib.Path(__file__).with_name('golden_obstacle_receive.tsv').write_text('\n'.join(rows)+'\n')
print('300 complete native command-29 deliveries captured')

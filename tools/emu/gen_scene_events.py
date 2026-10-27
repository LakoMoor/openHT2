"""Original event traversal, hooked virtual children, volume and callback boundaries."""
import pathlib,random,struct,sys
native_children="--native-children" in sys.argv
native="--native-volumes" in sys.argv or native_children
from emu import Emu,OBJ_BASE,STUB
r=random.Random(0x5dbf00);e=Emu();base=OBJ_BASE;vt=base+0x900;trace=[]
def child(q):
    i=q.arg(0);n=q.ecx
    return int.from_bytes(e.read(n+0x50+i*4,4),'little') if i<2 else 0
e.hook_thiscall(STUB+0x100,child,1)
e.set_u32(vt+0x40,0x4b9b70 if native_children else STUB+0x100)
if not native:e.hook_thiscall(0x5f88d0,lambda q:int.from_bytes(e.read(q.ecx,4),'little')!=0,1)
def event(q):
    trace.extend([123,q.arg(0),q.arg(1),*struct.unpack('<3I',e.read(0x721298,12))])
    x=struct.unpack('<f',e.read(0x721298,4))[0];e.write(0x721298,struct.pack('<f',x+1))
    return 0
e.hook_ret(STUB+0x200,event)
e.write(0x7025f0,struct.pack('<12f',1,0,0,0,1,0,0,0,1,0,0,0))
rows=[]
for case in range(100 if native else 300):
    e.write(base,b'\0'*2048)
    for i in range(12):
        n=base+i*128;kind=r.choice([4,5,7,8,9,15,17,18,21,99]) if i else 7
        e.set_u32(n,vt);e.set_u32(n+0x10,kind);e.set_u32(n+0x28,r.choice([0,8]));e.set_u32(n+0x38,r.choice([0,1,2,0xffffffff]));e.set_u32(n+0x40,r.randrange(2))
        if i<6:
            for branch in range(2):e.set_u32(n+0x50+branch*4,base+(i*2+1+branch)*128 if i*2+1+branch<12 else 0)
        if kind==17:e.set_u32(n+0x40,base+(i+1)*128 if i<11 else 0)
        if kind==18 and i<11 and case%2:e.set_u32(n+0x3c,base+(i+1)*128)
    e.set_u32(base+0x738,1)
    e.write(base+0x740,struct.pack('<12f',1,0,0,0,1,0,0,0,1,r.uniform(-10,10),r.uniform(-10,10),r.uniform(-10,10)))
    for i in range(12):
        n=base+i*128;kind=int.from_bytes(e.read(n+0x10,4),'little')
        if kind in (4,17) and case%3:e.set_u32(n+(0x3c if kind==4 else 0x44),base+0x700)
        if i>6 and i<11 and case%5==0:e.set_u32(n+8,base+(i+1)*128)
    if native:
        for i in range(12):
            n=base+i*128;kind=int.from_bytes(e.read(n+0x10,4),'little')
            if kind not in (8,9):continue
            volume=n+(0x38 if kind==8 else 0x40);data=base+0xa00+i*32
            e.set_u32(volume,r.randrange(8));e.set_u32(volume+8,data)
            e.write(data,struct.pack('<8f',0,0,0,0,0,1,r.uniform(0,10),r.uniform(0,10)))
    if native_children:
        e.write(base+0xc00,b'\0'*256)
        for i in range(12):
            n=base+i*128;table=base+0xc04+i*16
            e.set_u32(n+0x30,table);e.set_u32(n+0x34,2)
            e.write(table,e.read(n+0x50,8))
    initial=e.read(base,3328 if native_children else 3072 if native else 2048);trace.clear()
    e.call(0x5dbec0,[base,3.,4.,5.,STUB+0x200,456],'ifffii')
    rows.append(initial.hex()+'\t'+','.join(map(str,trace))+'\t'+','.join(map(str,struct.unpack('<3I',e.read(0x721298,12)))))
pathlib.Path(__file__).with_name('golden_scene_events_native_children.tsv' if native_children else 'golden_scene_events_native_volumes.tsv' if native else 'golden_scene_events.tsv').write_text('\n'.join(rows)+'\n')
print(f'{len(rows)} native event traversals captured; native volumes={native}')

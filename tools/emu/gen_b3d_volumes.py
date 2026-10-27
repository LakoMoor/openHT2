"""Native 0x5f9300 record loading; only fread and allocator replaced."""
import pathlib,random,struct
from emu import Emu,OBJ_BASE
r=random.Random(0x5f9300);e=Emu();node=OBJ_BASE;heap=node+0x1000;cursor=0;raw=b''
def read(q):
    global cursor
    size=q.arg(1)*q.arg(2);e.write(q.arg(0),raw[cursor:cursor+size]);cursor+=size;return q.arg(2)
def alloc(q):
    global heap
    out=heap;heap+=(q.arg(0)+15)&~15;return out
e.hook_ret(0x61e20f,read);e.hook_ret(0x632643,alloc);rows=[]
for case in range(100):
    kind=13+case%5;vk=kind-13;cursor=0;heap=node+0x1000
    floats=[r.uniform(-100,100) for _ in range(4+(0 if vk==0 else 4 if vk<3 else 8))]
    params=[r.getrandbits(32) for _ in range(case%5)];surface=[r.getrandbits(32),r.getrandbits(32)]
    raw=struct.pack('<'+'f'*len(floats),*floats)+struct.pack('<'+'I'*(3+len(params)),*surface,len(params),*params)
    e.write(node,b'\0'*128);e.set_u32(node+0x40,vk)
    e.call(0x5f9300,[123,0,0],'iii',this=node)
    ptr=int.from_bytes(e.read(node+0x48,4),'little');geometry=e.read(ptr,0 if vk==0 else 16 if vk<3 else 32)
    rows.append('\t'.join([str(kind),raw.hex(),e.read(node+0x14,16).hex(),geometry.hex(),e.read(node+0x30,12).hex(),str(cursor)]))
pathlib.Path(__file__).with_name('golden_b3d_volumes.tsv').write_text('\n'.join(rows)+'\n')
print('100 native volume loads captured')

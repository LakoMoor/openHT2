"""Native 0x5f8ed0 conditional node loader, boundaries fread/allocator/child loader."""
import pathlib,random,struct
from emu import Emu,OBJ_BASE
r=random.Random(0x5f8ed0);e=Emu();node=OBJ_BASE;cursor=0;heap=node+4096;raw=b'';child=[]
def read(q):
 global cursor
 size=q.arg(1)*q.arg(2);e.write(q.arg(0),raw[cursor:cursor+size]);cursor+=size;return q.arg(2)
def alloc(q):
 global heap
 out=heap;heap+=(q.arg(0)+15)&~15;return out
def children(q):
 child.append(q.arg(1));return 0
e.hook_ret(0x61e20f,read);e.hook_ret(0x632643,alloc);e.hook_thiscall(0x5f1db0,children,5);rows=[]
for case in range(100):
 kind=10+case%2;vk=1 if kind==10 else 3;cursor=0;heap=node+4096;child.clear()
 floats=[r.uniform(-100,100) for _ in range(8 if kind==10 else 12)]
 raw=struct.pack('<'+'f'*len(floats),*floats)+struct.pack('<I',0)
 e.write(node,b'\0'*128);e.set_u32(node+0x38,vk);e.call(0x5f8ed0,[123,0,0],'iii',this=node)
 ptr=int.from_bytes(e.read(node+0x40,4),'little')
 rows.append('\t'.join([str(kind),raw.hex(),e.read(node+0x14,16).hex(),e.read(ptr,16 if kind==10 else 32).hex(),str(cursor),str(child[0])]))
pathlib.Path(__file__).with_name('golden_b3d_branches.tsv').write_text('\n'.join(rows)+'\n')
print('100 native conditional volume loads captured')

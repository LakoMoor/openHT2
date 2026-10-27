"""B3D fixtures + native volume loaders and complete point traversal.
File/heap boundaries and child parsing replaced; native child tables/traversal/math run.
"""
import pathlib,random,struct
from emu import Emu,OBJ_BASE,STUB
r=random.Random(0xb3de);e=Emu();base=OBJ_BASE;vt=base+0x800;cursor=0;raw=b'';heap=base+0x10000;trace=[]
w=lambda *x:struct.pack('<'+'I'*len(x),*x)
f=lambda x:struct.pack('<'+'f'*len(x),*x)
name=lambda x:x.encode().ljust(32,b'\0')
def read(q):
 global cursor
 size=q.arg(1)*q.arg(2);e.write(q.arg(0),raw[cursor:cursor+size]);cursor+=size;return q.arg(2)
def alloc(q):
 global heap
 out=heap;heap+=(q.arg(0)+15)&~15;return out
e.hook_ret(0x61e20f,read);e.hook_ret(0x632643,alloc);e.hook_thiscall(0x5f1db0,lambda q:0,5)
e.set_u32(vt+0x40,0x4b9b70)
def event(q):
 c=q.arg(0);kind,sub,count,data=struct.unpack('<4I',e.read(c,16));params=struct.unpack('<'+'I'*count,e.read(data,count*4));trace.append(','.join(map(str,[kind,sub,*params])));return 0
e.hook_ret(STUB+0x200,event)
rows=[]
for case in range(100):
 heap=base+0x10000;e.write(base,b'\0'*0x700);branch=base+0x100;events=[base+0x200,base+0x300]
 bk=10+case%2;vk=1 if bk==10 else 3;geometry=[r.uniform(-5,5) for _ in range(4 if bk==10 else 8)]
 if bk==11:geometry=[0,0,0,0,0,1,r.uniform(0,10),r.uniform(0,10)]
 body=f([0,0,0,100]+geometry)+w(2);raw=body;cursor=0;e.set_u32(branch+0x38,vk);e.call(0x5f8ed0,[123,0,0],'iii',this=branch)
 blocks=[]
 for i,n in enumerate(events):
  kind=r.choice([12,13,14,15,16,17]);vk=5 if kind==12 else kind-13
  geo=[] if vk==0 else [r.uniform(-8,8) for _ in range(4 if vk in (1,2,5) else 8)]
  if vk in (3,4):geo=[0,0,0,0,0,1,r.uniform(0,10),r.uniform(0,10)]
  params=[r.getrandbits(32) for _ in range(i+1)];surface=[29,case*10+i]
  eb=f([0,0,0,100]+geo)+w(*surface,len(params),*params);raw=eb;cursor=0;e.set_u32(n+0x40,vk);e.call(0x5f9300,[123,0,0],'iii',this=n)
  e.set_u32(n+0x10,9);blocks.append(w(333)+name('Event'+str(i))+w(kind)+eb+w(555))
 for n,kind in [(base,15),(branch,8)]:e.set_u32(n,vt);e.set_u32(n+0x10,kind)
 e.set_u32(base+0x30,base+0x600);e.set_u32(base+0x34,1);e.set_u32(base+0x600,branch)
 e.set_u32(branch+0x30,base+0x610);e.set_u32(branch+0x34,2)
 for i,n in enumerate(events):e.set_u32(base+0x610+i*4,n)
 trace.clear();e.call(0x5dbec0,[base,3.,4.,5.,STUB+0x200,456],'ifffii')
 nodes=w(111,333)+name('Room')+w(19,1,333)+name('Branch')+w(bk)+body+blocks[0]+w(444)+blocks[1]+w(555,555,222)
 mats=w(0);offset=7;length=offset+len(nodes)//4
 b3d=b'b3d\0'+w(length,6,1,offset,len(nodes)//4)+mats+nodes
 rows.append(b3d.hex()+'\t'+';'.join(trace))
pathlib.Path(__file__).with_name('golden_b3d_event_pipeline.tsv').write_text('\n'.join(rows)+'\n')
print('100 B3D event pipelines captured')

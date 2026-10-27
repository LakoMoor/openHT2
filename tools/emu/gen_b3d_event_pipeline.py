"""B3D fixtures + native volume loaders and complete point traversal.
File/heap boundaries and child parsing replaced; native child tables/traversal/math run.
"""
import pathlib,random,struct,sys
group="--group" in sys.argv
link="--link" in sys.argv or group
refer="--refer" in sys.argv
switch="--switch" in sys.argv or refer
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
 if switch:
  bk=21;key=case%4;body=f([0,0,0,100])+w(2,key,2);e.set_u32(branch+0x38,key)
 blocks=[]
 for i,n in enumerate(events):
  kind=r.choice([12,13,14,15,16,17]);vk=5 if kind==12 else kind-13
  geo=[] if vk==0 else [r.uniform(-8,8) for _ in range(4 if vk in (1,2,5) else 8)]
  if vk in (3,4):geo=[0,0,0,0,0,1,r.uniform(0,10),r.uniform(0,10)]
  params=[r.getrandbits(32) for _ in range(i+1)];surface=[29,case*10+i]
  eb=f([0,0,0,100]+geo)+w(*surface,len(params),*params);raw=eb;cursor=0;e.set_u32(n+0x40,vk);e.call(0x5f9300,[123,0,0],'iii',this=n)
  e.set_u32(n+0x10,9);blocks.append(w(333)+name('Event'+str(i))+w(kind)+eb+w(555))
 for n,kind in [(base,15),(branch,18 if switch else 8)]:e.set_u32(n,vt);e.set_u32(n+0x10,kind)
 e.set_u32(base+0x30,base+0x600);e.set_u32(base+0x34,1);e.set_u32(base+0x600,branch)
 e.set_u32(branch+0x30,base+0x610);e.set_u32(branch+0x34,2)
 for i,n in enumerate(events):e.set_u32(base+0x610+i*4,n)
 if link:
  space=base+0x400;ln=base+0x500;matrix=[0,1,0,-1,0,0,0,0,1,r.uniform(-8,8),r.uniform(-8,8),r.uniform(-8,8)]
  e.set_u32(space+0x10,6);e.set_u32(space+0x38,1);e.write(space+0x40,f(matrix));e.set_u32(ln+0x10,17);e.set_u32(ln+0x40,branch);e.set_u32(ln+0x44,space);e.set_u32(ln+8,space);e.set_u32(base+0x600,ln)
 if group:
  e.set_u32(ln,vt);variant=case%4;gkind=[4,5,23,21][variant];e.set_u32(ln+0x10,gkind);e.set_u32(ln+0x3c,space if variant==0 else 0);e.set_u32(ln+0x30,base+0x620);e.set_u32(ln+0x34,1);e.set_u32(base+0x620,branch)
 if refer:
  source=base+0x400;e.set_u32(source,vt);e.set_u32(source+0x10,18);e.set_u32(source+0x38,key);e.set_u32(branch+0x38,99);e.set_u32(branch+0x3c,source)
  e.set_u32(branch+8,source)
 trace.clear();e.call(0x5dbec0,[base,3.,4.,5.,STUB+0x200,456],'ifffii')
 nodes=w(111,333)+name('Room')+w(19,2 if refer else 1,333)+name('refer_Source' if refer else 'Branch')+w(bk)+body+blocks[0]+w(444)+blocks[1]+w(555,555,222)
 if refer:
  extra=w(333)+name('Source')+w(21)+f([0,0,0,100])+w(0,key,0,555)
  nodes=nodes[:-8]+extra+w(555,222)
 if link:
  branchblock=w(333)+name('Branch')+w(bk)+body+blocks[0]+w(444)+blocks[1]+w(555)
  linkblock=w(333)+name('Placed')+w(18)+f([0,0,0,100])+name('Space')+name('Branch')+w(555)
  spaceblock=w(333)+name('Space')+w(24)+f(matrix)+w(1,1)+branchblock+w(555)
  if group:
   label=['PlacedGroup','NormalGroup','envIgnored','lampGroup'][variant]
   groupbody=f([0,0,0,100])+name('Space' if variant==0 else 'NotUsedForEvents')+(name('Unused') if variant==0 else b'')+w(1)
   linkblock=w(333)+name(label)+w(4 if variant==0 else 5)+groupbody+branchblock+w(555)
   spaceblock=w(333)+name('Space')+w(24)+f(matrix)+w(1,0,555)
  nodes=w(111,333)+name('Room')+w(19,2)+linkblock+spaceblock+w(555,222)
 mats=w(0);offset=7;length=offset+len(nodes)//4
 b3d=b'b3d\0'+w(length,6,1,offset,len(nodes)//4)+mats+nodes
 result=b3d.hex()+'\t'+';'.join(trace)
 if switch:
  key=(case+1)%4;e.set_u32(source+0x38 if refer else branch+0x38,key);trace.clear();e.call(0x5dbec0,[base,3.,4.,5.,STUB+0x200,456],'ifffii');result+='\t'+str(key)+'\t'+';'.join(trace)
 if refer:result+='\tSource'
 rows.append(result)
pathlib.Path(__file__).with_name('golden_b3d_group_events.tsv' if group else 'golden_b3d_link_events.tsv' if link else 'golden_b3d_refer_events.tsv' if refer else 'golden_b3d_switch_events.tsv' if switch else 'golden_b3d_event_pipeline.tsv').write_text('\n'.join(rows)+'\n')
print('100 B3D event pipelines captured')

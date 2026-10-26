"""Original viewport and projection upload commands 0x5adea0."""
import pathlib,random,struct
from emu import Emu,OBJ_BASE,STUB
r=random.Random(0x5adea0);e=Emu();c=OBJ_BASE;device=c+0x10000;v=device+0x1000;state={}
e.set_u32(0x720908,device);e.set_u32(device,v);e.set_u32(v+0x34,STUB+0x400);e.set_u32(v+0x2c,STUB+0x410)
def clip(q):q.write(q.arg(0),struct.pack('<4i',*state['clip']));return 0
e.hook_thiscall(0x5b6710,clip,1)
e.hook_thiscall(STUB+0x400,lambda q:state.update(view=struct.unpack('<4I',q.read(q.arg(1),16))) or 0,2)
e.hook_thiscall(STUB+0x410,lambda q:state.update(matrix=struct.unpack('<16I',q.read(q.arg(2),64))) or 0,3)
rows=[]
for i in range(240):
 viewport=[r.randrange(-20,40)<<20,r.randrange(-20,40)<<20,r.randrange(1,600)<<20,r.randrange(1,400)<<20]
 near=r.choice([0.,.00001,.0001,.1,1.,10.]);far=r.uniform(100,10000);values=[r.uniform(-2,2) for _ in range(12)];raw=struct.pack('<12f',*values)
 state['clip']=[r.randrange(20),r.randrange(20),r.randrange(100,1600),r.randrange(100,1200)]
 for k,x in enumerate(viewport):e.set_u32(c+0x7a0+k*4,x)
 e.set_u32(c+0x2024,1);e.write(c+0x10d0,raw);e.write(c+0x2110,struct.pack('<d',near));e.write(c+0x2118,struct.pack('<d',far));e.call(0x5adea0,this=c)
 fields=[struct.unpack('<I',e.read(c+o,4))[0] for o in [0x2164,0x2168,0x216c,0x2170]]
 rows.append(','.join(str(x&0xffffffff) for x in viewport)+f'\t{near!r}\t{far!r}\t'+','.join(map(str,struct.unpack('<12I',raw)))+'\t'+','.join(map(str,state['clip']))+'\t'+','.join(map(str,state['view']))+'\t'+','.join(map(str,state['matrix']))+'\t'+','.join(map(str,fields)))
pathlib.Path(__file__).with_name('golden_scene_projection.tsv').write_text('\n'.join(rows)+'\n');print('240 viewport/projection command pairs captured')

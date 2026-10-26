"""Capture exact scene rectangle passed by original 0x532330."""
import pathlib,random,struct
from emu import Emu,OBJ_BASE,STUB
e=Emu();r=random.Random(0x532330);s=OBJ_BASE;surface=s+0x10000;scene=s+0x20000;vtable=scene+0x100;out=[];rows=[]
e.set_u32(s+0x24,surface);e.set_u32(s+0x20,scene);e.set_u32(scene,vtable);e.set_u32(vtable+0x2c,STUB+0x500)
e.hook_thiscall(STUB+0x500,lambda q:out.append([q.arg(i) for i in range(4)]) or 0,4)
for case in range(400):
 width=r.choice([640,779,780,800,1024,1920,3840]);height=r.choice([200,479,480,579,580,600,1080]);mode=r.randrange(-2,3)
 offsets=[r.randrange(-400,1200) for _ in range(4)];tables=[r.randrange(-100,200),r.randrange(-200,700),r.randrange(-300,900)];minimum=r.choice([0,50,90])
 if case%13==0:offsets=[r.choice([-2147483648,2147483647,2000000000]) for _ in range(4)];tables=[0,2000000000,0]
 e.set_u32(surface+0x100,width);e.set_u32(surface+0x104,height);e.set_u32(s+0x574,mode)
 for o,v in zip([0x558,0x55c,0x560,0x564],offsets):e.set_u32(s+o,v)
 for a,v in zip([0x696bf8,0x696c0c,0x696c20],tables):e.set_u32(a+mode*4,v)
 e.set_u32(0x672d7c,minimum);out.clear();e.call(0x532330,this=s);assert len(out)==1
 signed=lambda v:v if v<0x80000000 else v-0x100000000
 rows.append(','.join(map(str,[width,height,mode]+offsets+tables+[minimum]))+'\t'+','.join(str(signed(v)) for v in out[0]))
pathlib.Path(__file__).with_name('golden_view_rectangle.tsv').write_text('\n'.join(rows)+'\n');print('400 original view rectangles captured')

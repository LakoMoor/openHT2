"""Unhooked 0x4b9b70 including negative signed indices/counts."""
import pathlib,random,struct
from emu import Emu,OBJ_BASE
r=random.Random(0x4b9b70);e=Emu();n=OBJ_BASE;t=n+512;rows=[]
for case in range(400):
 count=r.choice([-3,-1,0,1,2,8,16]);index=r.randrange(-8,20);raw=r.randbytes(128)
 e.write(t-32,raw);e.set_u32(n+0x30,t);e.set_u32(n+0x34,count)
 e.call(0x4b9b70,[index],'i',this=n)
 rows.append('\t'.join(map(str,[count,index,raw.hex(),e.eax])))
pathlib.Path(__file__).with_name('golden_scene_children.tsv').write_text('\n'.join(rows)+'\n')
print('400 native child list lookups captured')

"""Original cabin gear/lamp/odometer commands, actual 0x53d1f0 branches."""
import pathlib,random,struct
from emu import Emu,OBJ_BASE
e=Emu();r=random.Random(0x53d465);c=OBJ_BASE;p=c+0x10000;digits=p+0x2000;commands=[];rows=[]
e.set_u32(c+0x578,1);e.set_u32(0x696cb0,p);e.set_u32(p+0x12fc,1)
def capture(q,digit):
 value=q.arg(1);value=value if value<0x80000000 else value-0x100000000
 commands.append(f'{q.arg(0)},{value},{digit}');return 0
e.hook_ret(0x4e48a2,lambda q:capture(q,0));e.hook_ret(0x4e480c,lambda q:capture(q,1))
for case in range(512):
 nodes=[0x100+i if case&(1<<i) else 0 for i in range(9)]
 for i,node in enumerate(nodes):e.set_u32(p+0x1318+i*4,node)
 gear=r.randrange(-1,15);automatic=r.choice([0,1,2,-1]);park=r.choice([0,1,2]);low=r.choice([0,1,2]);numbers=[r.randrange(10) for _ in range(5)]
 e.write(digits,struct.pack('<5i',*numbers));commands.clear()
 e.call(0x53d1f0,[0,0,0.,digits,0.,gear,automatic,park,0.,low,0],'iififiiifii',this=c)
 rows.append(','.join(map(str,nodes))+'\t'+','.join(map(str,[gear,automatic,park,low]+numbers))+'\t'+';'.join(commands))
pathlib.Path(__file__).with_name('golden_cabin_keys.tsv').write_text('\n'.join(rows)+'\n');print('512 cabin indicator combinations captured')

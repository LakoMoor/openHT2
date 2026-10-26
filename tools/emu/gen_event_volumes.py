"""Full original 0x5f88d0 calls without replacement hooks."""
import pathlib,random,struct
from emu import Emu,OBJ_BASE
r=random.Random(0x5f88d0);e=Emu();v=OBJ_BASE;data=v+0x100;point=v+0x200;rows=[]
for case in range(1200):
    kind=case%10
    a=[r.uniform(-5,5) for _ in range(8)];p=[r.uniform(-10,10) for _ in range(3)]
    if case%3==0:a=[0,0,0,0,0,1,3,4];p=r.choice([[0,0,0],[3,0,0],[3.0000003,0,0],[0,0,4],[0,0,4.000001],[0,0,5],[4,0,0]])
    if case%7==0:(a if case%2 else p)[r.randrange(8 if case%2 else 3)]=r.choice([float('nan'),float('inf'),-float('inf'),-0.])
    raw=struct.pack('<11f',*a,*p)
    e.set_u32(v,kind);e.set_u32(v+8,data);e.write(data,raw[:32]);e.write(point,raw[32:])
    e.call(0x5f88d0,[point],'i',this=v)
    rows.append(str(kind)+'\t'+raw.hex()+'\t'+str(e.eax))
pathlib.Path(__file__).with_name('golden_event_volumes.tsv').write_text('\n'.join(rows)+'\n')
print('1200 native point volume tests captured')

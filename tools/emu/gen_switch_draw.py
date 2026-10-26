"""Capture case selection by actual switch draw dispatch 0x5f9630.

Base visibility predicate and final child-list renderer are boundary hooks.
"""
import pathlib, random
from emu import Emu, OBJ_BASE

e=Emu(); node=OBJ_BASE; reference=node+0x1000
visible=1; selected=[]
e.hook_thiscall(0x5ef800,lambda q:visible,2)
def capture(q):
    selected.append(q.arg(0))
    return 0
e.hook_thiscall(0x5f2080,capture,3)
r=random.Random(0x5f9630);rows=[]
for i in range(400):
    count=r.randrange(5)
    key=r.choice([-2147483648,-1,0,1,2,4,2147483647])
    other=r.choice([-2147483648,-1,0,1,2,4,2147483647])
    linked=i%2;visible=int(i%7!=0)
    e.set_u32(node+0x34,count);e.set_u32(node+0x38,key)
    e.set_u32(node+0x3c,reference if linked else 0);e.set_u32(reference+0x38,other)
    selected.clear();e.call(0x5f9630,[0,0],'ii',this=node)
    chosen=selected[0] if selected else 0
    if chosen>=0x80000000:chosen-=0x100000000
    rows.append('\t'.join(map(str,[count,key,other,linked,visible,int(bool(selected)),chosen,e.eax])))
pathlib.Path(__file__).with_name('golden_switch_draw.tsv').write_text('\n'.join(rows)+'\n')
print('400 original switch draw dispatches captured')

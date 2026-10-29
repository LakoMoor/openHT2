"""Smoke particle constructor 0x59ad70 (with base 0x59ab10); root 0x59a780 and rand replaced."""
import pathlib,random,struct
from emu import Emu,OBJ_BASE
r=random.Random(0x59ad70);e=Emu()
u32=lambda a:struct.unpack('<I',e.read(a,4))[0]
F=lambda *v:struct.pack('<%df'%len(v),*v)
fb=lambda x:struct.unpack('<I',struct.pack('<f',x))[0]
obj=OBJ_BASE+0x40000;clock=OBJ_BASE+0x30000;pos=OBJ_BASE+0x100;dirp=OBJ_BASE+0x120
st={'rands':[],'used':0,'root':None}
def rand(em):
    v=st['rands'][st['used']];st['used']+=1;return v
def root(em):
    st['root']=[em.arg(i) for i in range(11)];return em.ecx
e.hook_ret(0x61edde,rand);e.hook_thiscall(0x59a780,root,11)
rows=[]
for case in range(500):
    p=F(*[r.uniform(-9,9) for _ in range(3)]);d=F(*[r.uniform(-2,2) for _ in range(3)])
    table=r.randrange(1<<32);frames=r.randrange(1,30);life=fb(r.uniform(0.1,6));mgr=r.randrange(1<<32)
    pre=bytes(r.randrange(256) for _ in range(0xc8));rands=[r.randrange(32768) for _ in range(2)]
    e.write(pos,p);e.write(dirp,d);e.write(obj,pre);e.write(0x6f6a68,struct.pack('<I',mgr))
    st.update(rands=rands,used=0,root=None)
    e.call(0x59ad70,[clock,pos,dirp,table,frames,life],'iiiiii',this=obj)
    rows.append(chr(9).join([p.hex(),d.hex(),','.join(map(str,[table,frames,life,mgr,clock])),','.join(map(str,rands)),pre.hex(),
        ','.join(map(str,st['root'])),e.read(obj,0xc8).hex(),str(st['used'])]))
pathlib.Path(__file__).with_name('golden_smoke_ctor.tsv').write_text(chr(10).join(rows)+chr(10))
print(len(rows),'cases')

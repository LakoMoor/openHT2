"""Fume particle update 0x59c000 (vtable 0x650c78 slot 1); size/lifecycle step 0x59ac90 replaced by a recorder."""
import math,pathlib,random,struct
from emu import Emu,OBJ_BASE
r=random.Random(0x59c000);e=Emu()
F=lambda *v:struct.pack('<%df'%len(v),*v)
obj=OBJ_BASE+0x40000;clock=OBJ_BASE+0x30000;geom=OBJ_BASE+0x50000;arr8=OBJ_BASE+0x51000;arr12=OBJ_BASE+0x52000;wind=OBJ_BASE+0x53000
e.write(0x702650,F(0,0,0));e.write(0x7025f0,F(1,0,0,0,1,0,0,0,1,0,0,0))
rec={'n':0,'pos':b''}
def step(em):
    rec['n']+=1;rec['pos']=e.read(obj+0x28,12);return 0
e.hook_thiscall(0x59ac90,step,0)
rows=[]
for case in range(500):
    pre=bytearray(r.randrange(256) for _ in range(0xc8))
    fl=lambda lo,hi:r.uniform(lo,hi)
    life=fl(0.5,6);age=r.choice([fl(-0.2,life*1.2),0.0,life])
    pre[0x58:0x5c]=struct.pack('<I',clock);pre[0x48:0x4c]=struct.pack('<I',geom)
    pre[0x64:0x68]=F(life);pre[0x68:0x6c]=F(age)
    home=[fl(-9,9) for _ in range(3)];pos=[h+r.choice([fl(-3,3),fl(-30,30)]) for h in home]
    pre[0x94:0xa0]=F(*home);pre[0x4c:0x58]=F(*pos)
    pre[0xa0:0xac]=r.choice([F(fl(-2,2),fl(-2,2),fl(-2,2)),F(0,0,0),F(0,0,fl(-1,1))])
    pre[0xac:0xb0]=r.choice([F(fl(0.1,25)),F(0.0),F(3.0)]);pre[0xb0:0xc0]=F(fl(-3,3),fl(-3,3),fl(0,1),fl(0,1))
    dt=r.choice([fl(0.001,0.2),0.016,0.0])
    e.write(clock+0xb90,struct.pack('<d',dt));e.write(obj,bytes(pre))
    e.write(0x702620,F(fl(-1,1),fl(-1,1),fl(-1,1)));gl=F(fl(0,6.3),fl(0,6.3),fl(0,6.3),fl(0,6.3));e.write(0x6f6a54,gl)
    w=r.random()<0.5;wv=fl(-5,5);e.write(wind+0x214,F(wv));e.write(0x7025b8,struct.pack('<I',wind if w else 0))
    e.write(geom+8,struct.pack('<II',arr8,arr12));e.write(arr8,bytes(16));e.write(arr12,bytes(16))
    rec['n']=0;e.call(0x59c000,[],'',this=obj)
    rows.append(chr(9).join([bytes(pre).hex(),struct.pack('<d',dt).hex(),F(wv).hex(),e.read(0x702620,12).hex(),gl.hex(),'1' if w else '0',
        e.read(obj,0xc8).hex(),e.read(arr8,16).hex(),e.read(arr12,16).hex(),str(rec['n']),rec['pos'].hex()]))
pathlib.Path(__file__).with_name('golden_fume_update.tsv').write_text(chr(10).join(rows)+chr(10))
print(len(rows),'cases')

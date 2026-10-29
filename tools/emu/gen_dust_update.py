"""Dust particle update 0x59dbc0 (vtable 0x650f34 slot 1); lifecycle 0x59aa10 replaced by a recorder."""
import math,pathlib,random,struct
from emu import Emu,OBJ_BASE
r=random.Random(0x59dbc0);e=Emu()
u32=lambda a:struct.unpack('<I',e.read(a,4))[0]
f32=lambda a:struct.unpack('<f',e.read(a,4))[0]
F=lambda *v:struct.pack('<%df'%len(v),*v)
obj=OBJ_BASE+0x40000;clock=OBJ_BASE+0x30000;geom=OBJ_BASE+0x50000;arr1=OBJ_BASE+0x51000;arr2=OBJ_BASE+0x52000;wind=OBJ_BASE+0x53000
# runtime constants the original fills at start-up (same as ht2_sim::globals::install_constants)
e.write(0x702620,F(0,1,0,0,0,1,1,0,0,0,0,0));e.write(0x702650,F(0,0,0))
ident=[1,0,0,0,1,0,0,0,1,0,0,0];e.write(0x7025f0,F(*ident))
e.write(0x696ec0,b''.join(struct.pack('<f',math.sin(((i*3.1415926)+(i*3.1415926))/360.0)) for i in range(360)))
rec={'called':0,'pos':b''}
def life(em):
    rec['called']=1;rec['pos']=e.read(obj+0x28,12);return 0
e.hook_thiscall(0x59aa10,life,0)
rows=[]
for case in range(500):
    pre=bytearray(r.randrange(256) for _ in range(0xc8))
    life_total=r.choice([r.uniform(0.1,6),1.0])
    age=r.choice([r.uniform(-0.5,life_total*1.3),0.0,life_total,life_total*1.2])
    fl=lambda lo,hi:r.uniform(lo,hi)
    pre[0x58:0x5c]=struct.pack('<I',clock);pre[0x48:0x4c]=struct.pack('<I',geom)
    pre[0x64:0x68]=F(life_total)[:4];pre[0x68:0x6c]=F(age)
    pre[0x4c:0x58]=F(fl(-9,9),fl(-9,9),fl(-9,9))
    pre[0x84:0x94]=F(fl(0.1,6),fl(0.1,6),fl(0.1,6),fl(0.1,6))
    pre[0x9c:0xa0]=F(fl(-3,3))
    pre[0xa0:0xac]=r.choice([F(fl(-2,2),fl(-2,2),fl(-2,2)),F(0,0,0),F(0,0,fl(-1,1))])
    pre[0xac:0xb8]=F(fl(-1,1),fl(-1,1),fl(-1,1));pre[0xb8:0xc0]=F(fl(-3.14,3.14),fl(0,0.05))
    dt=r.choice([r.uniform(0.001,0.2),0.016,0.0])
    e.write(clock+0xb90,struct.pack('<d',dt));e.write(obj,bytes(pre))
    e.write(0x702620,F(fl(-1,1),fl(-1,1),fl(-1,1)))  # wind axis
    e.write(0x6f6a70,F(life_total))
    w=r.random()<0.5;wv=fl(-5,5)
    e.write(wind+0x214,F(wv));e.write(0x7025b8,struct.pack('<I',wind if w else 0))
    e.write(geom+0x10,struct.pack('<II',arr1,arr2));e.write(arr1,bytes(16));e.write(arr2,bytes(16))
    rec['called']=0;e.call(0x59dbc0,[],'',this=obj)
    rows.append('\t'.join([bytes(pre).hex(),struct.pack('<d',dt).hex(),F(wv).hex(),e.read(0x702620,12).hex(),
        F(life_total).hex(),'1' if w else '0',e.read(obj,0xc8).hex(),e.read(arr1,16).hex(),e.read(arr2,16).hex(),str(rec['called']),rec['pos'].hex()]))
pathlib.Path(__file__).with_name('golden_dust_update.tsv').write_text('\n'.join(rows)+'\n')
print(len(rows),'cases')

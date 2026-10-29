"""Flash requests at the tail of 0x551970 (0x55278b..0x552930), sink 0x4baf90 recorded, other sinks replaced."""
import pathlib,random,struct
from emu import Emu,OBJ_BASE
r=random.Random(0x55278b);e=Emu();cfg=OBJ_BASE+0x10000
e.set_u32(0x6cec6c,cfg)
requests=[[]]
def flash(em):
    assert em.arg(2)==1
    requests[0].append(e.read(em.arg(0),12)+struct.pack('<I',em.arg(1)));return 0
for a in (0x4bb050,0x4bac90,0x4ba660):e.hook_ret(a,lambda em:0)
e.hook_ret(0x4baf90,flash);e.hook_ret(0x549bf0,lambda em:0)
def rf(n,lo=-3,hi=3):return [r.uniform(lo,hi) for _ in range(n)]
def special(v):
    return [0.,-0.,float('nan'),1e-30,float('inf')][r.randrange(5)] if r.random()<0.08 else v
rows=[]
for case in range(400):
    data=struct.pack('<24f',*rf(24))
    vid=case%4
    base=0x6978cc+vid*0x970
    tech=struct.pack('<%df'%0x70,*[special(x) for x in rf(0x70)])   # 0x6978cc.. relative to a vehicle record
    axis=struct.pack('<3f',*[special(x) for x in rf(3)])
    flags=[0x8000,0x10000,0x18000,0x8000|0x400,0x10000|0x20][r.randrange(5)]
    e.set_u32(OBJ_BASE+0x51a4,0x100);e.set_u32(OBJ_BASE+0x27b4,flags);e.set_u32(OBJ_BASE+0x28b4,vid)
    e.write(OBJ_BASE+0x2c4,data[:48]);e.write(OBJ_BASE+0x204,data[48:])
    e.write(OBJ_BASE+0x2821,bytes([0]));e.set_u32(OBJ_BASE+0x5460,0);e.set_u32(OBJ_BASE+0x28bc,0)
    e.write(base,tech);e.write(0x702630,axis)
    requests[0]=[];e.call(0x551970,this=OBJ_BASE)
    out=b''.join(requests[0]) if requests[0] else None
    rows.append('\t'.join([data.hex(),tech.hex(),axis.hex(),str(vid),str(flags),out.hex() if out else '-']))
pathlib.Path(__file__).with_name('golden_scene_flash_tail.tsv').write_text('\n'.join(rows)+'\n')
print(len(rows),'cases;',sum(1 for x in rows if not x.endswith('-')),'with flashes')

"""Exhaust branch of 0x551970 (0x551ff4..0x55278b). rand 0x61edde, 0x4ba660 and 0x549e60 recorded; the rest runs."""
import pathlib,random,struct
from emu import Emu,OBJ_BASE
r=random.Random(0x551ff4);e=Emu();cfg=OBJ_BASE+0x10000;cfg2=OBJ_BASE+0x30000
e.set_u32(0x6cec6c,cfg)
rec={'req':[],'hi':[],'rands':[],'used':0}
def rand(em):
    v=rec['rands'][rec['used']];rec['used']+=1;return v
def sink(em):
    assert em.arg(3)==1
    rec['req'].append(e.read(em.arg(0),12)+e.read(em.arg(1),12)+struct.pack('<I',em.arg(2)));return 0
def hi(em):
    rec['hi'].append(em.arg(0));return 0
e.hook_ret(0x61edde,rand);e.hook_ret(0x4ba660,sink);e.hook_thiscall(0x549e60,hi,1)
for a in (0x4bb050,0x4bac90,0x4baf90):e.hook_ret(a,lambda em:0)
def rf(n,lo=-3,hi_=3):return [r.uniform(lo,hi_) for _ in range(n)]
rows=[]
for case in range(500):
    vid=case%4;mode=[1,2,3,4,0][r.randrange(5)] if case%11==0 else 1+r.randrange(4)
    data=struct.pack('<24f',*rf(24))
    base=vid*0x970+0x6979f8
    tech=bytearray(struct.pack('<%df'%(0x5f0//4),*rf(0x5f0//4)))
    struct.pack_into('<f',tech,0x697db8-0x6979f8,r.uniform(0,3))
    for off in (0x697dc4,0x697dc8,0x697dcc):struct.pack_into('<f',tech,off-0x6979f8,r.uniform(0.5,4))
    for off in (0x697dd0,0x697dd4,0x697dd8):struct.pack_into('<f',tech,off-0x6979f8,r.uniform(-4,-0.5))
    struct.pack_into('<f',tech,0x697dc0-0x6979f8,r.uniform(-1,2))
    axes=struct.pack('<12f',*rf(12));vel0=struct.pack('<3f',*rf(3))
    x=r.choice([r.uniform(-4,40),r.uniform(0,3),0.0,r.uniform(-3,3)])
    e.set_u32(OBJ_BASE+0x51a4,0x100);e.set_u32(OBJ_BASE+0x27b4,0);e.set_u32(OBJ_BASE+0x28b4,vid)
    e.write(OBJ_BASE+0x2c4,data[:48]);e.write(OBJ_BASE+0x204,data[48:]);e.write(OBJ_BASE+0x1b0,vel0)
    e.write(OBJ_BASE+0x2821,bytes([0]));e.set_u32(OBJ_BASE+0x5460,0);e.set_u32(OBJ_BASE+0x28bc,0)
    e.write(OBJ_BASE+0x4fb0,bytes(16))
    e.set_u32(OBJ_BASE+0x5454,cfg2);e.write(cfg2+8,struct.pack('<f',x));e.set_u32(cfg2+0xc,mode)
    e.write(base,bytes(tech));e.write(0x702620,axes[:12]);e.write(0x702630,axes[12:24] if False else struct.pack('<3f',*rf(3)));e.write(0x702640,struct.pack('<3f',*rf(3)))
    ax=e.read(0x702620,12)+e.read(0x702630,12)+e.read(0x702640,12)
    rands=[r.choice([r.randrange(32768),0,32767,16384,r.randrange(32768)]) for _ in range(400)]
    rec.update(req=[],hi=[],rands=rands,used=0)
    e.call(0x551970,this=OBJ_BASE)
    out=';'.join(q.hex() for q in rec['req']) or '-'
    rows.append('\t'.join([data.hex(),bytes(tech).hex(),ax.hex(),vel0.hex(),struct.pack('<fII',x,mode,vid).hex(),
        ','.join(map(str,rands[:rec['used']])),','.join(map(str,rec['hi'])) or '-',out]))
pathlib.Path(__file__).with_name('golden_scene_exhaust_branch.tsv').write_text('\n'.join(rows)+'\n')
print(len(rows),'cases;',sum(1 for x in rows if not x.endswith('\t-')),'emitted;',sum(1 for x in rows if x.split('\t')[6]!='-'),'high')

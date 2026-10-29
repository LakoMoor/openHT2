"""Whole 0x551970 with every sink recorded in call order: 0x4bac90 (E), 0x4bb050 (W), 0x549e60 (H), 0x4ba660 (X), 0x4baf90 (F), rand."""
import pathlib,random,struct
from emu import Emu,OBJ_BASE
r=random.Random(0x551970a);e=Emu();cfg=OBJ_BASE+0x10000;clk=OBJ_BASE+0x20000;owner_addr=OBJ_BASE+0x30000;cfg2=OBJ_BASE+0x40000
e.set_u32(0x6cec6c,cfg);e.set_u32(0x6ef1fc,clk)
st={'ev':[],'rands':[],'used':0,'linked':0}
def rand(em):
    v=st['rands'][st['used']];st['used']+=1;return v
def effect(em):
    st['ev'].append('E:'+(e.read(em.arg(0),12)+e.read(em.arg(1),12)).hex());return 0
def wheel(em):
    p=struct.pack('<I',em.arg(0)) if st['linked'] else e.read(em.arg(0),12)
    st['ev'].append('W:'+(p+e.read(em.arg(1),12)+struct.pack('<I',em.arg(2))).hex());return 0
def exhaust(em):
    st['ev'].append('X:'+(e.read(em.arg(0),12)+e.read(em.arg(1),12)+struct.pack('<I',em.arg(2))).hex());return 0
def flash(em):
    st['ev'].append('F:'+(e.read(em.arg(0),12)+struct.pack('<I',em.arg(1))).hex());return 0
def high(em):
    st['ev'].append('H');return 0
e.hook_ret(0x61edde,rand);e.hook_ret(0x4bac90,effect);e.hook_ret(0x4bb050,wheel);e.hook_ret(0x4ba660,exhaust);e.hook_ret(0x4baf90,flash);e.hook_thiscall(0x549e60,high,1)
def rf(n,lo=-3,hi=3):return [r.uniform(lo,hi) for _ in range(n)]
rows=[]
for case in range(600):
    vid=case%4
    data=struct.pack('<24f',*rf(24))
    base=vid*0x970+0x6978cc
    tech=bytearray(struct.pack('<%df'%(0x734//4),*rf(0x734//4)))
    def put(abs_,v):struct.pack_into('<f',tech,abs_-0x6978cc,v)
    put(0x697db8,r.uniform(0,3));put(0x697dc0,r.uniform(-1,2))
    for o in (0x697dc4,0x697dc8,0x697dcc):put(o,r.uniform(0.5,4))
    for o in (0x697dd0,0x697dd4,0x697dd8):put(o,r.uniform(-4,-0.5))
    if r.random()<0.7:
        for o in (0x697fb0,0x697fa8+8,0x697fa8+8+24):put(o,r.uniform(-12,12))
    axes=struct.pack('<9f',*rf(9));vel0=struct.pack('<3f',*rf(3))
    flags=r.choice([0,0x400,0x8000,0x10000,0x18000,0x8400,0x10400,0x20])
    f51=0x100|r.choice([0,2,2])
    x=r.choice([r.uniform(-4,40),r.uniform(0,3),0.0]);mode=r.choice([0,1,2,3,4,4])
    linked=r.randrange(2);count=r.randrange(0,6) if linked else 0;maskb=r.randrange(256)
    vals=bytearray(struct.pack('<12f',*rf(12,-0.5,1.0)))
    pos=struct.pack('<12f',*rf(12));idx=struct.pack('<4i',*[r.randrange(6) for _ in range(4)])
    e.set_u32(OBJ_BASE+0x51a4,f51);e.set_u32(OBJ_BASE+0x27b4,flags);e.set_u32(OBJ_BASE+0x28b4,vid)
    e.write(OBJ_BASE+0x2c4,data[:48]);e.write(OBJ_BASE+0x204,data[48:]);e.write(OBJ_BASE+0x1b0,vel0)
    e.write(OBJ_BASE+0x27f0,bytes(vals));e.write(OBJ_BASE+0x282c,pos);e.write(OBJ_BASE+0x4fb0,idx);e.write(OBJ_BASE+0x2821,bytes([maskb]))
    e.set_u32(OBJ_BASE+0x5460,owner_addr if linked else 0);e.set_u32(OBJ_BASE+0x28bc,count)
    e.set_u32(OBJ_BASE+0x5454,cfg2);e.write(cfg2+8,struct.pack('<f',x));e.set_u32(cfg2+0xc,mode)
    e.set_u32(cfg+0x514,0)
    e.write(base,bytes(tech));e.write(0x702620,axes[:12]);e.write(0x702630,axes[12:24]) if False else None
    e.write(0x702620,struct.pack('<3f',*rf(3)));e.write(0x702630,struct.pack('<3f',*rf(3)));e.write(0x702640,struct.pack('<3f',*rf(3)))
    ax=e.read(0x702620,12)+e.read(0x702630,12)+e.read(0x702640,12)
    rands=[r.choice([r.randrange(32768),0,32767,16384]) for _ in range(400)]
    st.update(ev=[],rands=rands,used=0,linked=linked)
    e.call(0x551970,this=OBJ_BASE)
    rows.append('\t'.join([data.hex(),bytes(tech).hex(),ax.hex(),vel0.hex(),bytes(vals).hex(),pos.hex(),idx.hex(),
        ','.join(map(str,[vid,f51,flags,linked,owner_addr,count,maskb,mode])),struct.pack('<f',x).hex(),
        ','.join(map(str,rands[:st['used']])),';'.join(st['ev']) or '-']))
pathlib.Path(__file__).with_name('golden_scene_effects_full.tsv').write_text('\n'.join(rows)+'\n')
from collections import Counter
c=Counter(t[0] for x in rows for t in x.split('\t')[-1].split(';') if t!='-');print(len(rows),'cases',dict(c))

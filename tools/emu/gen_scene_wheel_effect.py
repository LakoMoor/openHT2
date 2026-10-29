"""Wheel dust/spray requests of 0x551970 (0x551cf3..0x551ff4), sink 0x4bb050 replaced."""
import pathlib,random,struct
from emu import Emu,OBJ_BASE
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_EIP,UC_X86_REG_ESP
r=random.Random(0x551cf3);e=Emu();cfg=OBJ_BASE+0x10000;owner=OBJ_BASE+0x20000
e.set_u32(0x6cec6c,cfg)
requests=[[]];mode=[0]
def sink(em):
    assert em.arg(3)==1
    pos=struct.pack('<I',em.arg(0)) if mode[0] else e.read(em.arg(0),12)
    requests[0].append(pos+e.read(em.arg(1),12)+struct.pack('<I',em.arg(2)))
    return 0
e.hook_ret(0x4bb050,sink)
e.hook_ret(0x4bac90,lambda em:0)
def stop(mu,a,s,u):
    mu.reg_write(UC_X86_REG_ESP,mu.reg_read(UC_X86_REG_ESP)+8);mu.reg_write(UC_X86_REG_EIP,0x552933)
e.mu.hook_add(UC_HOOK_CODE,stop,begin=0x551ff4,end=0x551ff4)
def rf(n,lo=-3,hi=3):return [r.uniform(lo,hi) for _ in range(n)]
rows=[]
for case in range(400):
    data=struct.pack('<24f',*rf(24))
    vid=case%4
    tech=bytearray(struct.pack('<18f',*rf(18)))
    for i in range(0,18,3):
        if r.random()<0.3:tech[i*4:i*4+4]=struct.pack('<f',[0.,-0.,float('nan'),1e-30][r.randrange(4)])
    axis=struct.pack('<3f',*rf(3))
    idx=struct.pack('<4i',*[r.randrange(6) for _ in range(4)])
    vals=bytearray(struct.pack('<12f',*rf(12,-0.5,1.0)))
    for i in range(0,48,4):
        if r.random()<0.15:vals[i:i+4]=struct.pack('<f',[0.1,0.100000001,float('nan'),0.0][r.randrange(4)])
    pos=struct.pack('<12f',*rf(12))
    m=r.randrange(2);count=r.randrange(0,6) if m else 0;maskb=r.randrange(256)
    mode[0]=m
    e.set_u32(OBJ_BASE+0x51a4,0x100);e.set_u32(OBJ_BASE+0x27b4,0);e.set_u32(OBJ_BASE+0x28b4,vid)
    e.write(OBJ_BASE+0x2c4,data[:48]);e.write(OBJ_BASE+0x204,data[48:])
    e.write(OBJ_BASE+0x27f0,bytes(vals));e.write(OBJ_BASE+0x282c,pos);e.write(OBJ_BASE+0x4fb0,idx)
    e.write(OBJ_BASE+0x2821,bytes([maskb]))
    e.set_u32(OBJ_BASE+0x5460,owner if m else 0);e.set_u32(OBJ_BASE+0x28bc,count)
    e.write(0x697a10+vid*0x970,bytes(tech));e.write(0x702640,axis)
    requests[0]=[];e.call(0x551970,this=OBJ_BASE)
    out=b''.join(requests[0]) if requests[0] else None
    rows.append('\t'.join([data.hex(),bytes(tech).hex(),axis.hex(),idx.hex(),bytes(vals).hex(),pos.hex(),
        ','.join(map(str,[vid,m,count,maskb])),out.hex() if out else '-']))
pathlib.Path(__file__).with_name('golden_scene_wheel_effect.tsv').write_text('\n'.join(rows)+'\n')
print('400 wheel effect cases;',sum(1 for x in rows if not x.endswith('-')),'with requests')

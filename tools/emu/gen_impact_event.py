"""Impact event block of FUN_004ff800 (0x4ff917..0x4ffb22): real code run from the middle of the function."""
import pathlib,random,struct
from emu import Emu,OBJ_BASE
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import *
r=random.Random(0x4ff917);e=Emu();mu=e.mu
owner=OBJ_BASE;SP=0x2fff0000;STOP=0x4ffb22;SIZE=0x4100
def rf():return r.choice([r.uniform(-3,3),0.0,0.5,0.001,1.0,r.uniform(0,1),float('nan')] if r.random()<0.04 else [r.uniform(-3,3),r.uniform(0,1),0.0,0.5,0.001,1.0])
rows=[]
for case in range(500):
    mem=bytearray(SIZE)
    def f(o,v):struct.pack_into('<f',mem,o,v)
    for o in list(range(0x10,0x40,4))+[0x1844,0x1848,0x184c,0x1850,0x1854,0x1858,0x1b74,0x1b78,0x29dc,0xdc8]:f(o,rf())
    for o in (0x29dc,0xdc8):
        if r.random()<0.75:f(o,r.uniform(0.55,1.0))
    for k in range(0x3f8c,0x3f8c+0x40,4):f(k,rf())
    struct.pack_into('<I',mem,0x3f8c+0x28,r.randrange(0,3))
    struct.pack_into('<H',mem,0x19a4,r.choice([0,0,0,0,0,1]));struct.pack_into('<H',mem,0x19a6,r.choice([0,0,0,0,0,2]))
    for o in (0x17dc,0x3c18,0x3c1c,0x3c20,0x3c24,0x3c28,0x3c2c,0x3c30):struct.pack_into('<I',mem,o,r.randrange(1<<32))
    now=struct.pack('<f',rf());axis=struct.pack('<3f',*[r.uniform(-1,1) for _ in range(3)])
    e.write(owner,bytes(mem));e.write(0x702630,axis);e.write(SP,bytes(0x80));e.write(SP+0x10,now)
    mu.reg_write(UC_X86_REG_ESI,owner);mu.reg_write(UC_X86_REG_ESP,SP)
    h=mu.hook_add(UC_HOOK_CODE,lambda u,a,s,d:u.emu_stop(),begin=STOP,end=STOP)
    mu.emu_start(0x4ff917,0,count=2000);mu.hook_del(h)
    out=e.read(owner,SIZE)
    sel=lambda b:b''.join(b[o:o+4] for o in (0x17dc,0x3c18,0x3c1c,0x3c20,0x3c24,0x3c28,0x3c2c,0x3c30))
    rows.append('\t'.join([bytes(mem).hex(),axis.hex(),now.hex(),sel(out).hex()]))
pathlib.Path(__file__).with_name('golden_impact_event.tsv').write_text('\n'.join(rows)+'\n')
n=sum(1 for x in rows if x.split('\t')[3][8:16]!=x.split('\t')[0][0:0] )
print(len(rows))

"""Contact value + impact event of FUN_004ff800 (0x4ff809..0x4ffb22): real code incl. the clock call 0x4e27a5."""
import pathlib,random,struct
from emu import Emu,OBJ_BASE
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import *
r=random.Random(0x4ff809);e=Emu();mu=e.mu
owner=OBJ_BASE;clock=OBJ_BASE+0x8000;SP=0x2fff0000;STOP=0x4ffb22;SIZE=0x4100
OUT=(0x17dc,0x29d8,0x29dc,0x1c1c,0x3c18,0x3c1c,0x3c20,0x3c24,0x3c28,0x3c2c,0x3c30)
def rf(extra=()):return r.choice([r.uniform(-3,3),r.uniform(0,1),0.0,0.5,0.001,1.0,0.6,float('nan')]+list(extra))
rows=[]
for case in range(600):
    mem=bytearray(SIZE)
    def f(o,v):struct.pack_into('<f',mem,o,v)
    def w(o,v):struct.pack_into('<H',mem,o,v)
    for o in list(range(0x10,0x40,4))+[0x1844,0x1848,0x184c,0x1850,0x1854,0x1858,0x1b74,0x1b78,0xdc8,0x17dc,0x2710,0x3bd4]:f(o,rf())
    if r.random()<0.7:f(0xdc8,r.uniform(0.55,1.0))
    for k in range(0x3f8c,0x3f8c+0x40,4):f(k,rf())
    struct.pack_into('<I',mem,0x3f8c+0x28,r.randrange(0,3))
    w(0x2716,r.choice([0,0,0,1,2,3]));w(0x2714,r.choice([0,0,0,1]));w(0x2704,r.choice([0,1,1,5]));w(0x2718,r.choice([0,1,2,3,0xffff]))
    w(0x19a4,r.choice([0,0,0,0,0,1]));w(0x19a6,r.choice([0,0,0,0,0,2]))
    for o in OUT:struct.pack_into('<I',mem,o,r.randrange(1<<32))
    flag=r.choice([0,1]);tf=struct.pack('<f',r.uniform(0,100));td=struct.pack('<d',r.uniform(0,100))
    if r.random()<0.5:  # make 17dc close to the time so that deltas are small
        struct.pack_into('<f',mem,0x17dc,struct.unpack('<f',tf)[0]-r.uniform(0,2))
    e.write(owner,bytes(mem));e.write(0x702630,struct.pack('<3f',*[r.uniform(-1,1) for _ in range(3)]))
    e.write(0x6974c2,struct.pack('<H',flag));e.write(0x6974bc,tf);e.set_u32(0x6974c4,clock);e.write(clock+0xb98,td)
    e.write(SP,bytes(0x80))
    mu.reg_write(UC_X86_REG_ESI,owner);mu.reg_write(UC_X86_REG_ESP,SP)
    h=mu.hook_add(UC_HOOK_CODE,lambda u,a,s,d:u.emu_stop(),begin=STOP,end=STOP)
    mu.emu_start(0x4ff809,0,count=3000);mu.hook_del(h)
    out=e.read(owner,SIZE)
    rows.append('\t'.join([bytes(mem).hex(),e.read(0x702630,12).hex(),str(flag),tf.hex(),td.hex(),b''.join(out[o:o+4] for o in OUT).hex()]))
pathlib.Path(__file__).with_name('golden_owner_contact.tsv').write_text('\n'.join(rows)+'\n')
print(len(rows))

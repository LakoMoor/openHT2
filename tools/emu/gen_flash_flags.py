"""Flash flag block of 0x548250 (0x5489e2..0x548a45): runs the real code from the middle of the function."""
import pathlib,random,struct
from emu import Emu,OBJ_BASE
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import *
r=random.Random(0x5489e2);e=Emu();mu=e.mu
scene=OBJ_BASE;phys=OBJ_BASE+0x10000;rec=OBJ_BASE+0x20000;SP=0x2fff0000
STOP=0x548a45
def run():
    e.write(SP,bytes(0x400))
    e.set_u32(SP+0x160,rec)
    mu.reg_write(UC_X86_REG_EBX,phys);mu.reg_write(UC_X86_REG_EBP,scene)
    mu.reg_write(UC_X86_REG_ESP,SP)
    h=mu.hook_add(UC_HOOK_CODE,lambda u,a,s,d:u.emu_stop(),begin=STOP,end=STOP)
    mu.emu_start(0x5489e2,0,count=200)
    mu.hook_del(h)
rows=[]
for case in range(400):
    flags=r.choice([0,r.randrange(1<<32),0x8000,0x10000,0x18000,0xffffffff,0x20000])
    event=r.choice([0,1,2,3,-1,0x7fffffff,r.randrange(1<<32),struct_f if False else 0x3f800000])
    rflag=r.choice([0,0x40,0xff,0xbf,r.randrange(256)])
    e.set_u32(scene+0x27b4,flags);e.set_u32(phys+0x1228,event);e.write(rec+0x108,bytes([rflag]))
    run()
    rows.append('\t'.join(map(str,[flags,event,rflag,e.read(scene+0x27b4,4)[::-1].hex() and int.from_bytes(e.read(scene+0x27b4,4),'little'),int.from_bytes(e.read(phys+0x1228,4),'little')])))
pathlib.Path(__file__).with_name('golden_flash_flags.tsv').write_text('\n'.join(rows)+'\n')
print(len(rows))

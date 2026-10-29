"""Flag rebuild block of 0x54ff80 (0x550452..0x55054a): real code run from the middle of the function."""
import pathlib,random,struct
from emu import Emu,OBJ_BASE
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import *
r=random.Random(0x550452);e=Emu();mu=e.mu
scene=OBJ_BASE;SP=0x2fff0000;STOP=0x55054a
rows=[]
offs=[0x51b8,0x51b4,0x517c,0x5178,0x4c4,0x51a0,0x51c0]
for case in range(500):
    flags=r.choice([0,0xffffffff,r.randrange(1<<32),r.randrange(1<<32)])
    vals=[r.choice([0,1,2,3,-1,r.randrange(1<<32)]) & 0xffffffff for _ in offs]
    e.set_u32(scene+0x27b4,flags)
    for o,v in zip(offs,vals):e.set_u32(scene+o,v)
    dl=r.randrange(256);e.write(scene+0x27b8,bytes([0x55]))
    mu.reg_write(UC_X86_REG_EBP,scene);mu.reg_write(UC_X86_REG_ESP,SP);mu.reg_write(UC_X86_REG_EDI,1);mu.reg_write(UC_X86_REG_ESI,0);mu.reg_write(UC_X86_REG_EDX,dl)
    h=mu.hook_add(UC_HOOK_CODE,lambda u,a,s,d:u.emu_stop(),begin=STOP,end=STOP)
    mu.emu_start(0x550452,0,count=300);mu.hook_del(h)
    rows.append('\t'.join(map(str,[flags]+vals+[dl,int.from_bytes(e.read(scene+0x27b4,4),'little'),e.read(scene+0x27b8,1)[0]])))
pathlib.Path(__file__).with_name('golden_scene_flag_bits.tsv').write_text('\n'.join(rows)+'\n')
print(len(rows))

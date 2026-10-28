"""Native particle drag block 0x59c510..0x59c61c."""
import pathlib,random,struct
from emu import Emu,OBJ_BASE,STUB
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_EIP,UC_X86_REG_ESP
r=random.Random(0x59c510);e=Emu();rows=[];clock=OBJ_BASE+0x1000;e.set_u32(OBJ_BASE+0x58,clock)
def stop(mu,a,s,u):mu.reg_write(UC_X86_REG_ESP,mu.reg_read(UC_X86_REG_ESP)+60);mu.reg_write(UC_X86_REG_EIP,STUB)
e.mu.hook_add(UC_HOOK_CODE,stop,begin=0x59c61c,end=0x59c61c)
for case in range(400):
 values=[r.uniform(-10,10) for _ in range(3)]+[r.choice([0.,3.,15.]),r.uniform(0,3)]
 raw=struct.pack('<4f',*values[:4]);e.write(OBJ_BASE+0xa0,raw);e.write(clock+0xb90,struct.pack('<d',values[4]));e.call(0x59c510,this=OBJ_BASE)
 rows.append(raw.hex()+'\t'+struct.pack('<d',values[4]).hex()+'\t'+e.read(OBJ_BASE+0xa0,16).hex())
pathlib.Path(__file__).with_name('golden_particle_drag.tsv').write_text('\n'.join(rows)+'\n');print('400 particle drag steps captured')

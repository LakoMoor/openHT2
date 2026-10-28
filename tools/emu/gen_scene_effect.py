"""First 0x551970 effect transform, real matrix calculation, effect sink replaced."""
import pathlib,random,struct
from emu import Emu,OBJ_BASE,TECH_BASE
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_EIP,UC_X86_REG_ESP
r=random.Random(0x551a47);e=Emu();cfg=OBJ_BASE+0x10000;result=[None];rows=[]
e.set_u32(0x6cec6c,cfg);e.set_u32(OBJ_BASE+0x51a4,0x102);e.set_u32(OBJ_BASE+0x27b4,0x400)
def effect(em):result[0]=e.read(em.arg(0),12)+e.read(em.arg(1),12);return 0
e.hook_ret(0x4bac90,effect)
def stop(mu,a,s,u):
 mu.reg_write(UC_X86_REG_ESP,mu.reg_read(UC_X86_REG_ESP)+8);mu.reg_write(UC_X86_REG_EIP,0x552933)
e.mu.hook_add(UC_HOOK_CODE,stop,begin=0x551b9d,end=0x551b9d)
for case in range(300):
 data=struct.pack('<24f',*[r.uniform(-3,3) for _ in range(24)]);params=struct.pack('<6f',*[r.uniform(-3,3) for _ in range(6)])
 e.write(OBJ_BASE+0x2c4,data[:48]);e.write(OBJ_BASE+0x204,data[48:]);e.write(0x697fa8,params);result[0]=None;e.call(0x551970,this=OBJ_BASE)
 assert result[0] is not None
 rows.append(data.hex()+'\t'+params.hex()+'\t'+result[0].hex())
pathlib.Path(__file__).with_name('golden_scene_effect.tsv').write_text('\n'.join(rows)+'\n');print('300 native effect transforms captured')

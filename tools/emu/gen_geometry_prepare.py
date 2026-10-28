"""Native geometry preparation through 0x551a27; controlled scene collision."""
import pathlib,random,struct
from emu import Emu,OBJ_BASE
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_EIP,UC_X86_REG_ESP
r=random.Random(0x551a27);e=Emu();cfg=OBJ_BASE+0x10000;obj=cfg+0x10000;vt=obj+0x1000;out=[0];captured=[None];answer=[0,0];rows=[]
e.set_u32(0x6cec6c,cfg);e.set_u32(cfg+0x514,0);e.set_u32(OBJ_BASE+0x51a4,0x100);e.set_u32(OBJ_BASE+0x330,obj);e.set_u32(obj,vt);e.set_u32(vt+0x34,0x541000)
def matrix(mu,a,s,u):out[0]=e.arg(0)
def query(em):
 rec=em.arg(1);assert em.ecx==obj and em.arg(0)==0
 captured[0]=e.read(rec+0x4c,24);em.set_u32(rec+0x68,answer[1]);return answer[0]
def stop(mu,a,s,u):
 mu.reg_write(UC_X86_REG_ESP,mu.reg_read(UC_X86_REG_ESP)+32);mu.reg_write(UC_X86_REG_EIP,0x552933)
e.hook_thiscall(0x541000,query,2)
e.mu.hook_add(UC_HOOK_CODE,matrix,begin=0x5aba40,end=0x5aba40)
e.mu.hook_add(UC_HOOK_CODE,stop,begin=0x551a27,end=0x551a27)
for case in range(300):
 data=struct.pack('<24f',*[r.uniform(-10,10) for _ in range(24)]);answer[:]=[case%8,r.getrandbits(32)]
 e.write(OBJ_BASE+0x2c4,data[:48]);e.write(OBJ_BASE+0x204,data[48:]);e.call(0x551970,this=OBJ_BASE)
 rows.append(data.hex()+'\t'+captured[0].hex()+'\t'+e.read(out[0],48).hex()+'\t'+','.join(map(str,answer)))
pathlib.Path(__file__).with_name('golden_geometry_prepare.tsv').write_text('\n'.join(rows)+'\n');print('300 native geometry preparations captured')

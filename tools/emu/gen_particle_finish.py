"""Root constructor tail after attachment, virtual registration observed."""
import pathlib,random
from emu import Emu,OBJ_BASE,STUB
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_EDI,UC_X86_REG_EBX,UC_X86_REG_EBP,UC_X86_REG_ESP,UC_X86_REG_EIP
r=random.Random(0x59a929);e=Emu();rows=[];state={};manager=OBJ_BASE+0x1000;vt=manager+0x1000
e.set_u32(manager,vt);e.set_u32(vt+0x14,0x541000)
def registration(em):state['trace']=[em.ecx,em.arg(0),em.arg(1)];return 0
e.hook_thiscall(0x541000,registration,2)
def start(mu,a,s,u):
 mu.reg_write(UC_X86_REG_EDI,OBJ_BASE);mu.reg_write(UC_X86_REG_EBX,OBJ_BASE+0x14);mu.reg_write(UC_X86_REG_EBP,0);e.set_u32(mu.reg_read(UC_X86_REG_ESP)+0x44,state['height'])
def stop(mu,a,s,u):mu.reg_write(UC_X86_REG_EIP,STUB)
e.mu.hook_add(UC_HOOK_CODE,start,begin=0x59a929,end=0x59a929);e.mu.hook_add(UC_HOOK_CODE,stop,begin=0x59a95a,end=0x59a95a)
for case in range(300):
 raw=bytes(r.getrandbits(8) for _ in range(0x84));state['height']=r.getrandbits(32);e.write(OBJ_BASE,raw);e.set_u32(OBJ_BASE+0x5c,manager);e.call(0x59a929)
 rows.append(raw.hex()+'\t'+str(state['height'])+'\t'+','.join(map(str,state['trace']))+'\t'+e.read(OBJ_BASE,0x84).hex())
pathlib.Path(__file__).with_name('golden_particle_finish.tsv').write_text('\n'.join(rows)+'\n');print('300 particle registrations captured')

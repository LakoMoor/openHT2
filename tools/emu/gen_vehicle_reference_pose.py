"""Native reference-point block, only world lookup replaced."""
import pathlib,random,struct
from emu import Emu,OBJ_BASE,STUB
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_EBX,UC_X86_REG_ESP,UC_X86_REG_EIP
r=random.Random(0x4cf746);e=Emu();v=OBJ_BASE;pose=v+0x10000;out=b''
def world(q):
 dst=q.arg(0);e.write(dst,e.read(pose,48));return dst
e.hook_thiscall(0x57de90,world,1)
def stop(mu,a,size,u):
 global out
 sp=mu.reg_read(UC_X86_REG_ESP);out=e.read(sp+0x30,48);mu.reg_write(UC_X86_REG_EIP,STUB)
e.mu.hook_add(UC_HOOK_CODE,stop,begin=0x4cf7e8,end=0x4cf7e8);rows=[]
for case in range(400):
 values=[r.uniform(-1000,1000) for _ in range(15)];raw=struct.pack('<15f',*values)
 e.write(pose,raw[:48]);e.write(v+0x5230,raw[48:]);e.mu.reg_write(UC_X86_REG_EBX,v);e.call(0x4cf746)
 rows.append(raw.hex()+'\t'+out.hex())
pathlib.Path(__file__).with_name('golden_vehicle_reference_pose.tsv').write_text('\n'.join(rows)+'\n');print('400 native reference poses captured')

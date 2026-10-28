"""Native police radar relative-distance block 0x4d13eb..0x4d1492."""
import pathlib,random,struct,math
from emu import Emu,OBJ_BASE,STUB
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_EBP,UC_X86_REG_EBX,UC_X86_REG_ESP,UC_X86_REG_EIP
E=Emu();v=OBJ_BASE;human=v+0x10000;police=v+0x20000;mats={}
def world(q):q.write(q.arg(0),mats[q.ecx]);return q.arg(0)
E.hook_thiscall(0x57de90,world,1)
def start(mu,a,s,u):
 mu.reg_write(UC_X86_REG_EBP,police+0x10);mu.reg_write(UC_X86_REG_EBX,police);E.set_u32(mu.reg_read(UC_X86_REG_ESP)+0x34,human)
def stop(mu,a,s,u):mu.reg_write(UC_X86_REG_EIP,STUB)
E.mu.hook_add(UC_HOOK_CODE,start,begin=0x4d13eb,end=0x4d13eb);E.mu.hook_add(UC_HOOK_CODE,stop,begin=0x4d1492,end=0x4d1492)
r=random.Random(0x4d13eb);rows=[]
for i in range(300):
 a=r.uniform(-3,3);c=math.cos(a);s=math.sin(a);h=struct.pack('<12f',c,s,0,-s,c,0,0,0,1,*[r.uniform(-200,200) for _ in range(3)]);p=struct.pack('<12f',1,0,0,0,1,0,0,0,1,*[r.uniform(-200,200) for _ in range(3)])
 mats[human+0x10]=h;mats[police+0x10]=p;E.call(0x4d13eb);sp=E.mu.reg_read(UC_X86_REG_ESP);out=int.from_bytes(E.read(sp+0x50,8),'little');rows.append('\t'.join([h.hex(),p.hex(),str(out)]))
pathlib.Path(__file__).with_name('golden_police_relative_distance.tsv').write_text('\n'.join(rows)+'\n');print('300 native police radar distances captured')

"""Native trajectory synchronization block with AI-world service boundaries hooked."""
import pathlib,random,struct
from emu import Emu,OBJ_BASE,STUB
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_EBX,UC_X86_REG_EDI,UC_X86_REG_EIP,UC_X86_REG_ESP
r=random.Random(0x4cf861);e=Emu();v=OBJ_BASE;driver=v+0x10000;player=driver+0x10000;flags=player+0x1000;clock=flags+0x1000;trace=[];snapshot=b'';pose=b'';direction=b'';width=0.;lane=0
f=lambda *x:struct.pack('<'+'f'*len(x),*x)
e.write(0x7025f0,f(1,0,0,0,1,0,0,0,1,0,0,0));e.write(0x702640,f(1,0,0));e.write(0x702650,f(0,0,0));e.set_u32(0x696ca0,clock)
def lookup(q):
 trace.append('place');dst=q.arg(0);e.write(dst,snapshot);return dst
def segment(q):
 trace.append('width');dst=q.arg(8);e.write(dst+8,struct.pack('<d',width*2));return dst
def get_lane(q):
 trace.append('lane');return lane
def road(q):
 trace.append('pose');dst=q.arg(0);e.write(dst,pose);return dst
def tangent(q):
 trace.append('direction');dst=q.arg(0);e.write(dst,direction);return dst
def publish(q):
 trace.append('publish:'+str(q.arg(1)));return 0
e.hook_ret(0x437520,lookup);e.hook_ret(0x438d70,segment);e.hook_thiscall(0x4d4620,get_lane,2);e.hook_ret(0x43a4a0,road);e.hook_ret(0x439320,tangent);e.hook_ret(0x438c90,publish)
def enter(mu,a,size,u):
 sp=mu.reg_read(UC_X86_REG_ESP);e.set_u32(sp+0xc,v+0x509c);e.set_u32(sp+0x1c,driver);mu.reg_write(UC_X86_REG_EBX,v);mu.reg_write(UC_X86_REG_EDI,flags)
def stop(mu,a,size,u):mu.reg_write(UC_X86_REG_EIP,STUB)
e.mu.hook_add(UC_HOOK_CODE,enter,begin=0x4cf861,end=0x4cf861)
for address in [0x4cfb05,0x4cfb0f]:e.mu.hook_add(UC_HOOK_CODE,stop,begin=address,end=address)
rows=[]
for case in range(300):
 active=case%5!=0;lane=r.randrange(-2,5);width=r.uniform(1,5);snapshot=r.randbytes(32);angle=r.uniform(-3,3);import math
 c,s=math.cos(angle),math.sin(angle);pose=f(c,s,0,-s,c,0,0,0,1,*[r.uniform(-30,30) for _ in range(3)]);direction=f(*[r.uniform(-10,10) for _ in range(3)])
 old=f(*[r.uniform(-30,30) for _ in range(3)]);dt=r.choice([0.,0.016,0.1,1.]);initial=r.randbytes(96)
 e.write(v+0x50a0,initial);e.write(v+0x50c4,old);e.set_u32(v+0x509c,player);e.set_u32(flags,0 if active else 1);e.write(clock+0xb90,struct.pack('<d',dt));e.write(driver+0x40a8,struct.pack('<d',width));trace.clear();e.call(0x4cf861)
 rows.append('\t'.join([str(int(active)),str(lane),struct.pack('<d',width).hex(),struct.pack('<d',dt).hex(),snapshot.hex(),pose.hex(),direction.hex(),old.hex(),initial.hex(),e.read(v+0x50a0,96).hex(),';'.join(trace)]))
pathlib.Path(__file__).with_name('golden_trajectory_sync_block.tsv').write_text('\n'.join(rows)+'\n');print('300 native trajectory synchronization blocks captured')

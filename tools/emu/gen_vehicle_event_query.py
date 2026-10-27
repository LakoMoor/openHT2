"""Original vehicle query block: native root traversal, boundary world/query."""
import pathlib,random,struct
from emu import Emu,OBJ_BASE,STUB
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_EBX,UC_X86_REG_EAX,UC_X86_REG_ESP,UC_X86_REG_EIP
r=random.Random(0x4cfb0f);e=Emu();v=OBJ_BASE;mat=v+0x1000;captured=[]
def world(q):
 dst=q.arg(0);e.write(dst,e.read(mat,48));return dst
e.hook_thiscall(0x57de90,world,1)
def query(q):
 captured.extend([q.arg(0),q.arg(1),q.arg(2),q.arg(3),q.arg(4),q.arg(5)]);return 0
e.hook_thiscall(0x5dbec0,query,6)
def stop(mu,a,size,user):
 sp=mu.reg_read(UC_X86_REG_ESP);ret=int.from_bytes(mu.mem_read(sp,4),'little');mu.reg_write(UC_X86_REG_ESP,sp+4);mu.reg_write(UC_X86_REG_EIP,ret)
e.mu.hook_add(UC_HOOK_CODE,stop,begin=0x4cfb51,end=0x4cfb51)
rows=[]
for case in range(300):
 depth=case%7;e.write(v+0x110,b'\0'*64)
 for i in range(depth):
  n=v+0x110 if i==0 else v+0x2000+i*128
  parent=v+0x2000+(i+1)*128;e.set_u32(n+0x2c,parent);e.set_u32(parent+0x2c,0)
 values=[r.uniform(-100,100) for _ in range(12)];raw=struct.pack('<12f',*values);e.write(mat,raw);captured.clear();e.mu.reg_write(UC_X86_REG_EBX,v)
 e.call(0x4cfb0f)
 rows.append(str(depth)+'\t'+raw.hex()+'\t'+','.join(map(str,captured)))
pathlib.Path(__file__).with_name('golden_vehicle_event_query.tsv').write_text('\n'.join(rows)+'\n')
print('300 native vehicle query blocks captured')

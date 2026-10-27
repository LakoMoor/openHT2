"""Native x87 lane smoothing and translation preparation, no replacement hooks."""
import pathlib,random,struct
from emu import Emu,OBJ_BASE,STUB
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_EAX,UC_X86_REG_EBX,UC_X86_REG_ESP,UC_X86_REG_EIP
r=random.Random(0x4cf96d);e=Emu();vehicle=OBJ_BASE;matrix=vehicle+0x10000;driver=matrix+0x1000;clock=driver+0x10000;result=b'';lane=0
identity=[1.,0.,0.,0.,1.,0.,0.,0.,1.,0.,0.,0.]
e.write(0x7025f0,struct.pack('<12f',*identity));e.write(0x702640,struct.pack('<3f',1.,0.,0.));e.set_u32(0x696ca0,clock)
def enter(mu,a,size,u):
 sp=mu.reg_read(UC_X86_REG_ESP);e.set_u32(sp+0x1c,driver);e.set_u32(sp+0x20,lane);mu.reg_write(UC_X86_REG_ESP,sp-0x30);mu.reg_write(UC_X86_REG_EAX,matrix);mu.reg_write(UC_X86_REG_EBX,vehicle)
def stop(mu,a,size,u):
 global result
 result=e.read(mu.reg_read(UC_X86_REG_EAX)+36,12);mu.reg_write(UC_X86_REG_EIP,STUB)
e.mu.hook_add(UC_HOOK_CODE,enter,begin=0x4cf96d,end=0x4cf96d);e.mu.hook_add(UC_HOOK_CODE,stop,begin=0x4cfa3a,end=0x4cfa3a)
rows=[]
for case in range(600):
 vals=[r.uniform(-10,10) for _ in range(15)];data=struct.pack('<15f',*vals);dt=r.choice([0.,1/60,0.1,0.25,1.]);width=r.uniform(1.,5.);lane=r.randrange(-3,6)
 e.write(matrix,data[:48]);e.write(vehicle+0x50c4,data[48:]);e.write(clock+0xb90,struct.pack('<d',dt));e.write(driver+0x40a8,struct.pack('<d',width));e.call(0x4cf96d)
 rows.append('\t'.join([data.hex(),struct.pack('<d',dt).hex(),struct.pack('<d',width).hex(),str(lane),result.hex()]))
pathlib.Path(__file__).with_name('golden_trajectory_lateral.tsv').write_text('\n'.join(rows)+'\n');print('600 native trajectory smoothing results captured')

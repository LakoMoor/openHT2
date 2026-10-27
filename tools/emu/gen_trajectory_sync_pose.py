"""Native inverse, smoothing, composition and stored scene trajectory pose."""
import pathlib,random,struct,math
from emu import Emu,OBJ_BASE,STUB
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_EAX,UC_X86_REG_EBX,UC_X86_REG_ESP,UC_X86_REG_EIP
r=random.Random(0x4cfa4b);e=Emu();vehicle=OBJ_BASE;pose=vehicle+0x10000;inverse=pose+0x1000;driver=inverse+0x1000;clock=driver+0x10000;lane=0
identity=[1.,0.,0.,0.,1.,0.,0.,0.,1.,0.,0.,0.];e.write(0x7025f0,struct.pack('<12f',*identity));e.write(0x702640,struct.pack('<3f',1.,0.,0.));e.set_u32(0x696ca0,clock)
def enter(mu,a,size,u):
 sp=mu.reg_read(UC_X86_REG_ESP);e.set_u32(sp+0x1c,driver);e.set_u32(sp+0x20,lane);e.write(sp+0xc8,e.read(pose,48));mu.reg_write(UC_X86_REG_ESP,sp-0x30);mu.reg_write(UC_X86_REG_EAX,inverse);mu.reg_write(UC_X86_REG_EBX,vehicle)
def stop(mu,a,size,u):mu.reg_write(UC_X86_REG_EIP,STUB)
e.mu.hook_add(UC_HOOK_CODE,enter,begin=0x4cf96d,end=0x4cf96d);e.mu.hook_add(UC_HOOK_CODE,stop,begin=0x4cfa62,end=0x4cfa62)
rows=[]
for case in range(400):
 angle=r.uniform(-3.14,3.14);c,s=math.cos(angle),math.sin(angle);vals=[c,s,0.,-s,c,0.,0.,0.,1.]+[r.uniform(-100,100) for _ in range(6)];raw=struct.pack('<15f',*vals)
 dt=r.choice([0.,1/60,0.1,0.25,1.]);width=r.uniform(1.,5.);lane=r.randrange(-3,6)
 e.write(pose,raw[:48]);e.write(vehicle+0x50c4,raw[48:]);e.write(clock+0xb90,struct.pack('<d',dt));e.write(driver+0x40a8,struct.pack('<d',width));e.call(0x5ac2e0,[inverse,pose],'ii');e.call(0x4cf96d)
 rows.append('\t'.join([raw.hex(),struct.pack('<d',dt).hex(),struct.pack('<d',width).hex(),str(lane),e.read(vehicle+0x50a0,48).hex()]))
pathlib.Path(__file__).with_name('golden_trajectory_sync_pose.tsv').write_text('\n'.join(rows)+'\n');print('400 native smoothed trajectory poses captured')

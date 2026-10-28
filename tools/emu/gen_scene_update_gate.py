"""0x551970 entry decision; stops before geometry work."""
import pathlib,random,struct
from emu import Emu,OBJ_BASE
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_EIP
r=random.Random(0x551970);e=Emu();cfg=OBJ_BASE+0x10000;clock=cfg+0x10000;hit=[False];rows=[]
e.set_u32(0x6cec6c,cfg);e.set_u32(0x6ef1fc,clock)
def reached(mu,a,s,u):hit[0]=True;mu.reg_write(UC_X86_REG_EIP,0x552933)
e.mu.hook_add(UC_HOOK_CODE,reached,begin=0x5519f5,end=0x5519f5)
for case in range(400):
 flags=r.getrandbits(32);mode=r.randrange(-2,5);kind=r.randrange(5);dt=r.choice([0.,0.01,-0.02,0.1,0.5,1.,2.]);frame=r.randrange(-1000,1000)
 for a,v in [(OBJ_BASE+0x51a4,flags),(cfg+0x514,mode),(cfg+0x524,kind),(clock+0xbb0,frame)]:e.set_u32(a,v&0xffffffff)
 e.write(clock+0x1f18,struct.pack('<d',dt));hit[0]=False;e.call(0x551970,this=OBJ_BASE)
 rows.append(','.join(map(str,[flags,mode,kind,dt,frame]))+'\t'+str(int(hit[0])))
pathlib.Path(__file__).with_name('golden_scene_update_gate.tsv').write_text('\n'.join(rows)+'\n');print('400 update gates captured')

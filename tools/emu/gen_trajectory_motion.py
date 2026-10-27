"""Native direction normalization plus original trajectory velocity write block."""
import pathlib,random,struct
from emu import Emu,OBJ_BASE,STUB
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_EBX,UC_X86_REG_EAX,UC_X86_REG_EIP
r=random.Random(0x4cfa8d);e=Emu();v=OBJ_BASE;src=v+0x10000;unit=src+32
# Stop after the angular vector stores, before lane publication.
def stop(mu,a,size,u):mu.reg_write(UC_X86_REG_EIP,STUB)
e.mu.hook_add(UC_HOOK_CODE,stop,begin=0x4cfaf3,end=0x4cfaf3);rows=[]
for case in range(400):
 vector=[r.uniform(-100,100) for _ in range(3)]
 if case%5==0:vector=r.choice([[0,0,0],[1e-6,0,0],[1e-5,0,0],[0,-1,0]])
 if case%11==0:vector=[float('nan'),1.,2.]
 raw=struct.pack('<3f',*vector);zero=struct.pack('<3f',0.,-0.,0.)
 e.write(src,raw);e.write(0x702650,zero);e.call(0x5ab400,[unit,src],'ii')
 e.mu.reg_write(UC_X86_REG_EBX,v);e.mu.reg_write(UC_X86_REG_EAX,unit);e.call(0x4cfa8d)
 rows.append(raw.hex()+'\t'+zero.hex()+'\t'+e.read(v+0x50d0,12).hex()+'\t'+e.read(v+0x50e8,12).hex())
pathlib.Path(__file__).with_name('golden_trajectory_motion.tsv').write_text('\n'.join(rows)+'\n');print('400 native trajectory motion writes captured')

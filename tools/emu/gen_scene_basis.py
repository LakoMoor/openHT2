"""Execute 0x551970 through its native matrix product, before room lookup."""
import pathlib,random,struct
from emu import Emu,OBJ_BASE
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_EIP,UC_X86_REG_ESP
r=random.Random(0x551a0f);e=Emu();cfg=OBJ_BASE+0x10000;out=[0];result=[None];rows=[]
e.set_u32(0x6cec6c,cfg);e.set_u32(cfg+0x514,0);e.set_u32(OBJ_BASE+0x51a4,0x100)
def capture(mu,a,s,u):out[0]=e.arg(0)
def stop(mu,a,s,u):
 result[0]=e.read(out[0],48);mu.reg_write(UC_X86_REG_ESP,mu.reg_read(UC_X86_REG_ESP)+20);mu.reg_write(UC_X86_REG_EIP,0x552933)
e.mu.hook_add(UC_HOOK_CODE,capture,begin=0x5aba40,end=0x5aba40)
e.mu.hook_add(UC_HOOK_CODE,stop,begin=0x551a0f,end=0x551a0f)
for case in range(300):
 data=struct.pack('<24f',*[r.uniform(-10,10) for _ in range(24)])
 e.write(OBJ_BASE+0x2c4,data[:48]);e.write(OBJ_BASE+0x204,data[48:]);e.call(0x551970,this=OBJ_BASE)
 rows.append(data.hex()+'\t'+result[0].hex())
pathlib.Path(__file__).with_name('golden_scene_basis.tsv').write_text('\n'.join(rows)+'\n');print('300 scene basis matrices captured')

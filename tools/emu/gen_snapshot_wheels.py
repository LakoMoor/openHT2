"""Header and wheel decoding through 0x541bfe; synthetic snapshots only."""
import pathlib,random
from emu import Emu,OBJ_BASE,STUB
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_ESI,UC_X86_REG_EIP,UC_X86_REG_ESP
r=random.Random(0x541bfe);e=Emu();rows=[]
e.mu.hook_add(UC_HOOK_CODE,lambda mu,a,s,u:mu.reg_write(UC_X86_REG_ESI,OBJ_BASE),begin=0x541adf,end=0x541adf)
def stop(mu,a,s,u):
 mu.reg_write(UC_X86_REG_ESP,mu.reg_read(UC_X86_REG_ESP)+4);mu.reg_write(UC_X86_REG_EIP,STUB)
e.mu.hook_add(UC_HOOK_CODE,stop,begin=0x541bfe,end=0x541bfe)
for case in range(360):
 data=bytearray(r.getrandbits(8) for _ in range(0x150));data[0x10:0x14]=(case%9).to_bytes(4,'little')
 e.write(OBJ_BASE,data);e.call(0x541adf)
 rows.append(data.hex()+'\t'+e.read(OBJ_BASE+0x50,0x72).hex())
pathlib.Path(__file__).with_name('golden_snapshot_wheels.tsv').write_text('\n'.join(rows)+'\n')
print('360 native header and wheel snapshots captured')

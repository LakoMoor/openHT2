"""Decode header block 0x541adf..0x541b6a; body decoding excluded."""
import pathlib,random,struct
from emu import Emu,OBJ_BASE,STUB
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_ESI,UC_X86_REG_EIP,UC_X86_REG_ESP
r=random.Random(0x541adf);e=Emu();rows=[]
e.mu.hook_add(UC_HOOK_CODE,lambda mu,a,s,u:mu.reg_write(UC_X86_REG_ESI,OBJ_BASE),begin=0x541adf,end=0x541adf)
def stop(mu,a,s,u):
 mu.reg_write(UC_X86_REG_ESP,mu.reg_read(UC_X86_REG_ESP)+4);mu.reg_write(UC_X86_REG_EIP,STUB)
e.mu.hook_add(UC_HOOK_CODE,stop,begin=0x541b6a,end=0x541b6a)
for packed in range(256):
 flags=r.getrandbits(32);old=r.getrandbits(32)
 e.set_u32(OBJ_BASE+0x18,flags);e.set_u32(OBJ_BASE+0x1c,packed);e.set_u32(OBJ_BASE+0x5c,old);e.call(0x541adf)
 out=[int.from_bytes(e.read(OBJ_BASE+o,n),'little') for o,n in [(0x50,4),(0x54,1),(0x58,4),(0x5c,4)]]
 rows.append(','.join(map(str,[flags,packed,old]))+'\t'+','.join(map(str,out)))
pathlib.Path(__file__).with_name('golden_snapshot_header.tsv').write_text('\n'.join(rows)+'\n')
print('256 snapshot headers captured')

"""0x59a780 initial fields; base identities replaced, stop before allocation."""
import pathlib,random
from emu import Emu,OBJ_BASE
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_EIP,UC_X86_REG_ESP
r=random.Random(0x59a780);e=Emu();rows=[]
e.hook_thiscall(0x579310,lambda em:em.ecx,0);e.hook_thiscall(0x5fa6f0,lambda em:em.ecx,1)
def stop(mu,a,s,u):
 mu.reg_write(UC_X86_REG_ESP,mu.reg_read(UC_X86_REG_ESP)+4);mu.reg_write(UC_X86_REG_EIP,0x59a98e)
# Stop emulator directly: no need to execute the uninitialised destructor epilogue.
def capture(mu,a,s,u):mu.emu_stop()
e.mu.hook_add(UC_HOOK_CODE,capture,begin=0x59a80d,end=0x59a80d)
for case in range(300):
 args=[r.getrandbits(32) for _ in range(11)];e.write(OBJ_BASE,bytes(0x84))
 try:e.call(0x59a780,args,'i'*11,this=OBJ_BASE)
 except RuntimeError:pass
 rows.append(','.join(map(str,args))+'\t'+e.read(OBJ_BASE,0x84).hex())
pathlib.Path(__file__).with_name('golden_particle_root_fields.tsv').write_text('\n'.join(rows)+'\n');print('300 root field initializations captured')

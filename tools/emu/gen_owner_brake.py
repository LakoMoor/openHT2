"""Native owner-flag brake branch with both exits captured before other work."""
import pathlib,random,struct
from emu import Emu,OBJ_BASE,STUB
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_EAX,UC_X86_REG_EBP,UC_X86_REG_EBX,UC_X86_REG_EIP
r=random.Random(0x4d67db);e=Emu();driver=OBJ_BASE;which=0

def stop(mu,address,size,user):
 global which
 which=1 if address==0x4d67f9 else 0;mu.reg_write(UC_X86_REG_EIP,STUB)
e.mu.hook_add(UC_HOOK_CODE,lambda mu,a,size,u:mu.reg_write(UC_X86_REG_EBP,driver),begin=0x4d67db,end=0x4d67db)
for p in [0x4d67f9,0x4d68c5]:e.mu.hook_add(UC_HOOK_CODE,stop,begin=p,end=p)
rows=[]
for case in range(400):
 flags=r.getrandbits(32);before=r.randbytes(20);e.write(driver+0x270c,before)
 e.mu.reg_write(UC_X86_REG_EAX,flags);e.mu.reg_write(UC_X86_REG_EBP,driver);e.mu.reg_write(UC_X86_REG_EBX,0);e.call(0x4d67db)
 rows.append(str(flags)+'\t'+before.hex()+'\t'+str(which)+'\t'+e.read(driver+0x270c,20).hex())
pathlib.Path(__file__).with_name('golden_owner_brake.tsv').write_text('\n'.join(rows)+'\n');print('400 native brake-override branches captured')

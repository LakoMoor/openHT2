"""Manager cleanup phase 0x57975d..0x5797be; destructors observed."""
import pathlib,random
from emu import Emu,OBJ_BASE,STUB
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_EIP
r=random.Random(0x57975d);e=Emu();rows=[];trace=[];pool=OBJ_BASE+0x1000;vt=pool+0x1000
def destroy(em):assert em.arg(0)==1;trace.append(em.ecx);return 0
e.hook_thiscall(0x541000,destroy,1);e.set_u32(vt,0x541000)
e.mu.hook_add(UC_HOOK_CODE,lambda mu,a,s,u:mu.reg_write(UC_X86_REG_EIP,STUB),begin=0x5797be,end=0x5797be)
for case in range(300):
 count=case%10;marks=[r.getrandbits(32) if r.randrange(2) else 0 for _ in range(count)];e.set_u32(OBJ_BASE+0x1b0,1);e.set_u32(OBJ_BASE+0x20,pool);e.set_u32(pool+8,pool+0x100 if count else 0)
 for i,mark in enumerate(marks):
  obj=pool+0x100+i*32;e.set_u32(obj,vt);e.set_u32(obj+4,mark);e.set_u32(obj+8,obj+32 if i+1<count else 0)
 trace.clear();e.call(0x57975d,this=OBJ_BASE)
 rows.append(','.join(map(str,marks))+'\t'+','.join(map(str,trace)))
pathlib.Path(__file__).with_name('golden_effect_cleanup.tsv').write_text('\n'.join(rows)+'\n');print('300 cleanup traversals captured')

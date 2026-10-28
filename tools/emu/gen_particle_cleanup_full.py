"""Manager cleanup with real particle deleting destructors and pool recycle."""
import pathlib,random,struct
from emu import Emu,OBJ_BASE,STUB
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_EIP
r=random.Random(0x57975d);e=Emu();rows=[];pool=OBJ_BASE+0x1000
e.hook_ret(0x63266c,lambda em:0);e.mu.hook_add(UC_HOOK_CODE,lambda mu,a,s,u:mu.reg_write(UC_X86_REG_EIP,STUB),begin=0x5797be,end=0x5797be)
for case in range(200):
 raw=bytearray(0x2000);count=case%8
 def put(a,v):struct.pack_into('<I',raw,a-OBJ_BASE,v)
 put(OBJ_BASE+0x1b0,1);put(OBJ_BASE+0x20,pool);put(pool+8,pool+0x100 if count else 0)
 for i in range(count):
  obj=pool+0x100+i*0x100
  for offset,value in [(0,0x650dd4),(4,r.randrange(2)),(8,obj+0x100 if i+1<count else 0),(12,obj-0x100 if i else 0),(16,pool)]:put(obj+offset,value)
 e.write(OBJ_BASE,raw);e.call(0x57975d,this=OBJ_BASE)
 rows.append(raw.hex()+'\t'+e.read(OBJ_BASE,0x2000).hex())
pathlib.Path(__file__).with_name('golden_particle_cleanup_full.tsv').write_text('\n'.join(rows)+'\n');print('200 full particle cleanup passes captured')

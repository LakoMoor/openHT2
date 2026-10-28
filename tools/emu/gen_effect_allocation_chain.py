"""Native pool recycle 0x579480 on valid intrusive lists."""
import pathlib,random,struct
from emu import Emu,OBJ_BASE
r=random.Random(0x579480);e=Emu();rows=[]
for case in range(300):
 raw=bytearray(0x200);active=[OBJ_BASE+0x40+i*32 for i in range(case%5)];free=[OBJ_BASE+0x100+i*32 for i in range(case%4)]
 def put(offset,value):struct.pack_into('<I',raw,offset,value)
 put(4,free[0] if free else 0);put(8,active[0] if active else 0)
 for group in [active,free]:
  for i,p in enumerate(group):put(p-OBJ_BASE+8,group[i+1] if i+1<len(group) else 0);put(p-OBJ_BASE+12,group[i-1] if i else 0)
 e.write(OBJ_BASE,raw);e.call(0x579380,[0xb0,OBJ_BASE],'ii')
 from unicorn.x86_const import UC_X86_REG_EAX
 result=e.mu.reg_read(UC_X86_REG_EAX)
 if result:e.call(0x579310,this=result)
 rows.append(raw.hex()+'\t'+str(result)+'\t'+e.read(OBJ_BASE,0x200).hex())
pathlib.Path(__file__).with_name('golden_effect_allocation_chain.tsv').write_text('\n'.join(rows)+'\n');print('300 pool allocation constructor chains captured')

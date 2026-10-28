"""Full manager 0x5796e0, controlled virtual updates and destructors."""
import pathlib
from emu import Emu,OBJ_BASE
rows=[];e=Emu();trace=[];pool=OBJ_BASE+0x1000;vt=pool+0x1000;state={}
def update(em):
 trace.append(em.ecx)
 if state['insert']:
  state['insert']=False;new=pool+0x800;em.set_u32(new,vt);em.set_u32(new+8,int.from_bytes(em.read(pool+8,4),'little'));em.set_u32(pool+8,new)
 return 0
e.hook_thiscall(0x541000,update,0);e.set_u32(vt+4,0x541000)
for case in range(200):
 count=case%8;insert=bool(case%2 and count);e.set_u32(OBJ_BASE+0x1b0,1);e.set_u32(OBJ_BASE+0x20,pool);e.set_u32(pool+8,pool+0x100 if count else 0)
 for i in range(count):
  obj=pool+0x100+i*32;e.set_u32(obj,vt);e.set_u32(obj+4,0);e.set_u32(obj+8,obj+32 if i+1<count else 0)
 e.set_u32(pool+0x800+4,0);state['insert']=insert;trace.clear();e.call(0x5796e0,this=OBJ_BASE)
 rows.append(str(count)+','+str(int(insert))+'\t'+','.join(map(str,trace)))
pathlib.Path(__file__).with_name('golden_effect_manager_tick.tsv').write_text('\n'.join(rows)+'\n');print('200 manager ticks captured')

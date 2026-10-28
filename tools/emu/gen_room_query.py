"""Native 0x4e5ba7 with controlled virtual scene query."""
import pathlib,random
from emu import Emu,OBJ_BASE
r=random.Random(0x4e5ba7);e=Emu();obj=OBJ_BASE;vt=obj+0x1000;point=obj+0x2000;answer=[0,0];rows=[]
e.set_u32(obj,vt);e.set_u32(vt+0x34,0x541000)
def query(em):
 assert em.ecx==obj and em.arg(0)==0
 em.set_u32(em.arg(1)+0x68,answer[1]);return answer[0]
e.hook_thiscall(0x541000,query,2)
for case in range(300):
 answer[:]=[case%8,r.getrandbits(32)];owner=0 if case%11==0 else obj
 result=e.call(0x4e5ba7,[owner,point,point+12],'iii')
 # Emu.call returns ST0; this routine returns EAX.
 from unicorn.x86_const import UC_X86_REG_EAX
 result=e.mu.reg_read(UC_X86_REG_EAX)
 rows.append(','.join(map(str,[owner,*answer]))+'\t'+str(result))
pathlib.Path(__file__).with_name('golden_room_query.tsv').write_text('\n'.join(rows)+'\n');print('300 native room queries captured')

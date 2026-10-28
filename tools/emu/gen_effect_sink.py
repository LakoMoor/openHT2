"""Native effect sink wrapper, allocator/constructor replaced."""
import pathlib,random
from emu import Emu,OBJ_BASE
r=random.Random(0x4bac90);e=Emu();rows=[];state={}
def alloc(em):
 state['allocation']=[em.arg(i) for i in range(3)];return state['result']
def ctor(em):state['ctor']=[em.ecx]+[em.arg(i) for i in range(6)];return em.ecx
e.hook_ret(0x579380,alloc);e.hook_thiscall(0x59c420,ctor,6)
for case in range(300):
 values=[r.getrandbits(32) for _ in range(7)];pool,texture,count,point,direction,kind,enabled=values
 e.set_u32(0x6f6a8c,pool);e.set_u32(0x696224,texture);e.set_u32(0x696208,count)
 state.clear();state['result']=OBJ_BASE if case%3 else 0;e.call(0x4bac90,[point,direction,kind,enabled],'iiii')
 rows.append(','.join(map(str,values+[state['result']]))+'\t'+','.join(map(str,state['allocation']))+'\t'+','.join(map(str,state.get('ctor',[]))))
pathlib.Path(__file__).with_name('golden_effect_sink.tsv').write_text('\n'.join(rows)+'\n');print('300 effect wrappers captured')

"""Native 0x417530 competitor-list removal, with flag refresh hooked."""
import pathlib,random
from emu import Emu,OBJ_BASE
E=Emu();p=OBJ_BASE;arr=p+0x10000;trace=[]
E.hook_ret(0x477210,lambda q:trace.append('refresh') or 0)
r=random.Random(0x417530);rows=[]
for i in range(250):
 items=[r.choice([p,p,p+0x1000,p+0x2000]) for _ in range(r.randrange(9))];flags=r.getrandbits(32);tracked=r.randrange(256)
 E.set_u32(0x68a980,arr);E.set_u32(0x68a984,arr+4*len(items));E.set_u32(p+0x60,flags);E.mu.mem_write(p+0x1f0,bytes([tracked]));E.set_u32(p+0x20c,123);E.set_u32(p+0x210,456);E.set_u32(p+0x214,789);E.mu.mem_write(p+0x368,bytes([7]))
 for k,x in enumerate(items):E.set_u32(arr+k*4,x)
 trace.clear();E.call(0x417530,[p],'i')
 n=(int.from_bytes(E.read(0x68a984,4),'little')-arr)//4;out=[int.from_bytes(E.read(arr+k*4,4),'little') for k in range(n)]
 state=[E.read(p+0x1f0,1)[0]]+[int.from_bytes(E.read(p+o,4),'little') for o in [0x20c,0x210,0x214,0x60]]+[E.read(p+0x368,1)[0]]
 rows.append('\t'.join([','.join(map(str,items)),str(flags),str(tracked),','.join(map(str,out)),','.join(map(str,state)),';'.join(trace)]))
pathlib.Path(__file__).with_name('golden_remove_competitor.tsv').write_text('\n'.join(rows)+'\n');print('250 native competitor removals captured')

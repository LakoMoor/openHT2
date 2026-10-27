"""Native player-reference record assignment with allocation boundaries."""
import pathlib,random
from emu import Emu,OBJ_BASE
E=Emu();dst=OBJ_BASE;src=dst+0x100;db=dst+0x1000;sb=dst+0x2000;new=dst+0x3000;trace=[]
def alloc(q):
 n=q.arg(0);a=q.arg(1);b=q.arg(2);trace.append('alloc:'+str(n));q.write(new,q.read(a,b-a));return new
E.hook_thiscall(0x458cf0,alloc,3)
def free(q):trace.append('free:'+str(q.arg(1)));return 0
E.hook_ret(0x44b4a0,free)
r=random.Random(0x42bfe0);rows=[]
for case in range(200):
 cap=r.randrange(0,17);length=r.randrange(cap+1);size=r.randrange(0,17);same=case%19==0
 E.set_u32(dst,111);E.set_u32(dst+4,db);E.set_u32(dst+8,db+length);E.set_u32(dst+12,db+cap)
 E.set_u32(src,222);E.set_u32(src+4,sb);E.set_u32(src+8,sb+size);E.set_u32(src+12,sb+size)
 E.write(db,bytes(range(32)));E.write(sb,bytes(range(64,96)));E.write(new,bytes(32));trace.clear()
 E.call(0x42bfe0,[dst if same else src],'i',this=dst)
 state=[int.from_bytes(E.read(dst+j*4,4),'little') for j in range(4)]
 rows.append('\t'.join([','.join(map(str,[cap,length,size,int(same)])),','.join(map(str,state)),E.read(db,32).hex(),E.read(new,32).hex(),';'.join(trace)]))
pathlib.Path(__file__).with_name('golden_player_reference_assign.tsv').write_text('\n'.join(rows)+'\n');print('200 native player-reference assignments captured')

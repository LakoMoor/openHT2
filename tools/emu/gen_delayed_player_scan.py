"""Native delayed-player scan with deletion as an explicit owner boundary."""
import pathlib,random
from emu import Emu,OBJ_BASE
E=Emu();head=OBJ_BASE;nodes=[head+0x1000+i*0x100 for i in range(6)];trace=[];mutate=0
E.set_u32(0x68b990,head);E.set_u32(0x71e814,0)
E.hook_thiscall(0x4018b0,lambda q:0,1)
def copy(q):q.set_u32(q.ecx,int.from_bytes(q.read(q.arg(0),4),'little'));return q.ecx
E.hook_thiscall(0x41d300,copy,1)
def delete(q):
 n=q.arg(0);trace.append(str(nodes.index(n)));prev=head
 while int.from_bytes(q.read(prev,4),'little')!=n:prev=int.from_bytes(q.read(prev,4),'little')
 q.set_u32(prev,int.from_bytes(q.read(n,4),'little'))
 if mutate and len(trace)==1:q.set_u32(nodes[0]+0x60,int.from_bytes(q.read(nodes[0]+0x60,4),'little')|0x20000)
 return 0
E.hook_ret(0x4466a0,delete)
r=random.Random(0x446af0);rows=[]
for i in range(200):
 flags=[r.getrandbits(20)&~0x20000 for _ in nodes];mask=r.randrange(64);mutate=i%2
 for j,n in enumerate(nodes):E.set_u32(n,nodes[j+1] if j+1<len(nodes) else head);E.set_u32(n+0x60,flags[j]|(0x20000 if mask>>j&1 else 0))
 E.set_u32(head,nodes[0]);trace.clear();E.call(0x446af0)
 rows.append(f'{mask},{mutate}\t'+','.join(trace))
pathlib.Path(__file__).with_name('golden_delayed_player_scan.tsv').write_text('\n'.join(rows)+'\n');print('200 native delayed-player scans captured')

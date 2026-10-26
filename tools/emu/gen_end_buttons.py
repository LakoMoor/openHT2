"""Original horizontal container layout used by end-screen button groups."""
import pathlib,random,struct
from emu import Emu,OBJ_BASE
r=random.Random(0x5bdee0);e=Emu();e.hook_ret(0x5bdc80,lambda emu:0);rows=[]
for i in range(200):
 win=i%2;n=3 if win else 2;widths=[r.randrange(20,200) for _ in range(n)]
 group=OBJ_BASE;nodes=[group+0x1000*(k+1) for k in range(n)]
 e.write(group,bytes(0x130));e.set_u32(group+0x120,nodes[0])
 for k,a in enumerate(nodes):
  e.write(a,bytes(0x130));e.set_u32(a+8,nodes[k+1] if k+1<n else 0)
  e.set_u32(a+0x34,widths[k]);e.set_u32(a+0x38,30)
 gap=(145 if win else 290)-widths[0]
 e.call(0x5bdee0,[gap],'i',this=group)
 xs=[struct.unpack('<i',e.read(a+0x2c,4))[0] for a in nodes]
 rows.append(f'{win}\t'+','.join(map(str,widths))+'\t'+','.join(map(str,xs)))
pathlib.Path(__file__).with_name('golden_end_buttons.tsv').write_text('\n'.join(rows)+'\n')
print('200 layouts captured')

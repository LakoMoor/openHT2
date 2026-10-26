"""Original 0x5c4940 queue disposal order and controller fields."""
import pathlib,random,struct
from emu import Emu,OBJ_BASE
r=random.Random(0x5c4940);e=Emu();events=[];c=OBJ_BASE
# Destructor and allocator are separate original calls: record destruction only.
e.hook_ret(0x602e80,lambda emu:events.append(emu.ecx) or 0)
e.hook_ret(0x63266c,lambda emu:0)
e.hook_ret(0x5e03c0,lambda emu:events.append(0xffffffff) or 0)
rows=[]
for i in range(200):
 commands=[0 if r.randrange(3)==0 else OBJ_BASE+0x10000+k*256 for k in range(r.randrange(30))]
 e.write(c,bytes(0x2000));e.set_u32(c+0x1ef0,len(commands))
 for k,p in enumerate(commands):e.set_u32(c+0xf50+k*4,p)
 events.clear();e.call(0x5c4940,this=c)
 fields=[struct.unpack('<I',e.read(c+o,4))[0] for o in [0x1ef0,0x1f20,0x1f08,0x1f0c,0x1f24]]
 rows.append('\t'.join(','.join(map(str,v)) for v in [commands,events,fields]))
pathlib.Path(__file__).with_name('golden_controller_reset.tsv').write_text('\n'.join(rows)+'\n')
print('200 resets captured')

"""Original camera state restoration 0x5e02f0."""
import pathlib,struct,random
from emu import Emu,OBJ_BASE
r=random.Random(0x5e02f0);e=Emu();c=OBJ_BASE;events=[]
e.hook_thiscall(0x5f2190,lambda q:events.append(q.arg(0)) or 0,2)
e.hook_thiscall(0x5df3d0,lambda q:events.append(q.arg(0)+1) or 0,1)
e.hook_thiscall(0x5d96c0,lambda q:events.append(q.arg(0)-12) or 0,2)
rows=[]
for i in range(200):
 present=i%2;projection=0 if i%3==0 else 1234
 values=[r.uniform(-10000,10000) for _ in range(5)]
 e.write(c,bytes(0x3000));e.set_u32(c+0x1e8,c+0x2000 if present else 0);e.set_u32(c+0x2bc,projection)
 for o,v in zip([0xbe8,0xbf0,0xb58,0xb60,0xb68],values):e.write(c+o,struct.pack('<d',v))
 events.clear();e.call(0x5e02f0,this=c)
 fields=[struct.unpack('<I',e.read(c+o,4))[0] for o in [0x2058,0x2054,0x204c,0x238,0x23c,0x240,0x244,0x248,0x24c]]
 rows.append(f'{present}\t{projection}\t'+','.join(map(repr,values))+'\t'+','.join(map(str,fields))+'\t'+','.join(map(str,events)))
pathlib.Path(__file__).with_name('golden_controller_camera.tsv').write_text('\n'.join(rows)+'\n');print('200 camera resets captured')


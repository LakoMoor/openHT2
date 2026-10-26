"""Original scenario camera 0x5dbd30: complete Euler construction and inverse."""
import pathlib,struct,random
from emu import Emu,OBJ_BASE
r=random.Random(0x5dbd30);e=Emu();c=OBJ_BASE;out=c+0x10000;rows=[]
for i in range(200):
 values=[r.uniform(-10000,10000) for _ in range(3)]+[r.uniform(-3,3) for _ in range(4)]
 for o,v in zip([0xb58,0xb60,0xb68,0xb70,0xb78,0xb80,0xb88],values):e.write(c+o,struct.pack('<d',v))
 e.call(0x5dbd30,[out],'i',this=c)
 rows.append(','.join(map(repr,values))+'\t'+','.join(map(str,struct.unpack('<12I',e.read(out,48)))))
pathlib.Path(__file__).with_name('golden_controller_camera_matrix.tsv').write_text('\n'.join(rows)+'\n');print('200 complete camera matrices captured')

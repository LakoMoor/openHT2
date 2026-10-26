"""Native scene-specific inverse 0x5ac480 with all arithmetic unhooked."""
import pathlib,random,struct
from emu import Emu,OBJ_BASE
r=random.Random(0x5ac480);e=Emu();src=OBJ_BASE;dst=src+0x100;rows=[]
for i in range(500):
 values=[r.uniform(-2,2) for _ in range(9)]+[r.uniform(-10000,10000) for _ in range(3)]
 for k in [0,4,8]:values[k]+=5
 raw=struct.pack('<12f',*values);e.write(src,raw);e.call(0x5ac480,[dst,src],'ii')
 rows.append(','.join(map(str,struct.unpack('<12I',raw)))+'\t'+','.join(map(str,struct.unpack('<12I',e.read(dst,48)))))
pathlib.Path(__file__).with_name('golden_scene_inverse.tsv').write_text('\n'.join(rows)+'\n');print('500 scene inverses captured')

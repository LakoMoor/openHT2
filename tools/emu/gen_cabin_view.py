"""Full original 0x4de990 view transform, including both cabin modes."""
import pathlib,random,struct
from emu import Emu,OBJ_BASE
e=Emu();r=random.Random(0x4de990);s=OBJ_BASE;v=s+0x10000;out=v+0x10000;rows=[]
e.write(0x7025f0,struct.pack('<12f',1,0,0,0,1,0,0,0,1,0,0,0));e.write(0x702630,struct.pack('<3f',0,0,1))
for case in range(360):
 present=case%12!=0;vehicle=case%12!=1;cabin=case%2;ignore=(case//2)%2;mode=[-3,-2,-1,0,1,2,3][case%7];enabled=(case//4)%2
 values=struct.pack('<3f',r.uniform(-3.2,3.2),r.uniform(-.4,.4),r.uniform(-1,1));look,height,tilt=struct.unpack('<3f',values)
 e.set_u32(0x6d2078,s if present else 0);e.set_u32(s+0x268,v if vehicle else 0);e.set_u32(0x6cec68,cabin);e.set_u32(s+0x574,mode);e.set_u32(v+0x51a8,enabled)
 e.write(0x696d10,struct.pack('<f',look));e.write(v+0x4f48,struct.pack('<f',height));e.write(s+200,struct.pack('<f',tilt))
 matrices=[]
 for o in [0x1d8,0x5f0]:
  raw=struct.pack('<12f',*[r.uniform(-2,2) for _ in range(12)]);e.write(s+o,raw);matrices.append(','.join(map(str,struct.unpack('<12I',raw))))
 e.call(0x4de990,[out,ignore],'ii')
 fields=[str(present*1),str(vehicle*1),str(cabin),str(ignore),str(mode),str(enabled)]+list(map(str,struct.unpack('<3I',values)))
 rows.append(','.join(fields)+'\t'+'\t'.join(matrices)+'\t'+','.join(map(str,struct.unpack('<12I',e.read(out,48)))))
pathlib.Path(__file__).with_name('golden_cabin_view.tsv').write_text('\n'.join(rows)+'\n');print('360 original cabin view transforms captured')

"""Original 0x5df3d0 camera pose decomposition and angular following."""
import pathlib,random,struct
from emu import Emu,OBJ_BASE
e=Emu();r=random.Random(0x5df3d0);c=OBJ_BASE;mat=c+0x10000;rows=[]
e.write(0x702620,struct.pack('<3f',0,1,0));e.write(0x702630,struct.pack('<3f',0,0,1))
e.write(0x7025f0,struct.pack('<12f',1,0,0,0,1,0,0,0,1,0,0,0))
for case in range(400):
 pose=[r.uniform(-1000,1000) for _ in range(3)]+[r.uniform(-3,3),r.uniform(-1.4,1.4),r.uniform(-3,3)]
 e.call(0x5da950,[mat]+pose,'i'+'d'*6)
 raw=e.read(mat,48)
 old=[r.uniform(-7,7) for _ in range(3)]
 if case%4==0:old=[pose[3],pose[4],pose[5]]
 for o,v in zip([0xb70,0xb78,0xb80],old):e.write(c+o,struct.pack('<d',v))
 e.call(0x5df3d0,[mat],'i',this=c)
 outputs=[struct.unpack('<Q',e.read(c+o,8))[0] for o in [0xb58,0xb60,0xb68,0xb70,0xb78,0xb80]]
 outputs.extend(struct.unpack('<3I',e.read(c+0x244,12)))
 for o,v in zip([0xb70,0xb78,0xb80],old):e.write(c+o,struct.pack('<d',v))
 camera=c+0x20000;node=c+0x30000;e.set_u32(c+0x1e8,camera);e.set_u32(c+0x268,node)
 e.call(0x5df560,[0x12345678,mat],'ii',this=c)
 installed=list(struct.unpack('<12I',e.read(node+0x40,48)))+[struct.unpack('<I',e.read(camera+0x4c,4))[0]]
 rows.append(','.join(map(str,struct.unpack('<12I',raw)))+'\t'+','.join(map(repr,old))+'\t'+','.join(map(str,outputs))+'\t'+','.join(map(str,installed)))
pathlib.Path(__file__).with_name('golden_camera_tracking.tsv').write_text('\n'.join(rows)+'\n');print('400 original camera tracking states captured')

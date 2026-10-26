"""0x53d1f0 cabin branch: actual math, captured ReplaceSpace commands only."""
import pathlib,random,struct
from emu import Emu,OBJ_BASE
e=Emu();r=random.Random(0x53d1f0);c=OBJ_BASE;p=c+0x10000;rows=[];commands=[]
e.set_u32(c+0x578,1);e.set_u32(0x696cb0,p);e.set_u32(p+0x12fc,1)
for o,v in [(0x13cc,0x3dc104fb),(0x13d4,0x3d20d97c),(0x13d0,0x4016cbe4)]:e.set_u32(p+o,v)
for i in range(3):e.set_u32(p+0x130c+i*4,i+1)
e.hook_ret(0x4e48e2,lambda q:commands.append(struct.unpack('<12I',q.read(q.arg(2),48))) or 0)
for case in range(240):
 values=[r.uniform(-180,180),r.uniform(0,80),r.uniform(0,1)];raw=struct.pack('<3f',*values);speed,rev,fuel=struct.unpack('<3f',raw);bases=[]
 for i in range(3):
  b=struct.pack('<12f',*[r.uniform(-2,2) for _ in range(12)]);e.write(p+0x133c+i*48,b);bases.append(','.join(map(str,struct.unpack('<12I',b))))
 commands.clear();e.call(0x53d1f0,[0,0,speed,p+0x2000,rev,0,0,0,fuel,0,0],'iififiiifii',this=c)
 assert len(commands)==3
 rows.append(','.join(map(str,struct.unpack('<3I',raw)))+'\t'+'\t'.join(bases)+'\t'+'\t'.join(','.join(map(str,x)) for x in commands))
pathlib.Path(__file__).with_name('golden_cabin_needles.tsv').write_text('\n'.join(rows)+'\n');print('240 original cabin needle command triples captured')

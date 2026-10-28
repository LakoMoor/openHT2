"""Real manager updates and particle destruction across several ticks; free observed."""
import pathlib,random,struct
from emu import Emu,OBJ_BASE
r=random.Random(0x5796e0);e=Emu();rows=[];pool=OBJ_BASE+0x1000;clock=OBJ_BASE+0x3000;colors=OBJ_BASE+0x3800
e.hook_ret(0x63266c,lambda em:0)
for case in range(100):
 raw=bytearray(0x5000);count=case%5;ticks=case%7+1
 def put(a,v):struct.pack_into('<I',raw,a-OBJ_BASE,v)
 def fl(a,v):struct.pack_into('<f',raw,a-OBJ_BASE,v)
 put(OBJ_BASE+0x1b0,1);put(OBJ_BASE+0x20,pool);put(pool+8,pool+0x100 if count else 0);struct.pack_into('<d',raw,clock-OBJ_BASE+0xb90,.1)
 for j in range(100):put(colors+j*4,j*1234567)
 for i in range(count):
  obj=pool+0x100+i*0x100;g=OBJ_BASE+0x2000+i*0x100;x=g+0x40
  for offset,value in [(0,0x650dd4),(8,obj+0x100 if i+1<count else 0),(12,obj-0x100 if i else 0),(16,pool),(0x48,g),(0x58,clock),(0x70,50),(0x78,colors),(0x80,1)]:put(obj+offset,value)
  put(g+0x10,x);put(g+0x14,x+16)
  # A valid zero-length descriptor array allows real destructor execution.
  put(g-4,0)
  for offset in [0x4c,0x50,0x54,0xa0,0xa4,0xa8,0x84,0x88,0x8c,0x90]:fl(obj+offset,r.uniform(-2,2))
  for offset,value in [(0x60,.1),(0x64,r.uniform(.05,.6)),(0xac,3.)]:fl(obj+offset,value)
 e.write(OBJ_BASE,raw)
 for _ in range(ticks):e.call(0x5796e0,this=OBJ_BASE)
 rows.append(str(ticks)+'\t'+raw.hex()+'\t'+e.read(OBJ_BASE,0x5000).hex())
pathlib.Path(__file__).with_name('golden_particle_manager_full.tsv').write_text('\n'.join(rows)+'\n');print('100 multi-tick managers captured')

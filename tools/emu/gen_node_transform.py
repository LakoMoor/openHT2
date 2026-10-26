"""Original 0x5f25c0 transform chains, including mode mutation and bad roots."""
import pathlib,random,struct
from emu import Emu,OBJ_BASE
e=Emu();r=random.Random(0x5f25c0);base=OBJ_BASE;out=base+0x10000;rows=[]
for case in range(320):
 count=1+case%8;nodes=[]
 for i in range(count):
  node=base+i*0x100;e.write(node,bytes(0x100));mode=r.choice([0,1,2,3,0xffffffff])
  raw=struct.pack('<12f',*[r.uniform(-2,2) for _ in range(12)])
  e.set_u32(node+0x2c,node+0x100 if i+1<count else 0);e.set_u32(node+0x38,mode);e.write(node+0x40,raw)
  nodes.append(str(mode)+','+','.join(map(str,struct.unpack('<12I',raw))))
 initial=struct.pack('<12f',*[r.uniform(-2,2) for _ in range(12)]);e.write(out,initial)
 e.call(0x5f25c0,[out],'i',this=base);result=e.eax
 rows.append(';'.join(nodes)+'\t'+','.join(map(str,struct.unpack('<12I',initial)))+'\t'+str(result)+'\t'+','.join(map(str,struct.unpack('<12I',e.read(out,48))))+'\t'+','.join(str(struct.unpack('<I',e.read(base+i*0x100+0x38,4))[0]) for i in range(count)))
pathlib.Path(__file__).with_name('golden_node_transform.tsv').write_text('\n'.join(rows)+'\n');print('320 original transform chains captured')

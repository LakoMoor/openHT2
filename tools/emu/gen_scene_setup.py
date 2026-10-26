"""Matrix-stack state of original 0x5ae0e0; device uploads excluded."""
import pathlib,struct,random
from emu import Emu,OBJ_BASE
r=random.Random(0x5ae0e0);e=Emu();c=OBJ_BASE;a=c+0x10000;b=a+0x100
e.write(0x7025f0,struct.pack('<12f',1,0,0,0,1,0,0,0,1,0,0,0))
e.set_u32(0x702e08,3);e.hook_ret(0x5adea0,lambda q:0);e.set_u32(0x720904,0);rows=[]
for case in range(160):
 software=case%2;e.set_u32(0x720908,0 if software else 1);e.write(c,bytes(0x2200));blend=r.randrange(-50,150)
 matrices=[]
 for base in [a,b]:
  values=[r.uniform(-1,1) for _ in range(12)]
  for k in [0,4,8]:values[k]+=3
  raw=struct.pack('<12f',*values);e.write(base,raw);matrices.append(','.join(map(str,struct.unpack('<12I',raw))))
 e.write(c+0x2110,struct.pack('<d',2.));e.write(c+0x2148,struct.pack('<d',250.));e.set_u32(c+0x2108,blend)
 for k,v in enumerate([1,2,100<<19,4]):e.set_u32(c+0x7a0+k*4,v)
 e.set_u32(0x67ef88,9);e.call(0x5ae0e0,[a,b],'ii',this=c)
 bits=[]
 for o in [0x1070,0x10a0,0x8f0,0xcb0,0x10d0,0x1490]:bits.extend(struct.unpack('<12I',e.read(c+o,48)))
 bits.extend(struct.unpack('<I',e.read(c+o,4))[0] for o in [0x2020,0x2024,0x2028,0x202c,0x7b0,0x800,0x850,0x8a0,0x6d8,0x2150,0x2154,0x20fc])
 rows.append(f'{software}\t'+ '\t'.join(matrices)+f'\t{blend}\t'+','.join(map(str,bits)))
pathlib.Path(__file__).with_name('golden_scene_setup.tsv').write_text('\n'.join(rows)+'\n');print('160 scene setups captured')


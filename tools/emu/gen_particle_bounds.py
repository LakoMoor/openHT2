"""Native geometry bounds 0x5e6800, finite synthetic vertices."""
import pathlib,random,struct
from emu import Emu,OBJ_BASE
r=random.Random(0x5e6800);e=Emu();rows=[];x=OBJ_BASE+0x1000;z=x+0x1000;bound=z+0x1000
e.set_u32(OBJ_BASE+0x10,x);e.set_u32(OBJ_BASE+0x14,z)
for case in range(400):
 n=case%9;values=[r.uniform(-100,100) for _ in range(3+2*n)];raw=struct.pack('<'+'f'*len(values),*values);e.write(OBJ_BASE+4,struct.pack('<H',n));e.write(bound,raw[:12]);e.write(x,raw[12:12+4*n]);e.write(z,raw[12+4*n:]);e.call(0x5e6800,[bound],'i',this=OBJ_BASE)
 rows.append(str(n)+'\t'+raw.hex()+'\t'+e.read(bound,16).hex())
pathlib.Path(__file__).with_name('golden_particle_bounds.tsv').write_text('\n'.join(rows)+'\n');print('400 geometry bounds captured')

"""Native sphere merge 0x5e5580 on finite inputs."""
import pathlib,random,struct
from emu import Emu,OBJ_BASE
r=random.Random(0x5e5580);e=Emu();rows=[]
for case in range(400):
 v=[r.uniform(-100,100) for _ in range(8)];v[3]=abs(v[3]);v[7]=abs(v[7])
 if case%7==0:v[4:7]=v[:3]
 raw=struct.pack('<8f',*v);e.write(OBJ_BASE,raw);e.call(0x5e5580,[OBJ_BASE+16],'i',this=OBJ_BASE)
 rows.append(raw.hex()+'\t'+e.read(OBJ_BASE,16).hex())
pathlib.Path(__file__).with_name('golden_bounds_merge.tsv').write_text('\n'.join(rows)+'\n');print('400 sphere merges captured')

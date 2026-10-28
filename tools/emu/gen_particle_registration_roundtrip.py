"""Native sort registration/unlink round trips in varying removal order."""
import pathlib,random,struct
from emu import Emu,OBJ_BASE
r=random.Random(0x593270);e=Emu();rows=[];slots=OBJ_BASE+0x100
for case in range(200):
 raw=bytearray(0x1000);count=case%8;struct.pack_into('<I',raw,0x30,slots);struct.pack_into('<I',raw,0x34,1);struct.pack_into('<I',raw,0x24,r.getrandbits(32));e.write(OBJ_BASE,raw)
 nodes=[OBJ_BASE+0x200+i*0x80 for i in range(count)]
 for node in nodes:e.call(0x593270,[0,node],'ii',this=OBJ_BASE)
 r.shuffle(nodes);remove=nodes[:case%(count+1)]
 for node in remove:e.call(0x5ef780,this=node)
 rows.append(raw.hex()+'\t'+str(count)+'\t'+','.join(map(str,remove))+'\t'+e.read(OBJ_BASE,0x1000).hex())
pathlib.Path(__file__).with_name('golden_particle_registration_roundtrip.tsv').write_text('\n'.join(rows)+'\n');print('200 registration/unlink sequences captured')

"""0x5fa4f0 and 0x5fa5f0 with bounds routines observed."""
import pathlib,random
from emu import Emu,OBJ_BASE
r=random.Random(0x5fa5f0);e=Emu();rows=[];trace=[]
def bound(em):trace.append([em.ecx,em.arg(0)]);return 0
e.hook_thiscall(0x5e6800,bound,1);e.hook_thiscall(0x5e5580,bound,1)
for case in range(300):
 raw=bytes(r.getrandbits(8) for _ in range(0x44));geometry=OBJ_BASE+0x1000;count=case%5;trace.clear();e.write(OBJ_BASE,raw)
 e.call(0x5fa4f0,[geometry,count],'ii',this=OBJ_BASE);e.call(0x5fa5f0,this=OBJ_BASE)
 # Stack addresses of temporary bounds vary; retain receiver/call count only.
 rows.append(raw.hex()+'\t'+str(count)+'\t'+e.read(OBJ_BASE,0x44).hex()+'\t'+str(len(trace)))
pathlib.Path(__file__).with_name('golden_particle_attach.tsv').write_text('\n'.join(rows)+'\n');print('300 geometry attachments captured')

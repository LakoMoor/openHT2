"""Native smoke material enumeration with controlled lookup."""
import pathlib
from emu import Emu,OBJ_BASE
e=Emu();rows=[];state={};names=[]
def lookup(em):
 p=em.arg(0);b=bytearray()
 while e.read(p,1)!=b'\0':b+=e.read(p,1);p+=1
 names.append(b.decode());i=len(names)-1;return OBJ_BASE+i*0x100 if i<state['count'] else 0
e.hook_ret(0x4b8720,lookup)
for count in range(1,25):
 state['count']=count;names.clear()
 for i in range(count):e.write(OBJ_BASE+i*0x100,bytes(0x60))
 e.call(0x4b8c40)
 rows.append(str(count)+'\t'+','.join(names)+'\t'+','.join(str(int.from_bytes(e.read(OBJ_BASE+i*0x100+0x20,4),'little')) for i in range(count)))
pathlib.Path(__file__).with_name('golden_smoke_materials.tsv').write_text('\n'.join(rows)+'\n');print('24 material enumerations captured')

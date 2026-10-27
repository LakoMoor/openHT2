"""Native cargo lookup 0x4777b0; diagnostics intercepted."""
import pathlib,random
from emu import Emu,OBJ_BASE
E=Emu();id=OBJ_BASE;truck=id+0x10000;trailer=id+0x20000;trace=[]
E.hook_ret(0x401010,lambda q:trace.append('diagnostic') or 0)
r=random.Random(0x4777b0);rows=[]
for i in range(400):
 t=r.randrange(3);s=r.randrange(3);a=r.randrange(2);b=r.randrange(2);fallback=r.randrange(2)
 E.set_u32(id+0x1fc,truck if t else 0);E.set_u32(id+0x200,trailer if s else 0);E.set_u32(truck+4,0xffffffff if t==1 else 7);E.set_u32(trailer+4,0xffffffff if s==1 else 8)
 for obj,nonempty in [(truck,a),(trailer,b)]:E.set_u32(obj+0x1c,obj+0x1000);E.set_u32(obj+0x20,obj+0x1000+nonempty*0x90)
 E.set_u32(id,0x12340000);E.set_u32(id+4,0x12340000+fallback*4);trace.clear();E.call(0x4777b0,[id],'i')
 rows.append(','.join(map(str,[t,s,a,b,fallback]))+'\t'+str(E.eax)+'\t'+';'.join(trace))
pathlib.Path(__file__).with_name('golden_cargo_lookup.tsv').write_text('\n'.join(rows)+'\n');print('400 native cargo lookups captured')

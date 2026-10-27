"""Native derived-player flags with the native cargo lookup."""
import pathlib,random
from emu import Emu,OBJ_BASE
E=Emu();p=OBJ_BASE;fallback=p+0x10000;truck=p+0x20000;tech=p+0x30000;cargo=0;trace=[]
E.set_u32(0x68b990,fallback)
def lookup(q):trace.append('cargo');return cargo
# 0x4777b0 executes original code; only the invalid-vehicle diagnostic is hooked.
E.hook_ret(0x401010,lambda q:trace.append('diagnostic') or 0)
r=random.Random(0x477210);rows=[]
for i in range(400):
 flags=r.getrandbits(32);tracked=r.randrange(3);boss=r.randrange(2);members=r.randrange(2);cargo=r.randrange(2);route=r.randrange(2);vehicle=1;attachment=r.randrange(2);human=r.randrange(2);engine=r.randrange(2)
 E.set_u32(p+0x60,flags);E.mu.mem_write(p+0x1f0,bytes([tracked]));E.set_u32(p+0xb0,p+0x40000 if boss else fallback);E.set_u32(p+0xa0,0x111);E.set_u32(p+0xa4,0x111+members*4);E.set_u32(p+0x18,0x222);E.set_u32(p+0x1c,0x222+route*12);E.set_u32(p+0x204,truck if vehicle or human else 0);E.set_u32(truck+0x10,tech);E.set_u32(truck+0x28,engine);E.set_u32(tech+0x80,attachment);E.set_u32(p+0x5c,1 if human else 8);E.set_u32(p+0x208,0);E.set_u32(truck+4,1);E.set_u32(truck+0x1c,0x2a050000);E.set_u32(truck+0x20,0x2a050000+cargo*0x90);trace.clear();E.call(0x477210,[p],'i')
 rows.append(','.join(map(str,[flags,tracked,boss,members,cargo,route,vehicle,attachment,human,engine]))+'\t'+str(int.from_bytes(E.read(p+0x60,4),'little'))+'\t'+';'.join(trace))
pathlib.Path(__file__).with_name('golden_player_flags_native_cargo.tsv').write_text('\n'.join(rows)+'\n');print('400 native derived-player flag updates captured')

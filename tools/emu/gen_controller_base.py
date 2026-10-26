"""Original base scenario reset with observable subsystem callbacks."""
import pathlib,struct,random
from emu import Emu,OBJ_BASE,STUB
r=random.Random(0x5e03c0);e=Emu();c=OBJ_BASE;v=c+0x4000;config=c+0x5000;events=[]
e.set_u32(0x721750,config);e.set_u32(v+0x7c,STUB+0x400);e.set_u32(v+0x78,STUB+0x410)
e.hook_ret(0x5e4d30,lambda q:events.append(q.ecx) or 0)
e.hook_ret(0x5fd160,lambda q:events.append(1) or 0)
e.hook_ret(0x5d3460,lambda q:events.append(q.ecx) or 0)
e.hook_ret(STUB+0x400,lambda q:events.append(0x7c) or 0);e.hook_ret(STUB+0x410,lambda q:events.append(0x78) or 0)
# A real fld loads the supplied clock value into ST0.
e.write(0x6037e0,b'\xdd\x05'+struct.pack('<I',c+0x6000)+b'\xc3')
offsets=[0x250,0x254,0x258,0x25c,0xb90,0xb94,0xba0,0xba4,0xb98,0xb9c,0x220,0xb48,0xb38,0xb40,0xb50,0xb88,0xb4c,0xb3c,0xb44,0xb54,0xb8c,0xba8,0x1cc,0x1c8,0xbac,0xbd0,0x2008,0x308c]
rows=[]
for i in range(200):
 t=r.randrange(100000000);linked=i%2;audio=(i//2)%2
 e.write(c,b'\x55'*0xc00);e.set_u32(c,v);e.set_u32(c+0x1e4,c+0x2000 if linked else 0);e.set_u32(c+0x1d4,c+0x3000)
 e.set_u32(c+0x2008,0x12345678);e.set_u32(c+0x308c,0x87654321);e.set_u32(config+0x158,audio);e.write(c+0x6000,struct.pack('<d',t))
 events.clear();e.call(0x5e03c0,this=c)
 vals=[struct.unpack('<I',e.read(c+o,4))[0] for o in offsets]
 rows.append(f'{t}\t{linked}\t{audio}\t'+','.join(map(str,vals))+'\t'+','.join(map(str,events)))
pathlib.Path(__file__).with_name('golden_controller_base.tsv').write_text('\n'.join(rows)+'\n');print('200 base resets captured')

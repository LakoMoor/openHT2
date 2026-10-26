"""Actual 0x5812a0, replacing diagnostic formatting/output only."""
import pathlib,random,struct
from emu import Emu,OBJ_BASE
e=Emu();body=OBJ_BASE;external=body+0x10000;errors=[];r=random.Random(0x5812a0)
e.hook_ret(0x61cca9,lambda q:0);e.hook_ret(0x4e260c,lambda q:errors.append(1) or 0)
e.set_u32(0x6d2078,0);rows=[]
e.set_u32(body+0x320,external+0x1000)
e.set_u32(body+0x20,external+0x2000);e.set_u32(external+0x2040,external+0x3000)
offsets=[0x1a0,0x1a4,0x1a8,0xce4,0xce8,0xcec,0x378,0xd84]
for case in range(400):
    source=r.choice([external,body+0x19c,body+0x1a0,body+0x1a4])
    values=[r.choice([-151.,-150.,-0.,0.,150.,151.,1e20,r.uniform(-200,200)]) for _ in range(3)]
    raw=struct.pack('<3f',*values);e.write(source,raw);errors.clear()
    e.call(0x5812a0,[source,0],'ii',this=body)
    result=[int.from_bytes(e.read(body+o,4),'little') for o in offsets]
    rows.append('\t'.join([str(source),','.join(map(str,struct.unpack('<3I',raw))),','.join(map(str,result)),str(len(errors))]))
pathlib.Path(__file__).with_name('golden_body_velocity.tsv').write_text('\n'.join(rows)+'\n')
print('400 native body velocity updates captured')

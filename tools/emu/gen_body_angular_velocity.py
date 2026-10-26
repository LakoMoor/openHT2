"""Full native 0x581430; only diagnostic formatting/output replaced."""
import pathlib,random,struct
from emu import Emu,OBJ_BASE
e=Emu();body=OBJ_BASE;external=body+0x10000;errors=[];r=random.Random(0x581430)
e.hook_ret(0x61cca9,lambda q:0);e.hook_ret(0x4e260c,lambda q:errors.append(1) or 0)
e.set_u32(0x6d2078,0);e.set_u32(body+0x320,external+0x1000)
e.set_u32(body+0x20,external+0x2000);e.set_u32(external+0x2040,external+0x3000)
rows=[]
def bits(raw):return ','.join(map(str,struct.unpack('<'+'I'*(len(raw)//4),raw)))
for case in range(400):
    source=r.choice([external,body+0x1a8,body+0x1ac,body+0x1b0])
    old=struct.pack('<3f',*[r.uniform(-5,5) for _ in range(3)])
    inertia=struct.pack('<3f',*[r.uniform(0.1,5) for _ in range(3)])
    axes=struct.pack('<9f',*[r.uniform(-2,2) for _ in range(9)])
    raw=struct.pack('<3f',*[r.choice([-10001.,-10000.,-11.,-10.,-0.,0.,10.,11.,10000.,10001.,float('nan')]) for _ in range(3)])
    e.write(body+0x1ac,old);e.write(body+0x194,inertia);e.write(body+0x1f4,axes);e.write(source,raw)
    errors.clear();e.call(0x581430,[source,0],'ii',this=body)
    result=e.read(body+0x1ac,24)+e.read(body+0xcf0,24)
    rows.append('\t'.join([str(source),bits(old),bits(inertia),bits(axes),bits(raw),bits(result),str(len(errors))]))
pathlib.Path(__file__).with_name('golden_body_angular_velocity.tsv').write_text('\n'.join(rows)+'\n')
print('400 native body angular velocity updates captured')

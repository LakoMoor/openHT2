"""Original full XYZ bounds and XY-only diagnostic validation.

Only diagnostic formatting/output are replaced; comparison logic is native.
"""
import pathlib,random,struct
from emu import Emu,OBJ_BASE
e=Emu();v=OBJ_BASE;name=v+0x1000;e.write(name,b'test\0');errors=[]
e.hook_ret(0x61cca9,lambda q:0)
e.hook_ret(0x4e260c,lambda q:errors.append(1) or 0)
r=random.Random(0x4e2821);rows=[]
for case in range(600):
    vector=[r.choice([-101.,-100.,-0.,0.,99.,100.,101.,float('nan'),float('inf'),-float('inf'),r.uniform(-200,200)]) for _ in range(3)]
    limit=r.choice([-100.,-0.,0.,100.,float('nan'),float('inf')])
    raw=struct.pack('<4f',*vector,limit);e.write(v,raw[:12]);original=e.read(v,12)
    e.call(0x4e28b0,[v,limit],'if');xyz=e.eax
    errors.clear();e.call(0x4e2821,[v,name,limit],'iif');assert e.read(v,12)==original
    rows.append('\t'.join(map(str,list(struct.unpack('<4I',raw))+[xyz,len(errors)])))
pathlib.Path(__file__).with_name('golden_velocity_validation.tsv').write_text('\n'.join(rows)+'\n')
print('600 native XYZ/XY velocity validations captured')

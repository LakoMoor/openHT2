"""Native plane sides at cancellation and representable boundary points."""
import pathlib,random,struct,math
from emu import Emu,OBJ_BASE
r=random.Random(0x5f88d5);e=Emu();v=OBJ_BASE;data=v+128;point=v+256;rows=[];different=0
f=lambda x:struct.unpack('<f',struct.pack('<f',x))[0]
e.set_u32(v,5);e.set_u32(v+8,data)
for case in range(600):
    p=[f(r.uniform(-10000,10000)) for _ in range(3)];a=[f(r.uniform(-1,1)) for _ in range(3)]
    d=f(-((p[2]*a[2]+p[1]*a[1])+p[0]*a[0]));a.append(d)
    if case%5==0:
        a=[1.,1.,1.,0.];p=[16777216.,1.,-16777216.]
    if case%11==0:p[case%3]=r.choice([float('nan'),float('inf'),-float('inf')])
    raw=struct.pack('<7f',*a,*p);e.write(data,raw[:16]);e.write(point,raw[16:]);e.call(0x5f88d0,[point],'i',this=v)
    old=f(f(f(p[0]*a[0])+f(a[1]*p[1]))+f(a[2]*p[2]));old=f(old+a[3])>0
    different+=old!=bool(e.eax)
    rows.append(raw.hex()+'\t'+str(e.eax))
pathlib.Path(__file__).with_name('golden_split_side.tsv').write_text('\n'.join(rows)+'\n')
print(f'600 native plane comparisons; former f32 expression disagrees in {different}')

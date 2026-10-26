"""Original 0x4cedd0/0x4cee70 with supplied world-transform translations.

Only 0x57de90 scene lookup is a boundary hook; vector math is native.
"""
import pathlib,random,struct
from emu import Emu,OBJ_BASE

e=Emu();vehicle=OBJ_BASE;human=vehicle+0x10000;scenario=vehicle+0x20000;driver=vehicle+0x30000
e.set_u32(0x6d2078,scenario);e.set_u32(scenario+0x268,human);e.set_u32(vehicle+0x5460,driver)
positions={}
def world_transform(q):
    dst=q.arg(0)
    q.write(dst,struct.pack('<12f',1,0,0,0,1,0,0,0,1,*positions[q.ecx]))
    return dst
e.hook_thiscall(0x57de90,world_transform,1)
r=random.Random(0x4cee70);rows=[]
for case in range(600):
    h=[r.uniform(-10000,10000) for _ in range(3)]
    v=[h[i]+r.uniform(-350,350) for i in range(3)]
    if case<180:
        h=[0.,0.,0.];v=[0.,0.,0.]
        v[case%3]=[79.99999,80.,80.00001,99.99999,100.,100.00001,299.99997,300.,300.00003,float('nan')][case%10]
    h=list(struct.unpack('<3f',struct.pack('<3f',*h)));v=list(struct.unpack('<3f',struct.pack('<3f',*v)))
    positions[human+0x10]=h;positions[vehicle+0x10]=v
    lateral=r.choice([-1.,-0.50000001,-0.5,0.,0.5,0.50000001,1.,float('nan')]);flags=r.getrandbits(8)
    e.write(driver+0x40b8,struct.pack('<d',lateral));e.set_u32(driver+0x4070,flags)
    e.call(0x4cedd0,[vehicle],'i');enter=e.eax
    e.call(0x4cee70,[vehicle],'i');leave=e.eax
    bits=list(struct.unpack('<6I',struct.pack('<6f',*(h+v))))
    rows.append(','.join(map(str,bits))+'\t'+str(int.from_bytes(struct.pack('<d',lateral),'little'))+'\t'+str(flags)+'\t'+str(enter)+'\t'+str(leave))
pathlib.Path(__file__).with_name('golden_ai_simulation_mode.tsv').write_text('\n'.join(rows)+'\n')
print('600 original near/far simulation-mode decisions captured')

"""Sky brightness 0x5a9d40 (x87 fsin) and the dust gate 0x4bb010 built on it."""
import pathlib,random,struct
from emu import Emu,OBJ_BASE
from unicorn.x86_const import UC_X86_REG_EAX
r=random.Random(0x5a9d40);e=Emu()
sky=OBJ_BASE+0x60000
consts={a:struct.unpack('<d',e.read(a,8))[0] for a in (0x64da00,0x64d0f0,0x64b670,0x64b6e0,0x64b678,0x64b7a8)}
print({hex(k):v for k,v in consts.items()})
print('f32 0.001 =',struct.unpack('<f',e.read(0x64c1c8,4))[0])
rows=[]
for case in range(500):
    pre=bytearray(r.randrange(256) for _ in range(0x240))
    struct.pack_into('<f',pre,0x220,r.choice([r.uniform(-0.3,0.8),0.0,0.5,0.25,0.75]))
    struct.pack_into('<d',pre,0x78,r.choice([r.uniform(-20,20),0.0,3.14159]))
    struct.pack_into('<d',pre,0x68,r.choice([r.uniform(-20,20),0.0,1.5707963]))
    struct.pack_into('<f',pre,0x218,r.choice([r.uniform(0,0.01),0.0,0.001,0.0010000001,0.0009999]))
    arg=r.choice([0.0,r.uniform(-4,4)])
    e.write(sky,bytes(pre));e.write(0x6f33ac,struct.pack('<I',sky))
    st0=e.call(0x5a9d40,[arg],'d',this=sky)
    e.call(0x4bb010,[],'');gate=e.mu.reg_read(UC_X86_REG_EAX)&0xff
    rows.append(chr(9).join([bytes(pre).hex(),struct.pack('<d',arg).hex(),struct.pack('<d',st0).hex(),str(gate)]))
pathlib.Path(__file__).with_name('golden_sky_gate.tsv').write_text(chr(10).join(rows)+chr(10))
print(len(rows))

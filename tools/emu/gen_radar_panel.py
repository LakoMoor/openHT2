"""Execute radar prefix of 0x538370, stopping after signal consumption.

Only geometry-key and sound calls are captured. Original arithmetic and clock
helper execute unchanged; remaining unrelated viewer update is skipped.
"""
import pathlib, random, struct
from emu import Emu, OBJ_BASE
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_EIP

e=Emu();viewer=OBJ_BASE;vehicle=viewer+0x10000;clock=vehicle+0x10000
e.set_u32(0x696ca0,clock);e.set_u32(0x6974c4,clock);e.set_u32(0x6d1d44,123)
e.set_u32(viewer+0x1b8,11);e.set_u32(viewer+0x1bc,12)
commands=[];sound=[];rows=[];r=random.Random(0x538370)
e.hook_ret(0x5e1a10,lambda q:commands.append((q.arg(0),q.arg(1))) or 0)
e.hook_ret(0x52f930,lambda q:sound.append(q.arg(1)) or 0)
e.mu.hook_add(UC_HOOK_CODE,lambda mu,a,s,u:mu.reg_write(UC_X86_REG_EIP,0x53b121),begin=0x53852f,end=0x53852f)
for i in range(512):
    present=int(i%19!=0);equipment=r.choice([0,1,2,-1]);cab=i%2
    signal=r.choice([-1.,0.,0.0001,0.2,1/3,0.5,2/3,0.9,1.,2.,float('nan')])
    dt=r.choice([0.,0.001,1/60,0.1,0.25,1.,-0.1])
    ticks=r.choice([-2147483648,-1,0,1,20,2147483647,r.randrange(1000)])
    start=r.choice([-2147483648,-1,0,10,2147483647,r.randrange(1000)])
    period=r.choice([-2147483648,-3,-2,-1,0,1,2,3,10,2147483647])
    e.set_u32(viewer+0x268,vehicle if present else 0);e.set_u32(vehicle+0x5400,equipment)
    raw=struct.pack('<f',signal);e.write(vehicle+0x51d8,raw)
    e.set_u32(viewer+0x578,cab);e.set_u32(viewer+0x1c0,start);e.set_u32(viewer+0x1c4,period)
    e.set_u32(clock+0xbb0,ticks);e.write(clock+0xb90,struct.pack('<d',dt))
    commands.clear();sound.clear();e.call(0x538370,[0,0],'ii',this=viewer)
    output=[int.from_bytes(e.read(a,4),'little') for a in [viewer+0x1c0,viewer+0x1c4,vehicle+0x51d8]]
    rows.append('\t'.join([','.join(map(str,[present,equipment,cab,int.from_bytes(raw,'little'),ticks,start,period])),str(dt),','.join(map(str,output)),';'.join(f'{n},{v}' for n,v in commands),','.join(map(str,sound))]))
pathlib.Path(__file__).with_name('golden_radar_panel.tsv').write_text('\n'.join(rows)+'\n')
print('512 original radar updates captured')

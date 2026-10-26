"""Native condition calculation and police activity/radar phase.

Starts after geometry gathering. Flags/speed queries are boundary stubs.
"""
import pathlib,random,struct
from emu import Emu,OBJ_BASE,STUB
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_ESP,UC_X86_REG_EBP,UC_X86_REG_EBX,UC_X86_REG_EAX,UC_X86_REG_EIP

e=Emu();vehicle=OBJ_BASE;police=vehicle+0x10;player=vehicle+0x10000
parts=vehicle+0x20000;state=vehicle+0x30000;driver=vehicle+0x40000
e.set_u32(vehicle+0x2540,parts);e.set_u32(vehicle+0x5454,state);e.set_u32(police+0x5450,driver)
flags=0;distance=0.
e.hook_ret(0x43f620,lambda q:flags);e.write(0x42c780,b'\xd9\xee\xc3')
def enter(mu,a,s,u):
    mu.reg_write(UC_X86_REG_EBP,police);mu.reg_write(UC_X86_REG_EBX,vehicle);mu.reg_write(UC_X86_REG_EAX,driver)
    sp=mu.reg_read(UC_X86_REG_ESP)
    e.set_u32(sp+0x10,driver);e.set_u32(sp+0x34,player);e.write(sp+0x50,struct.pack('<d',distance))
e.mu.hook_add(UC_HOOK_CODE,enter,begin=0x4d1492,end=0x4d1492)
for end in [0x4d1526,0x4d1685,0x4d2421]:
    e.mu.hook_add(UC_HOOK_CODE,lambda mu,a,s,u:mu.reg_write(UC_X86_REG_EIP,STUB),begin=end,end=end)
r=random.Random(0x4d1492);rows=[]
for line in pathlib.Path(__file__).with_name('golden_vehicle_condition.tsv').read_text().splitlines()[:400]:
    inputs=list(map(int,line.split('\t')[0].split(',')))
    e.set_u32(vehicle+0x5080,inputs[0]);e.set_u32(vehicle+0x224,inputs[1]);e.set_u32(state+12,inputs[2])
    for i in range(9):
        e.set_u32(parts+i*20+4,inputs[3+i*2]);e.set_u32(parts+i*20+8,inputs[4+i*2])
    mode=r.choice([4,5,5,5,6]);flags=r.getrandbits(24);distance=r.choice([0.,10.,50.,99.999,100.,200.])
    initial=[r.getrandbits(32) for _ in range(4)]
    addresses=[police+0x5594,driver+0x4074,driver+0x4068,player+0x51d8]
    for a,v in zip(addresses,initial):e.set_u32(a,v)
    e.set_u32(police+0x5568,mode);e.call(0x4d1492)
    output=[int.from_bytes(e.read(a,4),'little') for a in addresses]
    rows.append(','.join(map(str,inputs))+'\t'+','.join(map(str,[mode,flags]+initial))+'\t'+str(distance)+'\t'+','.join(map(str,output)))
pathlib.Path(__file__).with_name('golden_police_activity.tsv').write_text('\n'.join(rows)+'\n')
print('400 native police condition/radar phases captured')

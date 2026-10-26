"""Capture 0x4d14b9..0x4d1526 with original arithmetic and branches.

Earlier geometry/distance and activity tests are outside this fragment. The
flags query is supplied, and the unrelated speed query returns zero.
"""
import pathlib, random, struct
from emu import Emu, OBJ_BASE, STUB
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_ESP, UC_X86_REG_EBP, UC_X86_REG_EIP

e=Emu();police=OBJ_BASE;player=police+0x10000;flags=0;distance=0.
e.hook_ret(0x43f620,lambda q:flags)
e.write(0x42c780,b'\xd9\xee\xc3')  # boundary speed query: fldz; ret
def enter(mu,a,s,u):
    mu.reg_write(UC_X86_REG_EBP,police)
    stack=mu.reg_read(UC_X86_REG_ESP)
    e.set_u32(stack+0x34,player);e.write(stack+0x50,struct.pack('<d',distance))
e.mu.hook_add(UC_HOOK_CODE,enter,begin=0x4d14b9,end=0x4d14b9)
for exit_address in [0x4d1526,0x4d1685]:
    e.mu.hook_add(UC_HOOK_CODE,lambda mu,a,s,u:mu.reg_write(UC_X86_REG_EIP,STUB),begin=exit_address,end=exit_address)
r=random.Random(0x4d14b9);rows=[]
for i in range(400):
    state=r.choice([-1,0,4,5,5,5,6]);flags=r.getrandbits(24)
    distance=r.choice([0.,-1.,99.999999,100.,100.000001,10.,float('inf'),float('nan'),r.uniform(0,200)])
    old=r.getrandbits(32);e.set_u32(police+0x5568,state);e.set_u32(player+0x51d8,old)
    e.call(0x4d14b9)
    result=int.from_bytes(e.read(player+0x51d8,4),'little')
    rows.append('\t'.join(map(str,[state,flags,struct.unpack('<Q',struct.pack('<d',distance))[0],old,result])))
pathlib.Path(__file__).with_name('golden_police_radar.tsv').write_text('\n'.join(rows)+'\n')
print('400 police radar branch states captured')

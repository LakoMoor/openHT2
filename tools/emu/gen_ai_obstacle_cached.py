"""Native obstacle tail when the cached and current driver differ."""
import pathlib,random,struct
from emu import Emu,OBJ_BASE,STUB
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_ESP,UC_X86_REG_ESI,UC_X86_REG_EDI,UC_X86_REG_EBX,UC_X86_REG_EIP
e=Emu();vehicle=OBJ_BASE;driver=vehicle+0x10000;other=vehicle+0x50000;flags=vehicle+0x20000
command=vehicle+0x30000;data=command+0x100;identity=data+0x100
e.set_u32(vehicle+0x5460,driver);e.set_u32(command+12,data);e.set_u32(identity,0x12345678)
trace=[]
def enter(mu,a,s,u):
    for reg,value in [(UC_X86_REG_ESI,driver),(UC_X86_REG_EBX,vehicle),(UC_X86_REG_EDI,flags)]:mu.reg_write(reg,value)
    e.set_u32(mu.reg_read(UC_X86_REG_ESP)+12,identity)
e.mu.hook_add(UC_HOOK_CODE,enter,begin=0x4cfb51,end=0x4cfb51)
e.mu.hook_add(UC_HOOK_CODE,lambda mu,a,s,u:mu.reg_write(UC_X86_REG_EIP,STUB),begin=0x4cfbf7,end=0x4cfbf7)
def consume(q):
    assert q.ecx==vehicle+0x5570 and q.arg(0)==0 and q.arg(1)==0
    trace.append('consume');q.set_u32(q.ecx,0);return 0
e.hook_thiscall(0x4cfc30,consume,2)
def copy_id(q):q.set_u32(q.ecx,int.from_bytes(q.read(q.arg(0),4),'little'));return q.ecx
e.hook_thiscall(0x41d300,copy_id,1)
def publish(q):
    trace.append(f'publish:{q.arg(0)},{q.arg(1)},{q.arg(2)},{q.arg(3)}');return 0
e.hook_ret(0x447180,publish)
r=random.Random(0x4cfb51);rows=[]
for case in range(400):
    present=case%3!=0;pending=r.choice([0,0,1,2]);kind=r.choice([0,28,29,29,29,30]);subcode=r.choice([0,0,1,-1]);bits=r.choice([0,0x80000000,0x3f800000,0xbf800000,0x7fc00000,0x7f800000,r.getrandbits(32)]);initial=r.getrandbits(8)
    e.set_u32(vehicle+0x5460,other if case%2 else driver);e.write(other+0x40b0,struct.pack("<d",42.0));e.set_u32(vehicle+0x5570,command if present else 0);e.set_u32(vehicle+0x5574,pending)
    e.set_u32(command,kind);e.set_u32(command+4,subcode);e.set_u32(data,bits);e.set_u32(flags,initial);trace.clear()
    e.call(0x4cfb51)
    output=[int.from_bytes(e.read(driver+0x40b0,8),'little'),int.from_bytes(e.read(other+0x40b0,8),'little'),int.from_bytes(e.read(vehicle+0x5570,4),'little')]
    rows.append('\t'.join([','.join(map(str,[int(present),pending,kind,subcode,bits,initial,case%2])),','.join(map(str,output)),';'.join(trace)]))
pathlib.Path(__file__).with_name('golden_ai_obstacle_cached.tsv').write_text('\n'.join(rows)+'\n')
print('400 native obstacle command tails captured')

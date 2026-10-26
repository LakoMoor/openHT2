"""Native mode-dispatch block of 0x4cf620, with decision/action boundaries."""
import pathlib,random
from emu import Emu,OBJ_BASE,STUB
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_ESP,UC_X86_REG_EBP,UC_X86_REG_EBX,UC_X86_REG_EIP
e=Emu();vehicle=OBJ_BASE;flags=vehicle+0x10000;identity=flags+0x100;frame=vehicle+0x20000
drivers=[vehicle+0x30000,vehicle+0x40000];trace=[];enter=0;leave=0;after_flags=0;after_driver=0
e.set_u32(identity,0x12345678);e.set_u32(frame+12,flags)
def begin(mu,a,s,u):
    mu.reg_write(UC_X86_REG_EBP,frame);mu.reg_write(UC_X86_REG_EBX,vehicle)
    e.set_u32(mu.reg_read(UC_X86_REG_ESP)+12,identity)
e.mu.hook_add(UC_HOOK_CODE,begin,begin=0x4cf7e8,end=0x4cf7e8)
e.mu.hook_add(UC_HOOK_CODE,lambda mu,a,s,u:mu.reg_write(UC_X86_REG_EIP,STUB),begin=0x4cf861,end=0x4cf861)
def decision(q,label,value):
    assert q.arg(0)==vehicle;trace.append(label);return value
e.hook_ret(0x4cedd0,lambda q:decision(q,'enter?',enter));e.hook_ret(0x4cee70,lambda q:decision(q,'leave?',leave))
def copy_id(q):q.set_u32(q.ecx,int.from_bytes(q.read(q.arg(0),4),'little'));return q.ecx
e.hook_thiscall(0x41d300,copy_id,1)
def action(q,label,n):
    trace.append(label+':'+','.join(str(q.arg(i)) for i in range(n)))
    e.set_u32(flags,after_flags);e.set_u32(vehicle+0x5460,after_driver);return 0
e.hook_ret(0x4cf080,lambda q:action(q,'enter',4));e.hook_ret(0x4cef60,lambda q:action(q,'leave',3))
r=random.Random(0x4cf7e8);rows=[]
for case in range(400):
    initial=r.getrandbits(32);enter=case%2;leave=(case//2)%2;after_flags=r.getrandbits(32);after_driver=drivers[(case//4)%2]
    old=[r.getrandbits(32) for _ in drivers]
    e.set_u32(flags,initial);e.set_u32(vehicle+0x5460,drivers[0])
    for d,v in zip(drivers,old):e.set_u32(d+0x4078,v)
    trace.clear();e.call(0x4cf7e8)
    output=[int.from_bytes(e.read(a,4),'little') for a in [flags,vehicle+0x5460]+[d+0x4078 for d in drivers]]
    rows.append('\t'.join([','.join(map(str,[initial,enter,leave,after_flags,after_driver]+old)),','.join(map(str,output)),';'.join(trace)]))
pathlib.Path(__file__).with_name('golden_ai_mode_dispatch.tsv').write_text('\n'.join(rows)+'\n')
print('400 native mode-dispatch blocks captured')

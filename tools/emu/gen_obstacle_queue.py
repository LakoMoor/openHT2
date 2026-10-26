"""Native queue-rotation block of 0x4ce300 with deterministic heap hooks."""
import pathlib,random,struct
from emu import Emu,OBJ_BASE,STUB
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_ESP,UC_X86_REG_EBP,UC_X86_REG_EBX,UC_X86_REG_ESI,UC_X86_REG_EIP
e=Emu();vehicle=OBJ_BASE;temporary=vehicle+0x10000;frame=vehicle+0x20000
old0=vehicle+0x30000;old1=vehicle+0x31000;copy=vehicle+0x40000;payload=copy+0x100
trace=[];allocs=0;fail=False;subcode=0;bits=0
def enter(mu,a,s,u):
    mu.reg_write(UC_X86_REG_EBP,frame);mu.reg_write(UC_X86_REG_EBX,vehicle);mu.reg_write(UC_X86_REG_ESI,temporary)
    e.set_u32(frame+12,vehicle);sp=mu.reg_read(UC_X86_REG_ESP)
    for o,v in [(0x10,subcode),(0x14,bits),(0x18,temporary),(0x30,29)]:e.set_u32(sp+o,v)
e.mu.hook_add(UC_HOOK_CODE,enter,begin=0x4ce5df,end=0x4ce5df)
e.mu.hook_add(UC_HOOK_CODE,lambda mu,a,s,u:mu.reg_write(UC_X86_REG_EIP,STUB),begin=0x4ce6a2,end=0x4ce6a2)
def alloc(q):
    global allocs
    allocs+=1;trace.append(f'alloc:{q.arg(0)}');return (0 if fail else copy) if allocs==1 else payload
e.hook_ret(0x632643,alloc);e.hook_ret(0x63266c,lambda q:trace.append(f'free:{q.arg(0)}') or 0)
r=random.Random(0x4ce5df);rows=[]
for case in range(300):
    slots=[old0 if case%2 else 0,old1 if case%3 else 0];old_data=old1+0x100 if case%5 else 0
    subcode=r.getrandbits(32);bits=r.getrandbits(32);fail=case%7==0
    e.set_u32(vehicle+0x5570,slots[0]);e.set_u32(vehicle+0x5574,slots[1]);e.set_u32(old1+12,old_data)
    e.write(copy,b'\xcd'*16);e.write(payload,b'\xcd'*4);e.set_u32(temporary,0);allocs=0;trace.clear();e.call(0x4ce5df)
    output=[int.from_bytes(e.read(a,4),'little') for a in [vehicle+0x5570,vehicle+0x5574,temporary]+[copy+i*4 for i in range(4)]+[payload]]
    rows.append('\t'.join([','.join(map(str,slots+[old_data,subcode,bits,int(fail)])),','.join(map(str,output)),';'.join(trace)]))
pathlib.Path(__file__).with_name('golden_obstacle_queue.tsv').write_text('\n'.join(rows)+'\n')
print('300 native obstacle queue rotations captured')

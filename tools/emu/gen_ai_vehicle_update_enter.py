"""Transition from trajectory to physics through the complete update through native 0x4cf620 with world-service hooks."""
import pathlib,struct
from emu import Emu,OBJ_BASE,STUB
E=Emu();v=OBJ_BASE;human=v+0x10000;driver=v+0x20000;other=v+0x30000;flags=v+0x40000;player=v+0x50000;scenario=v+0x60000;clock=v+0x70000;root=v+0x80000;cmd=v+0x90000;data=cmd+0x100
trace=[]; value=0;swap=False
f=lambda *a:struct.pack('<'+'f'*len(a),*a)
E.write(0x7025f0,f(1,0,0,0,1,0,0,0,1,0,0,0));E.set_u32(0x6d2078,scenario);E.set_u32(0x6f33ac,clock);E.set_u32(0x696ca0,clock);E.set_u32(0x68b990,0x1234)
E.set_u32(v+0x509c,player);E.set_u32(player+0x1c,1);E.set_u32(v+0x13c,root)
def get_flags(q):trace.append('flags');return value
E.hook_ret(0x43f620,get_flags)
def world(q):
 trace.append('world');d=q.arg(0);q.write(d,f(1,0,0,0,1,0,0,0,1,20 if q.ecx==human+0x10 else 0,0,0));return d
E.hook_thiscall(0x57de90,world,1)
def events(q):
 trace.append('events')
 if swap:q.set_u32(v+0x5460,other)
 return 0
E.hook_thiscall(0x5dbec0,events,6)
def consume(q):trace.append('consume');q.set_u32(q.ecx,0);return 0
E.hook_thiscall(0x4cfc30,consume,2)
def copy(q):q.set_u32(q.ecx,int.from_bytes(q.read(q.arg(0),4),'little'));return q.ecx
E.hook_thiscall(0x41d300,copy,1)
def publish(q):trace.append('publish');return 0
E.hook_ret(0x447180,publish)
vtable=v+0xa0000
E.set_u32(v+0x10,vtable);E.set_u32(vtable+0x90,STUB+0x700);E.set_u32(vtable+0x94,STUB+0x720)
E.write(0x702620,f(0,1,0));E.write(0x702650,f(0,0,0))
def snapshot(q):trace.append('speed');q.write(q.arg(1)+0xa0,struct.pack('<d',12.));return 0
def place(q):q.write(q.arg(0),bytes(32));return q.arg(0)
def direction(q):trace.append('direction');q.write(q.arg(0),f(1,0,0));return q.arg(0)
E.hook_ret(0x43fc40,snapshot);E.hook_ret(0x437520,place);E.hook_ret(0x439320,direction);E.hook_ret(0x43f690,lambda q:trace.append('flags_out') or 0)
for addr,label in [(0x54ca60,'mode'),(0x58f2d0,'motion')]:
 E.hook_thiscall(addr,lambda q,label=label:trace.append(label) or 0,1)
for addr,label in [(STUB+0x700,'linear'),(STUB+0x720,'angular')]:
 E.hook_thiscall(addr,lambda q,label=label:trace.append(label+':'+','.join(map(str,struct.unpack('<3I',q.read(q.arg(0),12)))) ) or 0,2)
rows=[]
for i in range(32):
 human_case=False;swap=bool(i&2);value=(0x80 if i&4 else 0)|(4 if i&8 else 0);hour=23. if i&16 else 12.
 E.set_u32(scenario+0x268,v if human_case else human);E.write(clock+0x58,struct.pack('<d',hour));E.set_u32(v+0x5460,driver);E.set_u32(flags,0);E.set_u32(v+0x51b8,0x123);E.set_u32(driver+0x4078,0x234);E.write(driver+0x40b0,struct.pack('<d',42.));E.write(other+0x40b0,struct.pack('<d',43.));E.set_u32(v+0x5570,cmd);E.set_u32(v+0x5574,0);E.set_u32(cmd,29);E.set_u32(cmd+4,0);E.set_u32(cmd+12,data);E.write(data,f(12));trace.clear()
 E.call(0x4cf620,[v,flags],'ii')
 result=[E.eax]+[int.from_bytes(E.read(a,4),'little') for a in [flags,v+0x51b8,driver+0x4078,v+0x5460,v+0x5570]]+[int.from_bytes(E.read(a,8),'little') for a in [driver+0x40b0,other+0x40b0]]
 rows.append('\t'.join([','.join(map(str,[int(human_case),int(swap),value,int(hour)])),','.join(map(str,result)),';'.join(trace)]))
pathlib.Path(__file__).with_name('golden_ai_vehicle_update_enter.tsv').write_text('\n'.join(rows)+'\n');print('32 native vehicle updates captured')

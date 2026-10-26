"""Native command-29 preparation, capture immediately before queue rotation."""
import pathlib,random,struct
from emu import Emu,OBJ_BASE
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_ESP,UC_X86_REG_EIP
e=Emu();command=OBJ_BASE;vehicle=command+0x10000;controller=vehicle+0x10000;plane=controller+0x1000
kind=0;matrix=b'';result=[]
e.write(0x702620,struct.pack('<3f',0,1,0))
e.hook_ret(0x632643,lambda q:plane+0x1000)
def identity(q):q.set_u32(q.ecx,int.from_bytes(q.read(q.arg(0),4),'little'));return q.ecx
e.hook_thiscall(0x41d300,identity,1)
def snapshot(q):q.set_u32(q.arg(1)+8,kind);return 0
e.hook_ret(0x43fc40,snapshot)
def world(q):q.write(q.arg(0),matrix);return q.arg(0)
e.hook_thiscall(0x57de90,world,1)
def capture(mu,a,s,u):
    sp=mu.reg_read(UC_X86_REG_ESP)
    result.append(struct.unpack('<2I',e.read(sp+0x10,8)))
    mu.reg_write(UC_X86_REG_EIP,0x4ce6a2)
e.mu.hook_add(UC_HOOK_CODE,capture,begin=0x4ce5df,end=0x4ce5df)
r=random.Random(0x4ce300);rows=[]
def bits(b):return ','.join(map(str,struct.unpack('<'+'I'*(len(b)//4),b)))
for case in range(500):
    present=case%17!=0;kind=r.choice([1,2,5,5,8]);special=r.choice([0,1]);code=r.choice([-31,-10,-1,0,1,10,11,20,21,30,31,40,41,50]);state=r.getrandbits(3);has_plane=case%7!=0
    matrix=struct.pack('<12f',*[r.uniform(-3,3) for _ in range(12)])
    equation=struct.pack('<4f',*[r.uniform(-5,5) for _ in range(4)])
    e.set_u32(command,29);e.set_u32(command+4,code);e.set_u32(command+12,plane if has_plane else 0)
    e.set_u32(0x696338,controller if present else 0);e.set_u32(controller+0x24,state);e.set_u32(vehicle+0x55a4,special);e.write(plane,equation)
    result.clear();e.call(0x4ce300,[command,vehicle],'ii')
    rows.append('\t'.join([','.join(map(str,[int(present),kind,special,code,state,int(has_plane)])),bits(matrix),bits(equation),','.join(map(str,result[0])) if result else '']))
pathlib.Path(__file__).with_name('golden_obstacle_source.tsv').write_text('\n'.join(rows)+'\n')
print('500 native command-29 preparations captured')

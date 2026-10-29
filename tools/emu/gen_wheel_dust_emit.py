"""Wheel dust emitter 0x4bb050: gate 0x4bb010, pool 0x579380, constructor 0x59d790 and rand replaced."""
import pathlib,random,struct
from emu import Emu,OBJ_BASE
r=random.Random(0x4bb050);e=Emu()
e.u32=lambda a:struct.unpack('<I',e.read(a,4))[0]
clock=OBJ_BASE+0x30000;obj=OBJ_BASE+0x40000;pos=OBJ_BASE+0x100;dirp=OBJ_BASE+0x120
state={'gate':0,'gate_called':0,'rands':[],'used':0,'alloc':0,'alloc_called':0,'ctor':None}
def rand(em):
    v=state['rands'][state['used']];state['used']+=1;return v
def gate(em):
    state['gate_called']=1;return state['gate']
def alloc(em):
    assert em.arg(0)==200 and em.arg(1)==e.u32(0x6f6a9c)
    state['alloc_called']=1;return state['alloc']
def ctor(em):
    sp=em.mu.reg_read(4);args=[em.arg(i) for i in range(9)]
    assert args[0]==clock and args[1]==pos and args[5]==0x694a00 and args[7]==0
    state['ctor']=struct.pack('<I',em.ecx)+struct.pack('<I',args[3])+struct.pack('<I',args[4])+struct.pack('<I',args[6])+struct.pack('<I',args[8])+e.read(args[2],12)
    return em.ecx
e.hook_ret(0x61edde,rand);e.hook_ret(0x4bb010,gate);e.hook_ret(0x579380,alloc);e.hook_thiscall(0x59d790,ctor,9)
rows=[]
for case in range(500):
    inten=r.choice([r.uniform(-1,2),0.0,1.0,0.5,r.uniform(0,1)])
    d=struct.pack('<3f',*[r.uniform(-2,2) for _ in range(3)]);p=struct.pack('<3f',*[r.uniform(-9,9) for _ in range(3)])
    g=r.choice([r.uniform(0,1.5),0.5,0.4999,0.0]);dt=r.choice([r.uniform(0.001,0.2),0.05,0.0,0.01,0.02])
    counter=r.randrange(0,40);gate_v=int(r.random()<0.85);al=obj if r.random()<0.85 else 0
    rands=[r.randrange(32768) for _ in range(8)]
    if case%7==0:rands[0]=r.choice([0,26214,26215,32767])
    g694830=r.randrange(1<<32);g696218=r.randrange(1<<32);g6f6a9c=r.randrange(1<<32)
    e.write(pos,p);e.write(dirp,d);e.set_u32(0x69603c,struct.unpack('<I',struct.pack('<f',g))[0])
    e.set_u32(0x696224,clock);e.write(clock+0xb90,struct.pack('<d',dt));e.set_u32(0x6f6a6c,counter)
    e.set_u32(0x694830,g694830);e.set_u32(0x696218,g696218);e.set_u32(0x6f6a9c,g6f6a9c);e.set_u32(0x6f6a70,0)
    state.update(gate=gate_v,gate_gate=0,gate_called=0,rands=rands,used=0,alloc=al,alloc_called=0,ctor=None)
    e.call(0x4bb050,[pos,dirp,inten,1],'iifi')
    rows.append('\t'.join([struct.pack('<f',inten).hex(),d.hex(),p.hex(),struct.pack('<f',g).hex(),struct.pack('<d',dt).hex(),
        ','.join(map(str,[counter,gate_v,al,g694830,g696218,g6f6a9c])),','.join(map(str,rands)),
        ','.join(map(str,[state['gate_called'],state['used'],state['alloc_called']])),
        state['ctor'].hex() if state['ctor'] else '-',struct.pack('<I',e.u32(0x6f6a70)).hex()]))
pathlib.Path(__file__).with_name('golden_wheel_dust_emit.tsv').write_text('\n'.join(rows)+'\n')
print(len(rows),'cases;',sum(1 for x in rows if '\t-\t' not in x),'constructed;',sum(1 for x in rows if x.split('\t')[7].split(',')[2]=='1'),'allocated')

"""Dust particle constructor 0x59d790 (with 0x59abb0 body); root 0x59a780, rand replaced."""
import pathlib,random,struct,sys
from emu import Emu,OBJ_BASE
r=random.Random(0x59d790);e=Emu()
u32=lambda a:struct.unpack('<I',e.read(a,4))[0]
obj=OBJ_BASE+0x40000;clock=OBJ_BASE+0x30000;pos=OBJ_BASE+0x100;vel=OBJ_BASE+0x120
st={'rands':[],'used':0,'root':None}
def rand(em):
    v=st['rands'][st['used']];st['used']+=1;return v
def root(em):
    st['root']=[em.ecx]+[em.arg(i) for i in range(11)];return em.ecx
e.hook_ret(0x61edde,rand);e.hook_thiscall(0x59a780,root,11)
rows=[]
for case in range(500):
    axes=struct.pack('<6f',*[r.uniform(-1,1) for _ in range(6)])
    p=struct.pack('<3f',*[r.uniform(-9,9) for _ in range(3)])
    kind=r.choice([0,1,2,3,7])
    if case%5==0:v=struct.pack('<3f',*[r.choice([0.,1e-4,-1e-4,0.002]) for _ in range(3)])
    elif case%5==1:v=struct.pack('<3f',*[r.uniform(-.01,.01) for _ in range(3)])
    else:v=struct.pack('<3f',*[r.uniform(-2,2) for _ in range(3)])
    sa=struct.unpack('<I',struct.pack('<f',r.uniform(0.2,5)))[0];sb=struct.unpack('<I',struct.pack('<f',r.uniform(0.2,5)))[0]
    table=r.randrange(1<<32);frames=r.randrange(1,30);life=struct.unpack('<I',struct.pack('<f',r.uniform(0.1,6)))[0]
    mgr=r.randrange(1<<32);counter=r.randrange(0,1000)
    pre=bytes(r.randrange(256) for _ in range(0xc8))
    rands=[r.randrange(32768) for _ in range(3)]
    e.write(0x702630,axes[:12]);e.write(0x702640,axes[12:])
    e.write(pos,p);e.write(vel,v);e.write(obj,pre)
    e.write(0x6f6a68,struct.pack('<I',mgr));e.write(0x6f6a6c,struct.pack('<I',counter))
    st.update(rands=rands,used=0,root=None)
    e.call(0x59d790,[clock,pos,vel,sa,sb,table,frames,kind,life],'iiiiiiiii',this=obj)
    out=e.read(obj,0xc8)
    rows.append('\t'.join([axes.hex(),p.hex(),v.hex(),','.join(map(str,[kind,sa,sb,table,frames,life,mgr,counter,clock])),
        ','.join(map(str,rands)),pre.hex(),','.join(map(str,st['root'])),out.hex(),str(u32(0x6f6a6c)),str(st['used'])]))
if len(sys.argv)>1:
    for x in rows[:3]:print(x.split('\t')[3],x.split('\t')[6])
pathlib.Path(__file__).with_name('golden_dust_ctor.tsv').write_text('\n'.join(rows)+'\n')
print(len(rows),'cases')

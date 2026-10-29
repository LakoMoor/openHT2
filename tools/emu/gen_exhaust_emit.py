"""Exhaust/wheel smoke emitter 0x4ba660; rand, pool 0x579380 and the three constructors replaced."""
import pathlib,random,struct
from emu import Emu,OBJ_BASE
r=random.Random(0x4ba660);e=Emu()
u32=lambda a:struct.unpack('<I',e.read(a,4))[0]
F=lambda *v:struct.pack('<%df'%len(v),*v)
fbits=lambda x:struct.unpack('<I',struct.pack('<f',x))[0]
pos=OBJ_BASE+0x100;dirp=OBJ_BASE+0x120;obj=OBJ_BASE+0x40000;clock=OBJ_BASE+0x30000
st={'rands':[],'used':0,'alloc':[],'allocret':0,'ctor':[]}
def rand(em):
    v=st['rands'][st['used']];st['used']+=1;return v
def alloc(em):
    st['alloc'].append((em.arg(0),em.arg(1)));return st['allocret']
def mk(name,n):
    def f(em):
        st['ctor'].append((name,em.ecx,[em.arg(i) for i in range(n)],e.read(em.arg(1),12),e.read(em.arg(2),12)));return em.ecx
    return f
e.hook_ret(0x61edde,rand);e.hook_ret(0x579380,alloc)
e.hook_thiscall(0x59bed0,mk('A',6),6);e.hook_thiscall(0x59ad70,mk('B',6),6);e.hook_thiscall(0x59b3f0,mk('C',7),7)
rows=[]
for case in range(600):
    p=r.choice([r.uniform(-0.5,2.0),0.4,0.39999998,0.7,0.70000005,0.0,1.0])
    d=F(*[r.uniform(-2,2) for _ in range(3)]);ps=F(*[r.uniform(-9,9) for _ in range(3)])
    g=r.choice([r.uniform(0,1.5),0.0,0.8,0.7999999,1.0])
    rands=[r.choice([r.randrange(32768),0,32767,6553,r.randrange(32768)]) for _ in range(3)]
    ret=obj if r.random()<0.85 else 0
    gl=[r.randrange(1<<32) for _ in range(8)]
    e.write(pos,ps);e.write(dirp,d);e.write(0x696048,F(g))
    for a,v in zip((0x696224,0x696208,0x69620c,0x696210,0x6f6a7c,0x6f6a80,0x6f6a88,0x69620c),gl):e.write(a,struct.pack('<I',v))
    globs=[u32(a) for a in (0x696224,0x696208,0x69620c,0x696210,0x6f6a7c,0x6f6a80,0x6f6a88)]
    st.update(rands=rands,used=0,alloc=[],allocret=ret,ctor=[])
    e.call(0x4ba660,[pos,dirp,fbits(p),1],'iiii')
    cs=';'.join('%s:%d:%s:%s:%s'%(n,t,','.join(map(str,a)),pd.hex(),dd.hex()) for n,t,a,pd,dd in st['ctor']) or '-'
    al=';'.join('%d:%d'%x for x in st['alloc']) or '-'
    rows.append(chr(9).join([F(p).hex(),d.hex(),ps.hex(),F(g).hex(),','.join(map(str,globs)),','.join(map(str,rands)),str(ret),str(st['used']),al,cs]))
pathlib.Path(__file__).with_name('golden_exhaust_emit.tsv').write_text(chr(10).join(rows)+chr(10))
print(len(rows),'cases;',sum(1 for x in rows if not x.endswith('-')),'constructed')

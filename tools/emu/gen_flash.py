"""Flash particle class 0x59d4a0: emitter 0x4baf90, constructor, update 0x59d590 (all three goldens)."""
import pathlib,random,struct
from emu import Emu,OBJ_BASE
r=random.Random(0x59d4a0)
F=lambda *v:struct.pack('<%df'%len(v),*v)
fb=lambda x:struct.unpack('<I',struct.pack('<f',x))[0]
obj=OBJ_BASE+0x40000;clock=OBJ_BASE+0x30000;pos=OBJ_BASE+0x100;geom=OBJ_BASE+0x50000;arr10=OBJ_BASE+0x51000;arr14=OBJ_BASE+0x52000
W=lambda path,rows:pathlib.Path(__file__).with_name(path).write_text(chr(10).join(rows)+chr(10))
# ---- emitter 0x4baf90
e=Emu();u32=lambda a:struct.unpack('<I',e.read(a,4))[0]
st={'alloc':[],'ret':0,'ctor':[]}
def alloc(em):
    st['alloc'].append((em.arg(0),em.arg(1)));return st['ret']
def ctor(em):
    a=[em.arg(i) for i in range(5)];st['ctor'].append((em.ecx,a,e.read(a[1],12)));return em.ecx
e.hook_ret(0x579380,alloc);e.hook_thiscall(0x59d4a0,ctor,5)
rows=[]
for case in range(300):
    g=r.choice([r.uniform(0,1.5),0.5,0.50000006,0.4999,0.0]);p=F(*[r.uniform(-9,9) for _ in range(3)]);size=r.uniform(0.1,3)
    ret=obj if r.random()<0.8 else 0;gl=[r.randrange(1<<32) for _ in range(4)]
    e.write(0x696054,F(g));e.write(pos,p)
    for a,v in zip((0x696224,0x69620c,0x6f6a98),gl):e.write(a,struct.pack('<I',v))
    st.update(alloc=[],ret=ret,ctor=[])
    e.call(0x4baf90,[pos,fb(size)],'ii',this=0)
    al=';'.join('%d:%d'%x for x in st['alloc']) or '-'
    cs=';'.join('%d:%s:%s'%(t,','.join(map(str,a)),pd.hex()) for t,a,pd in st['ctor']) or '-'
    rows.append(chr(9).join([F(g).hex(),p.hex(),str(fb(size)),','.join(map(str,gl[:3])),str(ret),al,cs]))
W('golden_flash_emit.tsv',rows);print(len(rows),'emit cases',sum(1 for x in rows if not x.endswith('-')),'constructed')
# ---- constructor
e=Emu()
e.write(0x702650,F(0,0,0))
rootc={'root':None}
def root(em):
    rootc['root']=[em.arg(i) for i in range(11)];return em.ecx
e.hook_thiscall(0x59a780,root,11)
rows=[]
for case in range(400):
    p=F(*[r.uniform(-9,9) for _ in range(3)]);size=fb(r.uniform(0.1,3));table=r.randrange(1<<32);frames=r.randrange(1,30);mgr=r.randrange(1<<32)
    pre=bytes(r.randrange(256) for _ in range(0xb4))
    e.write(pos,p);e.write(obj,pre);e.write(0x6f6a68,struct.pack('<I',mgr))
    rootc['root']=None
    e.call(0x59d4a0,[clock,pos,size,table,frames],'iiiii',this=obj)
    rows.append(chr(9).join([p.hex(),','.join(map(str,[size,table,frames,mgr,clock])),pre.hex(),','.join(map(str,rootc['root'])),e.read(obj,0xb4).hex()]))
W('golden_flash_ctor.tsv',rows);print(len(rows),'ctor cases')
# ---- update
e=Emu();rec={'n':0,'pos':b''};used=[0];rands=[[]]
def rand(em):
    v=rands[0][used[0]];used[0]+=1;return v
def life(em):
    rec['n']+=1;rec['pos']=e.read(obj+0x28,12);return 0
e.hook_ret(0x61edde,rand);e.hook_thiscall(0x59aa10,life,0)
rows=[]
for case in range(500):
    pre=bytearray(r.randrange(256) for _ in range(0xb4))
    fl=lambda lo,hi:r.uniform(lo,hi)
    lt=fl(0.2,3);age=r.choice([fl(-0.1,lt*1.2),0.0,lt])
    pre[0x58:0x5c]=struct.pack('<I',clock);pre[0x48:0x4c]=struct.pack('<I',geom)
    pre[0x6c:0x70]=struct.pack('<i',r.choice([0,5,10,11,12,13,14,15,30]))
    pre[0x64:0x68]=F(lt);pre[0x68:0x6c]=F(age)
    pre[0x4c:0x58]=F(fl(-9,9),fl(-9,9),fl(-9,9));pre[0x84:0x8c]=F(fl(0.1,6),fl(0.1,6))
    pre[0xa0:0xac]=F(fl(-1,1),fl(-1,1),fl(-1,1));pre[0xb0:0xb4]=struct.pack('<I',r.choice([0,0,1]))
    dt=r.choice([fl(0.001,0.2),0.016,0.0])
    e.write(clock+0xb90,struct.pack('<d',dt));e.write(obj,bytes(pre))
    e.write(geom+0x10,struct.pack('<II',arr10,arr14));e.write(arr10,bytes(16));e.write(arr14,bytes(16))
    rs=[r.randrange(32768) for _ in range(4)];rands[0]=rs;used[0]=0;rec['n']=0
    e.call(0x59d590,[],'',this=obj)
    rows.append(chr(9).join([bytes(pre).hex(),struct.pack('<d',dt).hex(),','.join(map(str,rs)),e.read(obj,0xb4).hex(),e.read(arr10,16).hex(),e.read(arr14,16).hex(),str(used[0]),str(rec['n']),rec['pos'].hex()]))
W('golden_flash_update.tsv',rows);print(len(rows),'update cases')

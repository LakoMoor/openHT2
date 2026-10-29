"""Material loaders of aeffects.cpp: smoke 0x4b8c40, soot 0x4b8ad0, dust 0x4b8f00, fume 0x4b8fb0; lookup 0x4b8720 replaced."""
import pathlib,random,struct
from emu import Emu,OBJ_BASE
r=random.Random(0x4b8c40);e=Emu()
u32=lambda a:struct.unpack('<I',e.read(a,4))[0]
mats=OBJ_BASE+0x70000
st={'avail':0,'names':[],'ptrs':[]}
def lookup(em):
    name=e.read(em.arg(0),64).split(b'\0')[0].decode();st['names'].append(name)
    if len(st['names'])>st['avail']:return 0
    p=mats+0x100*(len(st['names'])-1);st['ptrs'].append(p);return p
errs=[0]
e.hook_ret(0x4b8720,lookup);e.hook_ret(0x60aad0,lambda em:errs.__setitem__(0,errs[0]+1) or 0)
loaders={'smoke':(0x4b8c40,0x695bf0,0x696208),'soot':(0x4b8ad0,0x694998,0x696210),'dust':(0x4b8f00,0x694a00,0x696218),'fume':(0x4b8fb0,0x695c58,0x69620c)}
rows=[]
for case in range(120):
    kind=list(loaders)[case%4];addr,table,count=loaders[kind]
    avail=r.choice([0,1,2,3,9,10,11,12,25,r.randrange(0,26)])
    for i in range(30):
        e.write(mats+0x100*i,bytes(r.randrange(256) for _ in range(0x60)))
    pre=[e.read(mats+0x100*i,0x60) for i in range(30)]
    e.write(table,bytes(r.randrange(256) for _ in range(0x68)))
    e.write(count,struct.pack('<I',r.randrange(1<<32)));e.write(0x696200,struct.pack('<I',0x1234))
    st.update(avail=avail,names=[],ptrs=[]);errs[0]=0
    e.call(addr,[],'')
    tbl=e.read(table,0x68);post=[e.read(mats+0x100*i,0x60) for i in range(30)]
    rows.append(chr(9).join([kind,str(avail),tbl.hex(),str(u32(count)),','.join(st['names']),str(errs[0]),
        ','.join(pre[i].hex() for i in range(min(avail,30))),','.join(post[i].hex() for i in range(min(avail,30)))]))
pathlib.Path(__file__).with_name('golden_effect_materials.tsv').write_text(chr(10).join(rows)+chr(10))
print(len(rows),'cases')

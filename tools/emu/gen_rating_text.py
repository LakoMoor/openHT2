"""Numeric/text draw commands from original 0x574190; no original assets exported."""
import pathlib,random,struct
from emu import Emu,OBJ_BASE
E=Emu(); E.install_crt_hooks(); E.write(OBJ_BASE,b'TEST\0')
E.hook_ret(0x43f620,lambda e:0)
E.hook_thiscall(0x41d300,lambda e:0,1)
state={}; draws=[]
def data(e):
 p=e.arg(1);e.write(p,bytes(192));e.set_u32(p+8,1);e.set_u32(p+0x30,OBJ_BASE)
 e.set_u32(p+0x50,state['members'])
 e.write(p+0x68,struct.pack('<d',state['share']));e.write(p+0x70,struct.pack('<d',state['fleet']))
 return 0
E.hook_ret(0x43fc40,data)
def draw(e,auto=False):
 text=e.read(e.arg(4 if auto else 5),128).split(b'\0')[0].decode('ascii')
 draws.append(f'{e.arg(1)},{e.arg(2)},{-1 if auto else e.arg(3)},{text}');return 0
E.hook_ret(0x5ea1b0,draw)
E.hook_ret(0x5ea170,lambda e:draw(e,True))
r=random.Random(0x574190);rows=[]
for i in range(160):
 x,y=r.randrange(1000),r.randrange(1000)
 state.update(share=r.choice([0.,.125,.5099,1.,1.05]),members=r.randrange(100),fleet=r.randrange(100000)/10)
 draws.clear();E.call(0x574190,[0,x,y,OBJ_BASE+1000],'iiii');assert E.eax==1
 rows.append('\t'.join(map(str,[x,y,state['share'],state['members'],state['fleet'],*draws])))
pathlib.Path(__file__).with_name('golden_rating_text.tsv').write_text('\n'.join(rows)+'\n')
print('160 original rows captured')

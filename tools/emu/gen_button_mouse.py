"""Original button message/state transitions with a supplied geometric hit test."""
import pathlib,random,struct
from emu import Emu,OBJ_BASE,STUB
r=random.Random(0x5bfa80);e=Emu();a=OBJ_BASE;v=a+0x1000;hit=STUB+0x400
state={};e.set_u32(a,v);e.set_u32(v+0x18,hit)
e.hook_thiscall(hit,lambda emu:state['inside'],2)
e.hook_ret(0x5bd590,lambda emu:0)
e.hook_ret(0x5bd560,lambda emu:0)
rows=[]
for i in range(500):
 id=r.choice([0xa3,0xa8,0xac]);disabled=r.randrange(2);bits=r.randrange(16);msg=r.choice([0x201,0x202,0x203,0x200]);inside=r.randrange(2)
 state['inside']=inside;e.set_u32(a+0x1c,id);e.set_u32(a+0x28,disabled);e.set_u32(a+0x11c,bits)
 e.call(0x5bfa80,[msg,0,0],'iii',this=a)
 after=struct.unpack('<I',e.read(a+0x11c,4))[0]
 rows.append(' '.join(map(str,[id,disabled,bits,msg,inside,e.eax,after])))
pathlib.Path(__file__).with_name('golden_button_mouse.txt').write_text('\n'.join(rows)+'\n')
print('500 button transitions captured')
rows=[]
for bits in range(16):
 for captured in range(2):
  for inside in range(2):
   state['inside']=inside;e.set_u32(a+0x11c,bits);e.set_u32(0x702f24,a if captured else 0)
   e.call(0x5bfb30,[0,0],'ii',this=a)
   rows.append(' '.join(map(str,[bits,captured,inside,e.eax,struct.unpack('<I',e.read(a+0x11c,4))[0]])))
pathlib.Path(__file__).with_name('golden_button_move.txt').write_text('\n'.join(rows)+'\n')

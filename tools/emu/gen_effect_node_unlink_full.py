"""Native 0x5ef780, linked-list helper controlled."""
import pathlib,random,struct
from emu import Emu,OBJ_BASE
from unicorn.x86_const import UC_X86_REG_EAX
r=random.Random(0x5ef780);e=Emu();rows=[];node=OBJ_BASE+0x100;parent=OBJ_BASE+0x200;slots=OBJ_BASE+0x300
for case in range(300):
 raw=bytearray(0x400)
 def put(a,v):struct.pack_into('<I',raw,a-OBJ_BASE,v)
 mode=case%5;put(node+4,OBJ_BASE+0x180 if mode==4 else 0);put(node+8,OBJ_BASE+0x1c0);put(node+0x2c,0 if mode==0 else parent);put(node+0x24,r.getrandbits(32));put(parent+0x30,slots);put(parent+0x34,3)
 for i in range(3):put(slots+i*4,node if mode in [2,3] and i==mode-2 else OBJ_BASE+0x40)
 e.write(OBJ_BASE,raw);e.call(0x5ef780,this=node);result=e.mu.reg_read(UC_X86_REG_EAX)
 rows.append(raw.hex()+'\t'+e.read(OBJ_BASE,0x400).hex()+'\t'+str(result))
pathlib.Path(__file__).with_name('golden_effect_node_unlink_full.tsv').write_text('\n'.join(rows)+'\n');print('300 node unlinks captured')

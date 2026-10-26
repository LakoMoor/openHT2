"""Capture steering matrix of 0x4decc0; skip later cabin-motion code only."""
import pathlib,random,struct
from emu import Emu,OBJ_BASE
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_ESP,UC_X86_REG_EIP
e=Emu();r=random.Random(0x4decc0);c=OBJ_BASE;s=c+0x20000;vehicle=s+0x10000;car=vehicle+0x10000;rows=[];out=[]
e.set_u32(0x6cec68,1);e.set_u32(0x6d2078,s);e.set_u32(s+0x268,vehicle);e.set_u32(vehicle+0x5460,car)
def capture(mu,address,size,user):
 out.append(struct.unpack('<12I',e.read(mu.reg_read(UC_X86_REG_ESP)+0x98,48)))
 mu.reg_write(UC_X86_REG_EIP,0x4df2f1)
e.mu.hook_add(UC_HOOK_CODE,capture,begin=0x4ded9e,end=0x4ded9e)
for i in range(240):
 steering=r.uniform(-1,1) if i>2 else [-1,0,1][i];raw=struct.pack('<f',steering);e.write(car+0x29e0,raw)
 base=struct.pack('<12f',*[r.uniform(-2,2) for _ in range(12)]);e.write(c+0x230,base)
 out.clear();e.call(0x4decc0,this=c);assert len(out)==1
 rows.append(str(struct.unpack('<I',raw)[0])+'\t'+','.join(map(str,struct.unpack('<12I',base)))+'\t'+','.join(map(str,out[0])))
pathlib.Path(__file__).with_name('golden_cabin_steering.tsv').write_text('\n'.join(rows)+'\n');print('240 original cabin steering matrices captured')

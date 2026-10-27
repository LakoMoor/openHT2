"""Original AI prelude through lights; native human return and trajectory check."""
import pathlib,random,struct
from emu import Emu,OBJ_BASE
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_EIP
r=random.Random(0x4cf620);e=Emu();v=OBJ_BASE;p=v+0x10000;scenario=p+4096;clock=scenario+4096;flags=clock+4096;trace=[];pf=0

def get_flags(q):trace.append('flags');return pf
e.hook_ret(0x43f620,get_flags)
e.hook_ret(0x5d2ba0,lambda q:(trace.append('lost') or 0))
e.mu.hook_add(UC_HOOK_CODE,lambda mu,a,size,u:mu.reg_write(UC_X86_REG_EIP,0x4cfbf7),begin=0x4cf746,end=0x4cf746)
rows=[]
for case in range(500):
 human=case%5==0;fallback=case%3==0;equal=case%4==0;pf=r.getrandbits(32);initial=r.getrandbits(32);old=r.getrandbits(32)
 hour=r.choice([6.,22.,5.999999,22.000001,float('nan'),r.uniform(-2,26)])
 e.set_u32(v+0x509c,p);e.set_u32(0x68b990,p if fallback else 0);e.set_u32(0x6d2078,scenario);e.set_u32(scenario+0x268,v if human else 0)
 e.set_u32(p+0x18,123);e.set_u32(p+0x1c,123 if equal else 124);e.set_u32(0x6f33ac,clock);e.write(clock+0x58,struct.pack('<d',hour));e.set_u32(flags,initial);e.set_u32(v+0x51b8,old)
 e.set_u32(0x696ca0,scenario);trace.clear();e.call(0x4cf620,[v,flags],'ii')
 rows.append('\t'.join(map(str,[int(human),int(fallback),int(equal),pf,initial,old,struct.pack('<d',hour).hex(),int.from_bytes(e.read(flags,4),'little'),int.from_bytes(e.read(v+0x51b8,4),'little'),';'.join(trace)])))
pathlib.Path(__file__).with_name('golden_vehicle_update_start.tsv').write_text('\n'.join(rows)+'\n');print('500 native AI update preludes captured')

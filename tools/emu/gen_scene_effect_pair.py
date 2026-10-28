"""First 0x551970 effect transform, real matrix calculation, effect sink replaced."""
import pathlib,random,struct
from emu import Emu,OBJ_BASE,TECH_BASE
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_EIP,UC_X86_REG_ESP
r=random.Random(0x551a47);e=Emu();cfg=OBJ_BASE+0x10000;result=[[]];rows=[]
e.set_u32(0x6cec6c,cfg);e.set_u32(OBJ_BASE+0x51a4,0x102);e.set_u32(OBJ_BASE+0x27b4,0x400)
def effect(em):
 assert em.arg(2)==0 and em.arg(3)==1
 result[0].append(e.read(em.arg(0),12)+e.read(em.arg(1),12));return 0
e.hook_ret(0x4bac90,effect)
def stop(mu,a,s,u):
 mu.reg_write(UC_X86_REG_ESP,mu.reg_read(UC_X86_REG_ESP)+8);mu.reg_write(UC_X86_REG_EIP,0x552933)

e.mu.hook_add(UC_HOOK_CODE,stop,begin=0x551cf3,end=0x551cf3)
for case in range(400):
 data=struct.pack('<24f',*[r.uniform(-3,3) for _ in range(24)]);params=struct.pack('<12f',*[r.uniform(-3,3) for _ in range(12)])
 flags=0x100|(2 if case%4!=0 else 0);state=0x400 if case%4!=1 else 0;ident=case%5
 params=bytearray(params);params[8:12]=struct.pack('<f',[0.,9.999,10.,-10.,11.][case%5])
 params[32:36]=struct.pack('<f',[0.,10.,-9.999,-10.][case%4])
 e.set_u32(OBJ_BASE+0x51a4,flags);e.set_u32(OBJ_BASE+0x27b4,state);e.set_u32(OBJ_BASE+0x28b4,ident)
 e.write(OBJ_BASE+0x2c4,data[:48]);e.write(OBJ_BASE+0x204,data[48:]);e.write(0x697fa8+ident*0x970,params);result[0]=[];e.call(0x551970,this=OBJ_BASE)

 rows.append(data.hex()+'\t'+params.hex()+'\t'+(b''.join(result[0]).hex() if result[0] else '-')+'\t'+','.join(map(str,[flags,state,ident])))
pathlib.Path(__file__).with_name('golden_scene_effect_pair.tsv').write_text('\n'.join(rows)+'\n');print('400 native effect dispatches captured')

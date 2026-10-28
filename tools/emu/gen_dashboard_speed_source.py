"""Native 0x53d601 speed source selection, without speed override."""
import pathlib,random,struct
from emu import Emu,OBJ_BASE,STUB
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_EBX,UC_X86_REG_EIP,UC_X86_REG_ESP
E=Emu();viewer=OBJ_BASE;scene=viewer+0x10000;car=viewer+0x20000;aux=viewer+0x30000
E.set_u32(0x6d20ec,0);E.set_u32(viewer+0x268,scene)
def start(mu,a,s,u):mu.reg_write(UC_X86_REG_EBX,viewer)
def stop(mu,a,s,u):mu.reg_write(UC_X86_REG_EIP,STUB)
E.mu.hook_add(UC_HOOK_CODE,start,begin=0x53d601,end=0x53d601);E.mu.hook_add(UC_HOOK_CODE,stop,begin=0x53d69a,end=0x53d69a)
r=random.Random(0x53d601);rows=[]
for case in range(400):
 mode=case%4;values=[struct.unpack('<f',struct.pack('<f',r.uniform(-90,90)))[0] for _ in range(9)]
 E.set_u32(scene+0x5460,0 if mode==0 else car);E.set_u32(scene+0x5464,aux if mode==3 else 0);E.set_u32(car+0x3344,1 if mode==2 else 0);E.set_u32(car+0x3464,case%2);E.write(scene+0x51e4,struct.pack('<f',values[0]));E.write(car+0x3328,struct.pack('<f',values[1]));E.write(car+0x3330,struct.pack('<f',values[2]));E.write(scene+0xcf4,struct.pack('<3f',*values[3:6]));E.write(aux+0x64,struct.pack('<3f',*values[6:9]));E.call(0x53d601);sp=E.mu.reg_read(UC_X86_REG_ESP)
 rows.append(','.join(map(str,[mode,case%2]+list(struct.unpack('<9I',struct.pack('<9f',*values)))))+'\t'+str(int.from_bytes(E.read(sp+0x10,4),'little')))
pathlib.Path(__file__).with_name('golden_dashboard_speed_source.tsv').write_text('\n'.join(rows)+'\n');print('400 native dashboard speed selections captured')

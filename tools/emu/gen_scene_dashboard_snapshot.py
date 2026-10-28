"""Native snapshot copy block 0x54d714..0x54d80c (finite float inputs)."""
import pathlib,random,struct
from emu import Emu,OBJ_BASE,STUB
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_ESI,UC_X86_REG_EIP
pairs=[(0x2814,0x51e4),(0x288c,0x51fc),(0x2818,0x51e8),(0x2818,0x51ec),(0x2890,0x520c),(0x2810,0x51f4),(0x28b0,0x5200),(0x2898,0x5214),(0x28a0,0x5218),(0x28ac,0x522c),(0x28a8,0x5228)]
e=Emu();r=random.Random(0x54d714);rows=[]
e.mu.hook_add(UC_HOOK_CODE,lambda mu,a,s,u:mu.reg_write(UC_X86_REG_ESI,OBJ_BASE),begin=0x54d714,end=0x54d714)
e.mu.hook_add(UC_HOOK_CODE,lambda mu,a,s,u:mu.reg_write(UC_X86_REG_EIP,STUB),begin=0x54d80c,end=0x54d80c)
for case in range(300):
 values=[r.getrandbits(32) for _ in pairs];values[2]=values[3]=struct.unpack('<I',struct.pack('<f',r.uniform(-1000,1000)))[0];gear=r.getrandbits(32);flags=(1<<case) if case<32 else r.getrandbits(32)
 if case<8:values[2]=values[3]=[0,0x80000000,0x7f800000,0xff800000,0x7fc12345,0x7f812345,1,0x80000001][case]
 for (src,_),value in zip(pairs,values):e.set_u32(OBJ_BASE+src,value)
 e.set_u32(OBJ_BASE+0x27b4,flags);e.set_u32(OBJ_BASE+0x2820,gear);e.call(0x54d714)
 out=[int.from_bytes(e.read(OBJ_BASE+dst,4),'little') for _,dst in pairs]+[int.from_bytes(e.read(OBJ_BASE+0x5204,4),'little')]
 out += [int.from_bytes(e.read(OBJ_BASE+dst,4),'little') for dst in [0x5208,0x51b4,0x51b8,0x5178,0x517c,0x5224,0x51c0]]
 rows.append(','.join(map(str,values+[gear,flags]))+'\t'+','.join(map(str,out)))
pathlib.Path(__file__).with_name('golden_scene_dashboard_snapshot.tsv').write_text('\n'.join(rows)+'\n')
print('300 native snapshot copies captured')

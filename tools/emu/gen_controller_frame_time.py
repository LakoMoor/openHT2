"""Whole original begin-frame handler, observing timing fields with rendering hooks."""
import pathlib,struct,random
from emu import Emu,OBJ_BASE
r=random.Random(0x5de600);e=Emu();c=OBJ_BASE;config=c+0x10000;scene=c+0x20000;player=c+0x30000;table=c+0x40000
for o,p in [(0x1d0,scene),(0x1d4,player)]:e.set_u32(c+o,p)
e.set_u32(0x721750,config);e.set_u32(player+0x64,table)
e.hook_thiscall(0x5ae9b0,lambda q:0,1);e.hook_thiscall(0x5ba2c0,lambda q:0,1)
e.write(0x6037e0,b'\xdd\x05'+struct.pack('<I',c+0x50000)+b'\xc3');rows=[]
for i in range(400):
 now=r.uniform(0,10000000);last=now*0.001+r.choice([-200.,-1.,-.3,-.016,0.,1.]);pause=r.choice([0.,.1,5.]);overhead=r.choice([0.,5.,1000.]);total=r.uniform(-100,10000);fixed=r.choice([-1,0,0,16,50,100001])
 for o,v in [(0x250,last),(0x258,pause),(0xb98,total)]:e.write(c+o,struct.pack('<d',v))
 e.set_u32(player+0x8c,i%2);e.set_u32(player+0x94,i);e.set_u32(c+0x220,0);e.set_u32(c+0xc14,0);e.set_u32(c+0xbcc,123);
 for k in range(4):e.set_u32(c+0xbf8+k*4,100+k)
 e.set_u32(c+0xbd8,fixed);e.write(config+0x168,struct.pack('<d',overhead));e.write(c+0x50000,struct.pack('<d',now));e.call(0x5de600,this=c)
 bits=[struct.unpack('<Q',e.read(c+o,8))[0] for o in [0x250,0x258,0xb90,0xb98,0xba0]]
 rows.append(','.join(map(repr,[now,last,pause,overhead,total]))+f'\t{fixed}\t'+','.join(map(str,bits))+'\t'+','.join(str(struct.unpack('<I',e.read(a,4))[0]) for a in [scene+0x2130,scene+0x2134,scene+0x2138,scene+0x213c,scene+0x2104,player+0x94,c+0x220,c+0xc14,0x702e8c,0x702e90]))
pathlib.Path(__file__).with_name('golden_controller_frame_time.tsv').write_text('\n'.join(rows)+'\n');print('400 frame timing states captured')

"""Native preferred-lane decision, player-flags boundary hooked."""
import pathlib,random,struct
from emu import Emu,OBJ_BASE
e=Emu();d=OBJ_BASE;wrapper=d+0x10000;player=wrapper+0x6000;e.set_u32(d+0x40c0,wrapper);e.set_u32(wrapper+0x509c,player)
flags=0;e.hook_ret(0x43f620,lambda q:flags)
r=random.Random(0x4d4620);rows=[]
for case in range(600):
 flags=r.choice([0,0x4000,0x80000,0x8000000,0x84000]);role=r.choice([-2147483648,-5,-2,-1,0,1,2,3,4,5,6,7,2147483647]);width=r.choice([0.,1.75,3.5,7.,14.,-3.5,r.uniform(0,50)])
 preferred=r.randrange(-3,9);old=r.uniform(-100,100);e.set_u32(player+0x2d8,role);e.set_u32(d+0x407c,preferred);e.write(d+0x40a8,struct.pack('<d',old));e.call(0x4d4620,[width],'d',this=d)
 rows.append('\t'.join(map(str,[flags,role,struct.pack('<d',width).hex(),preferred,struct.pack('<d',old).hex(),e.eax,e.read(d+0x40a8,8).hex()])))
pathlib.Path(__file__).with_name('golden_preferred_lane.tsv').write_text('\n'.join(rows)+'\n');print('600 native lane decisions captured')

"""Actual 0x447180 writes, with successful identity validation boundary."""
import pathlib,random,struct
from emu import Emu,OBJ_BASE
e=Emu();player=OBJ_BASE;e.hook_ret(0x4566c0,lambda q:0)
r=random.Random(0x447180);rows=[]
for case in range(300):
    clear=case%2;bits=r.getrandbits(64);before=r.randbytes(24)
    e.write(player+0x3d0,before)
    e.call(0x447180,[player,clear,bits&0xffffffff,bits>>32],'iiii')
    rows.append('\t'.join([str(clear),str(bits),before.hex(),e.read(player+0x3d0,24).hex()]))
pathlib.Path(__file__).with_name('golden_obstacle_publish.tsv').write_text('\n'.join(rows)+'\n')
print('300 native obstacle publications captured')

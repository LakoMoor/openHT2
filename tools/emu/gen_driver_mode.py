"""Original 0x4fc930 dispatch, capturing its three existing setup boundaries."""
import pathlib,random
from emu import Emu,OBJ_BASE
e=Emu();driver=OBJ_BASE;trace=[];r=random.Random(0x4fc930)
def capture(q,label,n):
    assert q.ecx==driver
    trace.append(label+':'+','.join(str(q.arg(i)) for i in range(n)))
    # Prove final mode is assigned after all setup callbacks.
    q.set_u32(driver+0x2638,0x12340000+n)
    return 0
for address,label,n in [(0x4f1240,'place',3),(0x4f1dd0,'linear',1),(0x4f1ef0,'angular',1)]:
    e.hook_thiscall(address,lambda q,label=label,n=n:capture(q,label,n),n)
rows=[]
for case in range(400):
    mode=r.choice([-2147483648,-1,0,1,2,3,4,2147483647]);old=r.choice([0,1,2,3,r.getrandbits(32)])
    args=[r.getrandbits(32) for _ in range(4)]
    e.set_u32(driver+0x2638,old);trace.clear();e.call(0x4fc930,[mode]+args,'iiiii',this=driver)
    result=int.from_bytes(e.read(driver+0x2638,4),'little')
    rows.append('\t'.join([','.join(map(str,[mode,old]+args)),str(result),';'.join(trace)]))
pathlib.Path(__file__).with_name('golden_driver_mode.tsv').write_text('\n'.join(rows)+'\n')
print('400 native driver activation dispatches captured')

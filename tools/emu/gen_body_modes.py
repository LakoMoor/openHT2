"""Native MovingBox mode setters; diagnostic calls captured for invalid detail."""
import pathlib,random
from emu import Emu,OBJ_BASE
e=Emu();body=OBJ_BASE;r=random.Random(0x58f230);rows=[];errors=[]
e.hook_ret(0x4e260c,lambda q:errors.append(1) or 0)
for case in range(400):
    value=r.choice([-2147483648,-1,0,1,2,3,4,8,16,2147483647])
    initial=[r.getrandbits(32) for _ in range(3)]
    for address in [0x58f230,0x58f290,0x58f2d0]:
        for offset,v in zip([0x4ac,0xf88,0xf8c],initial):e.set_u32(body+offset,v)
        errors.clear();e.call(address,[value],'i',this=body)
        output=[int.from_bytes(e.read(body+offset,4),'little') for offset in [0x4ac,0xf88,0xf8c]]
        rows.append('\t'.join(map(str,[address,value]+initial+output+[len(errors)])))
pathlib.Path(__file__).with_name('golden_body_modes.tsv').write_text('\n'.join(rows)+'\n')
print('1200 native body mode calls captured')

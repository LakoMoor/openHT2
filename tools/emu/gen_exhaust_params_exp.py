"""0x549ff0 modes 1 and 2 (pure arithmetic): outputs a, b and the returned st0."""
import pathlib,random,struct
from emu import Emu,OBJ_BASE
r=random.Random(0x549ff0);e=Emu()
this=OBJ_BASE;cfg=OBJ_BASE+0x100;oa=OBJ_BASE+0x200;ob=OBJ_BASE+0x204
rows=[]
for case in range(400):
    mode=[3,4,0,5][case%4]
    x=r.choice([r.uniform(-6,6),0.0,5.0,20.0,-5.0,-20.0,1e-30])
    e.set_u32(this+0x14,cfg);e.write(cfg+8,struct.pack('<f',x));e.set_u32(cfg+0xc,mode)
    e.write(oa,bytes(8))
    ret=e.call(0x549ff0,[oa,ob],'ii',this=this,ret='d') if False else e.call(0x549ff0,[oa,ob],'ii',this=this)
    rows.append('\t'.join([str(mode),struct.pack('<f',x).hex(),e.read(oa,4).hex(),e.read(ob,4).hex(),struct.pack('<d',ret).hex()]))
pathlib.Path(__file__).with_name('golden_exhaust_params_exp.tsv').write_text('\n'.join(rows)+'\n')
print(len(rows))

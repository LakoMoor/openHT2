"""Scheduling gate of 0x551970 (0x551970..0x5519f5): does the function go on to the basis step 0x5aba40?"""
import pathlib,random,struct
from emu import Emu,OBJ_BASE
r=random.Random(0x551970);e=Emu();cfg=OBJ_BASE+0x10000;clk=OBJ_BASE+0x20000
e.set_u32(0x6cec6c,cfg);e.set_u32(0x6ef1fc,clk)
hit=[0]
def basis(em):
    hit[0]+=1;return 0
e.hook_ret(0x5aba40,basis)
for a in (0x4bb050,0x4bac90,0x4baf90,0x4ba660,0x61edde):e.hook_ret(a,lambda em:0)
e.hook_ret(0x549bf0,lambda em:0)
rows=[]
for case in range(500):
    flags=r.choice([0,0x100,0x100,0x100,0x1ff,0xff])
    c514=r.choice([0,1,2,3,-1,5]);c524=r.choice([0,1,2,3])
    dt=r.choice([0.0,-0.0,float('nan'),1/60,0.05,-0.02,1.0,2.5,1e-9,0.9999,r.uniform(0.001,0.5)])
    counter=r.choice([0,1,2,60,r.randrange(-100,10000),r.randrange(0,1000)])
    e.set_u32(OBJ_BASE+0x51a4,flags);e.set_u32(cfg+0x514,c514&0xffffffff);e.set_u32(cfg+0x524,c524)
    e.write(clk+0x1f18,struct.pack('<d',dt));e.set_u32(clk+0xbb0,counter&0xffffffff)
    e.set_u32(OBJ_BASE+0x27b4,0);e.set_u32(OBJ_BASE+0x28b4,0);e.set_u32(OBJ_BASE+0x5460,0);e.set_u32(OBJ_BASE+0x28bc,0);e.write(OBJ_BASE+0x2821,bytes([0]))
    e.set_u32(OBJ_BASE+0x5454,cfg+0x100)
    hit[0]=0;e.call(0x551970,this=OBJ_BASE)
    rows.append('\t'.join([str(flags),str(c514),str(c524),struct.pack('<d',dt).hex(),str(counter),str(1 if hit[0] else 0)]))
pathlib.Path(__file__).with_name('golden_effect_schedule.tsv').write_text('\n'.join(rows)+'\n')
print(len(rows),'cases;',sum(1 for x in rows if x.endswith('\t1')),'pass the gate')

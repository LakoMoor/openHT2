"""Full original 0x557090, including its native 0x549bf0 query; no hooks."""
import pathlib,random,struct
from emu import Emu,OBJ_BASE

e=Emu();vehicle=OBJ_BASE;parts=vehicle+0x10000;state=parts+0x10000
e.set_u32(vehicle+0x2540,parts);e.set_u32(vehicle+0x5454,state)
r=random.Random(0x557090);rows=[]
for case in range(600):
    enabled=r.choice([-1,0,1,2,2147483647])
    health=r.choice([-1.,0.,0.15,0.3,0.49999997,0.5,1.,float('nan'),float('inf')])
    stopped=r.choice([0,0,0,1,2])
    pairs=[]
    for i in range(9):
        numerator=r.choice([0,0,r.randrange(-100,101),-2147483648,2147483647])
        denominator=r.choice([0,1,10,100,-100,2147483647])
        if case<90:
            numerator=(case%10)*10 if i==case//10 else 0
            denominator=100
            enabled=1;health=1.;stopped=0
        e.set_u32(parts+i*20+4,numerator);e.set_u32(parts+i*20+8,denominator)
        pairs.extend([numerator&0xffffffff,denominator&0xffffffff])
    raw=struct.pack('<f',health)
    e.set_u32(vehicle+0x5080,enabled);e.write(vehicle+0x224,raw);e.set_u32(state+12,stopped)
    result=e.call(0x557090,this=vehicle)
    inputs=[enabled&0xffffffff,int.from_bytes(raw,'little'),stopped]+pairs
    rows.append(','.join(map(str,inputs))+'\t'+str(int.from_bytes(struct.pack('<f',result),'little'))+'\t'+str(int.from_bytes(struct.pack('<d',result),'little')))
pathlib.Path(__file__).with_name('golden_vehicle_condition.tsv').write_text('\n'.join(rows)+'\n')
print('600 complete original vehicle-condition calls captured')

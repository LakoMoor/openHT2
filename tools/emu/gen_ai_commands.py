"""Native 0x4cfc30, deterministic allocation/free boundaries."""
import pathlib,random,struct
from emu import Emu,OBJ_BASE
e=Emu();owner=OBJ_BASE;old=owner+0x1000;source=owner+0x2000;input_data=source+0x100
copy=owner+0x3000;payload=owner+0x4000;old_payload=old+0x100
trace=[];allocs=0;outer_fail=False;inner_fail=False
def allocate(q):
    global allocs
    trace.append(f'alloc:{q.arg(0)}');allocs+=1
    return (0 if outer_fail else copy) if allocs==1 else (0 if inner_fail else payload)
e.hook_ret(0x632643,allocate);e.hook_ret(0x63266c,lambda q:trace.append(f'free:{q.arg(0)}') or 0)
r=random.Random(0x4cfc30);rows=[]
for case in range(400):
    slot=r.choice([0,0,1,1,2,0xffffffff]);present=case%3!=0;data_present=case%4!=0;new=case%5!=0
    outer_fail=case%11==0;count=r.choice([0,1,2,8,0x40000000,0x40000001]);inner_fail=count==0 and case%2==0
    header=[r.getrandbits(32),r.getrandbits(32),count,input_data];words=[r.getrandbits(32) for _ in range(8)]
    slots=[old if present else 0,old if present else 0]
    e.write(owner,struct.pack('<2I',*slots));e.set_u32(old+12,old_payload if data_present else 0)
    e.write(source,struct.pack('<4I',*header));e.write(input_data,struct.pack('<8I',*words))
    e.write(copy,b'\xcd'*16);e.write(payload,b'\xcd'*32);trace.clear();allocs=0
    e.call(0x4cfc30,[source if new else 0,slot],'ii',this=owner)
    result=list(struct.unpack('<2I',e.read(owner,8)))+list(struct.unpack('<4I',e.read(copy,16)))+list(struct.unpack('<8I',e.read(payload,32)))
    rows.append('\t'.join([','.join(map(str,[slot,int(present),int(data_present),int(new),int(outer_fail),int(inner_fail)]+header[:3])),','.join(map(str,words)),','.join(map(str,result)),';'.join(trace)]))
pathlib.Path(__file__).with_name('golden_ai_commands.tsv').write_text('\n'.join(rows)+'\n')
print('400 native command replacements captured')

"""Native 0x4464c0 removal of one player ID per vehicle list."""
import pathlib,random
from emu import Emu,OBJ_BASE
E=Emu();obj=OBJ_BASE;lists=obj+0x1000;data=obj+0x10000;target=0x12345678
E.set_u32(0x71e814,0);E.hook_thiscall(0x4018b0,lambda q:0,1)
def assign(q):
 src=q.arg(0);E.write(q.ecx,E.read(src,16));return q.ecx
E.hook_thiscall(0x42bfe0,assign,1)
E.hook_thiscall(0x44b090,lambda q:0,0)
r=random.Random(0x4464c0);rows=[]
for i in range(200):
 count=i%5;null=i%17==0;vals=[];E.set_u32(obj+0x1c,lists);E.set_u32(obj+0x20,lists+count*0x90)
 for j in range(4):
  length=r.randrange(6);base=data+j*0x100;E.set_u32(lists+j*0x90+0x78,base);E.set_u32(lists+j*0x90+0x7c,base+length*16)
  row=[r.choice([target,target,0x77777777,0x88888888]) for _ in range(length)];vals.append(row)
  for k in range(5):
   for t in range(4):E.set_u32(base+k*16+t*4,(row[k] if t==0 else ((j+1)*1000+k*10+t)) if k<length else 0)
 E.call(0x4464c0,[0 if null else obj,target],'ii')
 out=[]
 for j in range(4):
  base=data+j*0x100;length=(int.from_bytes(E.read(lists+j*0x90+0x7c,4),'little')-base)//16
  out.append(','.join(str(int.from_bytes(E.read(base+k*16+t*4,4),'little')) for k in range(length) for t in range(4)))
 rows.append('\t'.join([str(count),str(int(null)),*[','.join(map(str,row)) for row in vals],*out]))
pathlib.Path(__file__).with_name('golden_clear_vehicle_player.tsv').write_text('\n'.join(rows)+'\n');print('200 native vehicle player-reference removals captured')

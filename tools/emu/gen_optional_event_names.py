"""Original pending-name resolution with empty registry, no hooks."""
from emu import Emu,OBJ_BASE
import pathlib
q=Emu();reg=OBJ_BASE;record=reg+256;name=reg+512;out=reg+768;table=reg+4096;rows=[]
for label in ['$$world','$$People','$$','$$Missing','world','Missing','$world','x$$world']:
 q.write(reg,b'\0'*8192);q.write(name,label.encode()+b'\0');q.set_u32(reg,record);q.set_u32(reg+8,table);q.set_u32(record+12,name);q.set_u32(record+16,out);q.set_u32(out,0x12345678)
 q.call(0x5d27c0,this=reg)
 rows.append(label+'\t'+str(int(q.eax==0))+'\t'+str(int.from_bytes(q.read(out,4),'little')))
pathlib.Path(__file__).with_name('golden_optional_event_names.tsv').write_text('\n'.join(rows)+'\n')
print('8 unhooked registry resolutions captured')

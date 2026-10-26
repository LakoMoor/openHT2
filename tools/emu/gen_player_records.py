"""Native 0x476930/0x4769e0 record copies, no replacement hooks."""
import pathlib,random,struct
from emu import Emu,OBJ_BASE
e=Emu();player=OBJ_BASE;owner=player+0x1000;data=player+0x2000;external=data+0x1000
r=random.Random(0x476930);rows=[]
for case in range(300):
    present=case%7!=0;write=case%2;buffer=r.choice([external,data,data+4,data-4,data+16,data-16])
    raw=r.randbytes(352);other=r.randbytes(304)
    e.write(data-16,raw);e.write(external,other)
    e.set_u32(player+0x204,owner if present else 0);e.set_u32(owner+0x10,data)
    e.call(0x4769e0 if write else 0x476930,[player,buffer],'ii')
    rows.append('\t'.join([str(int(present)),str(write),str(buffer),raw.hex(),other.hex(),e.read(data-16,352).hex(),e.read(external,304).hex(),str(e.eax)]))
pathlib.Path(__file__).with_name('golden_player_records.tsv').write_text('\n'.join(rows)+'\n')
print('300 native bidirectional player record copies captured')

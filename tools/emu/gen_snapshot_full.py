"""Full 0x541ad0 on valid synthetic extended snapshots."""
import pathlib,random,struct
from emu import Emu,OBJ_BASE
r=random.Random(0x541ad0);e=Emu();rows=[]
for case in range(400):
 n=case%9;base=24+3*n;size=base+10 if case%2 else base
 data=bytearray(r.getrandbits(8) for _ in range(0x150))
 for offset,value in [(4,base),(8,size),(0x10,n)]:data[offset:offset+4]=struct.pack('<I',value)
 data[0x14:0x16]=struct.pack('<H',size);data[0x18]=(data[0x18]|1) if case%2 else (data[0x18]&254);data[0xc4:0xc8]=struct.pack('<f',r.uniform(-100,100))
 e.write(OBJ_BASE,data);e.call(0x541ad0,this=OBJ_BASE)
 rows.append(data.hex()+'\t'+e.read(OBJ_BASE+0x50,0xfc).hex())
pathlib.Path(__file__).with_name('golden_snapshot_full.tsv').write_text('\n'.join(rows)+'\n')
print('400 full native snapshots captured')

"""0x54d690 with real snapshot decoder; virtual/geometry calls substituted."""
import pathlib
from emu import Emu,OBJ_BASE
root=pathlib.Path(__file__).parent;e=Emu();rows=[]
e.hook_thiscall(0x541000,lambda em:0,1)
e.hook_thiscall(0x551970,lambda em:0,0)
e.hook_thiscall(0x586df0,lambda em:0,0)
for line in (root/'golden_snapshot_full.tsv').read_text().splitlines():
 raw=bytes.fromhex(line.split('\t')[0]);e.write(OBJ_BASE,bytes(0x6000));e.write(OBJ_BASE+0x2764,raw)
 e.set_u32(OBJ_BASE+0x10,OBJ_BASE+0x5800);e.set_u32(OBJ_BASE+0x5800+0x9c,0x541000)
 e.call(0x54d690,this=OBJ_BASE)
 rows.append(raw.hex()+'\t'+e.read(OBJ_BASE+0x51b4,0x7c).hex())
(root/'golden_scene_restore.tsv').write_text('\n'.join(rows)+'\n');print('400 scene restores with native decoder captured')

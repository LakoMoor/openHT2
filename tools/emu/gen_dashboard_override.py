"""Capture native 0x4ff5b0 for the existing numerical curve test inputs."""
import pathlib,re,struct
from emu import Emu,OBJ_BASE
root=pathlib.Path(__file__).resolve().parents[2]
source=(root/'crates/ht2-sim/tests/golden_curves.rs').read_text()
inputs=re.findall(r'm.set_u32\(car\+0x8cc,(\d+)\); m.set_u32\(scene\+0x5204,(\d+)\); m.set_f32\(car\+0x2640,([^f]+)f32\)',source)
e=Emu();e.load_tech();ids=[0,5,8,18,19,21,25,31,33,38];rows=[]
assert len(inputs)==150
for rec,gear,value in inputs:
 e.set_u32(OBJ_BASE+0x8cc,ids[int(rec)])
 e.write(OBJ_BASE+0x2640,struct.pack('<f',float(value)))
 result=e.call(0x4ff5b0,[int(gear)],'i',this=OBJ_BASE)
 rows.append(str(struct.unpack('<I',struct.pack('<f',result))[0]))
pathlib.Path(__file__).with_name('golden_dashboard_override.tsv').write_text('\n'.join(rows)+'\n')
print('Captured',len(rows),'native vehicle speed overrides')

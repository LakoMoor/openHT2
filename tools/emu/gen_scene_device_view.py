"""Capture hardware view uploads from original 0x5ae0e0, without math stubs."""
import pathlib, random, struct
from emu import Emu, OBJ_BASE, STUB

e = Emu(); r = random.Random(0x5ae440)
c = OBJ_BASE; a = c + 0x10000; device = a + 0x1000; vtable = device + 0x100
identity = struct.pack('<12f', 1,0,0,0,1,0,0,0,1,0,0,0)
e.write(0x7025f0, identity); e.set_u32(0x702e08, 3)
e.set_u32(0x720908, device); e.set_u32(0x720904, 1)
e.set_u32(device, vtable); e.set_u32(vtable + 0x2c, STUB + 0x410)
e.hook_ret(0x5adea0, lambda q: 0)
uploads = []
def capture(q):
    assert q.arg(0) == device and q.arg(1) == 2
    uploads.append(struct.unpack('<16I', q.read(q.arg(2), 64)))
    return 0
e.hook_thiscall(STUB + 0x410, capture, 3)
rows = []
for case in range(200):
    values = [r.uniform(-10, 10) for _ in range(12)]
    for k in [0, 4, 8]: values[k] += 30
    raw = identity if case == 0 else struct.pack('<12f', *values)
    e.write(c, bytes(0x2200)); e.write(a, raw)
    e.write(c+0x2110, struct.pack('<d', 2.0)); e.write(c+0x2148, struct.pack('<d', 250.0))
    e.set_u32(c+0x7a8, 100<<19)
    uploads.clear(); e.call(0x5ae0e0, [a, 0x7025f0], 'ii', this=c)
    assert len(uploads) == 1
    rows.append(','.join(map(str, struct.unpack('<12I', raw)))+'\t'+','.join(map(str, uploads[0])))
pathlib.Path(__file__).with_name('golden_scene_device_view.tsv').write_text('\n'.join(rows)+'\n')
print('200 original hardware view uploads captured')

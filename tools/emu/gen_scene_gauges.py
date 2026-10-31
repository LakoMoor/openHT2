"""0x5553e0 (the gauge change check of the vehicle scene update, ecx = scene) run unmodified; 0x546240 (the network
message about changed gauges) is a recording stub that notes [scene+0x4f50] at each call. Row: scene areas before,
gauge object before, calls, scene areas after, gauge object after."""
import pathlib, random, struct
from emu import Emu, OBJ_BASE

r = random.Random(0x5553e0)
e = Emu()
SCENE, OBJ = OBJ_BASE + 0x10000, OBJ_BASE + 0x30000
AREAS = [(0x2540, 0x2544), (0x4f50, 0x4f54), (0x51a4, 0x51a5), (0x523c, 0x5280 + 0x20)]
calls = []
e.hook_thiscall(0x546240, lambda emu: (calls.append(e.read(SCENE + 0x4f50, 4).hex()), 0)[1], 0)
rows = []
for case in range(400):
    calls.clear()
    e.write(SCENE, bytes(0x5400)); e.write(OBJ, bytes(0xc0))
    e.set_u32(SCENE + 0x2540, OBJ)
    e.write(SCENE + 0x51a4, bytes([r.choice([0x80, 0x80, 0x80, 0x00, 0x7f, 0xff])]))
    e.set_u32(SCENE + 0x4f50, r.choice([0, 0, 1]))
    for i in range(0xb4 // 4):
        e.set_u32(OBJ + 4 * i, r.choice([0, 1, 2, 3, 4]))
    e.set_u32(OBJ + 8, r.choice([0, 1, 4, 8]))
    same = r.random() < 0.5
    for i in range(9):
        e.set_u32(SCENE + 0x523c + 4 * i, e.u32(OBJ + 4 + 0x14 * i) if hasattr(e, 'u32') else struct.unpack('<I', e.read(OBJ + 4 + 0x14 * i, 4))[0])
    same2 = r.random() < 0.5
    for i in range(8):
        v = r.randrange(0, 5)
        e.set_u32(SCENE + 0x5260 + 4 * i, v)
        e.set_u32(SCENE + 0x5280 + 4 * i, v if same2 else r.randrange(0, 5))
    if not same:
        i = r.randrange(9)
        e.set_u32(SCENE + 0x523c + 4 * i, 99)
    before = (b''.join(e.read(SCENE + a, b - a) for a, b in AREAS), e.read(OBJ, 0xc0))
    e.call(0x5553e0, [], '', this=SCENE)
    after = (b''.join(e.read(SCENE + a, b - a) for a, b in AREAS), e.read(OBJ, 0xc0))
    rows.append(chr(9).join([before[0].hex(), before[1].hex(), ','.join(calls), after[0].hex(), after[1].hex()]))
pathlib.Path(__file__).with_name('golden_scene_gauges.tsv').write_text(chr(10).join(rows) + chr(10))
print(len(rows), 'cases;', sum(1 for x in rows if x.split(chr(9))[2]), 'with calls')

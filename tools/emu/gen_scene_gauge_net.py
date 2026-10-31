"""0x546240 (the gauge network message, ecx = scene) run unmodified. 0x555500 is replaced in emulator memory by
`fld dword ptr [LEVEL]; ret` (its callees are not ported); the stream functions are stubs, the write records the
payload. Row: scene bytes +0x4b0.., the words +0x523c.. (0x44 bytes), channel, the level (f32 hex), `channel:payload`."""
import pathlib, random, struct
from emu import Emu, OBJ_BASE

r = random.Random(0x546240)
e = Emu()
SCENE, LEVEL = OBJ_BASE + 0x10000, OBJ_BASE + 0x30000
e.write(0x555500, b'\xd9\x05' + struct.pack('<I', LEVEL) + b'\xc3')
got = []
e.hook_thiscall(0x602be0, lambda emu: 0, 0)
e.hook_thiscall(0x5c3630, lambda emu: 0, 1)
e.hook_thiscall(0x602e80, lambda emu: 0, 0)
e.hook_thiscall(0x603110, lambda emu: (got.append('%d:%s' % (struct.unpack('<i', struct.pack('<I', emu.arg(0)))[0],
                                                           bytes(e.read(emu.arg(1), emu.arg(2))).hex())), 0)[1], 3)
rows = []
for case in range(200):
    got.clear()
    e.write(SCENE, bytes(0x5600))
    e.write(SCENE + 0x4b0, bytes(r.randrange(256) for _ in range(16)))
    e.write(SCENE + 0x523c, bytes(r.randrange(256) for _ in range(0x44)))
    ch = r.choice([0, 1, 5, -1, -300, 32767])
    e.write(0x67d10a, struct.pack('<h', ch))
    lv = struct.pack('<f', r.choice([0.0, 1.0, -2.5, r.uniform(-100, 100)]))
    e.write(LEVEL, lv)
    pre = (e.read(SCENE + 0x4b0, 16).hex(), e.read(SCENE + 0x523c, 0x44).hex())
    e.call(0x546240, [], '', this=SCENE)
    rows.append(chr(9).join([pre[0], pre[1], str(ch), lv.hex(), got[0]]))
pathlib.Path(__file__).with_name('golden_scene_gauge_net.tsv').write_text(chr(10).join(rows) + chr(10))
print(len(rows), 'cases')

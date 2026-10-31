"""0x546360 (the effect-slot network message, ecx = scene) run unmodified; the stream functions 0x602be0 (construct),
0x603110 (write: channel, buffer, length), 0x5c3630 (flush), 0x602e80 (destroy) are stubs, the write records the
payload. Row: scene bytes +0x4b0..0x4c0, +0x4c0..0x4d0, +0x180..0x190, +0x5468..0x5540 (hex), channel, result
`channel:payload` (empty when nothing was written)."""
import pathlib, random, struct
from emu import Emu, OBJ_BASE

r = random.Random(0x546360)
e = Emu()
SCENE = OBJ_BASE + 0x10000
got = []
e.hook_thiscall(0x602be0, lambda emu: 0, 0)
e.hook_thiscall(0x5c3630, lambda emu: 0, 1)
e.hook_thiscall(0x602e80, lambda emu: 0, 0)
e.hook_thiscall(0x603110, lambda emu: (got.append('%d:%s' % (struct.unpack('<i', struct.pack('<I', emu.arg(0)))[0] if False else
                                                           struct.unpack('<i', struct.pack('<I', emu.arg(0)))[0],
                                                           bytes(e.read(emu.arg(1), emu.arg(2))).hex())), 0)[1], 3)
rows = []
for case in range(200):
    got.clear()
    e.write(SCENE, bytes(0x5600))
    e.write(SCENE + 0x4b0, bytes(r.randrange(256) for _ in range(16)))
    e.set_u32(SCENE + 0x4c4, r.choice([0, 1, 1, 1]))
    e.set_u32(SCENE + 0x184, r.choice([0, 0, 0, 1]))
    e.write(SCENE + 0x5468, bytes(r.randrange(256) for _ in range(0xd8)))
    # the channel is a signed word; the original sign-extends it
    ch = r.choice([0, 1, 5, -1, -300, 32767, 1234])
    e.write(0x67d10a, struct.pack('<h', ch))
    s = lambda a, n: e.read(SCENE + a, n).hex()
    pre = (s(0x4b0, 16), s(0x4c0, 16), s(0x180, 16), s(0x5468, 0xd8))
    e.call(0x546360, [], '', this=SCENE)
    rows.append(chr(9).join([pre[0], pre[1], pre[2], pre[3], str(ch), got[0] if got else '']))
pathlib.Path(__file__).with_name('golden_scene_net.tsv').write_text(chr(10).join(rows) + chr(10))
print(len(rows), 'cases;', sum(1 for x in rows if x.split(chr(9))[5]), 'sent')

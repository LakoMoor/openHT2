"""Scene light intensity 0x557090 (fastcall, ecx = scene) run unmodified; the result is ST0."""
import pathlib, random, struct
from emu import Emu, OBJ_BASE

r = random.Random(0x557090)
e = Emu()
SCENE, REC, FLAG = OBJ_BASE + 0x10000, OBJ_BASE + 0x30000, OBJ_BASE + 0x40000
rows = []
for case in range(400):
    e.write(SCENE, bytes(0x5500)); e.write(REC, bytes(0xc0)); e.write(FLAG, bytes(0x20))
    count = r.choice([r.randrange(1, 9)] * 4 + [r.randrange(-3, 3), 0])
    d = r.choice([r.uniform(0.5, 1.5)] * 3 + [r.uniform(-0.5, 0.5), 0.0, 0.5, 0.25, 0.49999, 1.0, -1.0, float('nan')])
    flag = r.choice([0] * 7 + [1])
    e.set_u32(SCENE + 0x5080, count & 0xffffffff)
    e.write(SCENE + 0x224, struct.pack('<f', d))
    e.set_u32(SCENE + 0x5440 + 0x14, FLAG); e.set_u32(FLAG + 0xc, flag)
    e.set_u32(SCENE + 0x2540, REC)
    pairs = []
    for off in (4, 0x18, 0x2c, 0x40, 0x54, 0x68, 0x7c, 0x90, 0xa4):
        big = r.choice([5, 50, 1000, 100000])
        a = r.choice([r.randrange(-big, big + 1), 0, 1, -1, 0, 0])
        b = r.choice([r.randrange(1, big + 1), r.randrange(-big, 0) if big else -1, 1, 3, 7])
        if r.random() < 0.6:
            a = r.randrange(-abs(b), abs(b) + 1)
        if r.random() < 0.03:
            b = 0
        e.write(REC + off, struct.pack('<ii', a, b)); pairs.append((a, b))
    res = e.call(0x557090, [], '', this=SCENE)
    rows.append(chr(9).join([str(count), struct.pack('<f', d).hex(), str(flag), ','.join('%d/%d' % p for p in pairs), struct.pack('<d', res).hex()]))
pathlib.Path(__file__).with_name('golden_scene_light_intensity.tsv').write_text(chr(10).join(rows) + chr(10))
print(len(rows), sum(1 for x in rows if x.split(chr(9))[-1] != '0' * 16), 'nonzero')

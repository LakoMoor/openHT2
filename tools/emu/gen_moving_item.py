"""MovingItem setters 0x5812a0 (set V) and 0x5816d0 (set w) run unmodified; the range report 0x4e2821 is a recording
stub (vector seen, limit). Row: kind, input words (sparse), vector, changed dwords (area 0x1000), report."""
import pathlib, random, struct
from emu import Emu, OBJ_BASE

r = random.Random(0x5812a0)
e = Emu()
e.hook_ret(0x61cca9, lambda emu: 0)   # the message text is only read by the report
OBJ, VEC, NAME = OBJ_BASE + 0x10000, OBJ_BASE + 0x30000, OBJ_BASE + 0x31000
F = lambda *v: struct.pack('<%df' % len(v), *v)
rep = []
e.hook_ret(0x4e2821, lambda emu: (rep.append('%s|%08x' % (e.read(emu.arg(0), 12).hex(), emu.arg(2))), 0)[1])
e.write(NAME, b'name\0')
e.set_u32(0x6d2078, 0)
IN = [0x194, 0x198, 0x19c] + list(range(0x254, 0x278, 4)) + [0xd84, 0x378]
val = lambda: r.choice([0.0, 1.0, -1.0, 0.5, 2.0, r.uniform(-8, 8), r.uniform(-1, 1)])
rows = []
for case in range(300):
    rep.clear()
    e.write(OBJ, bytes(0x1000))
    inp = {o: F(val()) for o in IN}
    for o in (0x194, 0x198, 0x19c):
        inp[o] = F(r.choice([1.0, 2.0, 0.5, r.uniform(0.2, 3)]))
    for o, b in inp.items():
        e.write(OBJ + o, b)
    AUX = OBJ_BASE + 0x40000
    e.write(AUX, bytes(0x100))
    e.set_u32(OBJ + 0x20, AUX); e.set_u32(AUX + 0x40, AUX); e.set_u32(OBJ + 0x320, AUX)
    vec = F(*[val() * r.choice([1, 40, 200]) for _ in range(3)])
    e.write(VEC, vec)
    before = e.read(OBJ, 0x1000)
    kind = 'v' if case % 2 else 'w'
    if kind == 'v':
        e.call(0x5812a0, [VEC, NAME], 'ii', this=OBJ)
    else:
        e.call(0x5816d0, [VEC, NAME], 'ii', this=OBJ)
    after = e.read(OBJ, 0x1000)
    diff = ','.join('%x:%s' % (i, after[i:i + 4].hex()) for i in range(0, 0x1000, 4) if after[i:i + 4] != before[i:i + 4])
    rows.append(chr(9).join([kind, ','.join('%x:%s' % (o, b.hex()) for o, b in inp.items()), vec.hex(), diff, rep[0] if rep else '']))
pathlib.Path(__file__).with_name('golden_moving_item.tsv').write_text(chr(10).join(rows) + chr(10))
print(len(rows), 'cases; reports', sum(1 for x in rows if x.split(chr(9))[4]))

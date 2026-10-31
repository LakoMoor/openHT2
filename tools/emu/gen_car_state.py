"""Car_V state setters run unmodified: 0x4f1dd0 (velocity; the report 0x4e2821 is a recording stub), 0x4f1e20
(angular), 0x4f0530 (derived velocity values). Row: kind, car input words (sparse `off:hex`), vector (12 bytes hex),
changed dwords `off:hex` (whole car area 0x3c00)."""
import pathlib, random, struct
from emu import Emu, OBJ_BASE

r = random.Random(0x4f0530)
e = Emu()
CAR, VEC = OBJ_BASE + 0x10000, OBJ_BASE + 0x30000
F = lambda *v: struct.pack('<%df' % len(v), *v)
for addr, v in ((0x702620, (0, 1, 0)), (0x702630, (0, 0, 1)), (0x702640, (1, 0, 0)), (0x702650, (0, 0, 0))):
    e.write(addr, F(*v))
checks = []
e.hook_ret(0x4e2821, lambda emu: (checks.append(1), 0)[1])
IN = [0x40, 0x44, 0x48, 0x4c, 0x50, 0x54, 0xd0, 0xd4, 0xd8, 0xdc0, 0xa38, 0xa3c, 0xa40, 0x900, 0x904, 0x908, 0x90c, 0x910,
      0x914, 0x918, 0x91c, 0x920, 0x1f48, 0x1f4c, 0x1f50, 0xee0, 0xee4, 0xdd4]
rows = []


def val():
    return r.choice([0.0, 1.0, -1.0, 0.5, 2.0, r.uniform(-8, 8), r.uniform(-1, 1), 0.0005, 6.0])


for case in range(400):
    checks.clear()
    e.write(CAR, bytes(0x3c00))
    kind = ['vel', 'ang', 'der', 'pose'][case % 4]
    inp = {}
    for o in IN:
        inp[o] = F(val())
    inp[0xdd4] = struct.pack('<i', r.choice([0, 1, 3, 4]))
    inp[0x1f48] = F(r.choice([1.0, 2.0, 0.5, r.uniform(0.2, 3)]))
    inp[0x1f4c] = F(r.choice([1.0, 2.0, 0.5, r.uniform(0.2, 3)]))
    inp[0x1f50] = F(r.choice([1.0, 2.0, 0.5, r.uniform(0.2, 3)]))
    if r.random() < 0.5:        # the speed is near the stored one and not tiny, so that the reset is skipped
        sp = [r.uniform(-4, 4) for _ in range(3)]
        inp[0x40], inp[0x44], inp[0x48] = F(sp[0]), F(sp[1]), F(sp[2])
        inp[0xdc0] = F((sp[0] ** 2 + sp[1] ** 2 + sp[2] ** 2) ** 0.5 + r.uniform(-1, 1))
    for o, b in inp.items():
        e.write(CAR + o, b)
    for o in range(0x3bb0 - 0x1540, 0x3bb0 - 0x1540 + 16, 4):
        e.write(CAR + o, F(val()))
        inp[o] = e.read(CAR + o, 4)
    for o in (0xc90, 0xc94, 0xc98, 0xcd8, 0xcdc, 0xce0):
        e.write(CAR + o, F(7.0)); inp[o] = F(7.0)
    vec = F(*[val() * r.choice([1, 20, 100]) for _ in range(3)])
    e.write(VEC, vec)
    POSEB = F(*[r.uniform(-1.5, 1.5) for _ in range(9)] + [r.uniform(-50, 50) for _ in range(3)])
    PID = r.randrange(0, 2 ** 32)
    if kind == 'pose':
        e.write(VEC, POSEB)
        vec = POSEB + struct.pack('<I', PID)
        for k in range(4):
            e.write(CAR + 0x1844 + 12 * k, F(val(), val(), val()))
            inp[0x1844 + 12 * k] = e.read(CAR + 0x1844 + 12 * k, 12)
    before = e.read(CAR, 0x3c00)
    if kind == 'pose':
        e.call(0x4f0fc0, [PID, VEC], 'ii', this=CAR)
    elif kind == 'vel':
        e.call(0x4f1dd0, [VEC], 'i', this=CAR)
    elif kind == 'ang':
        e.call(0x4f1e20, [VEC], 'i', this=CAR)
    else:
        e.call(0x4f0530, [], '', this=CAR)
    after = e.read(CAR, 0x3c00)
    diff = ','.join('%x:%s' % (i, after[i:i + 4].hex()) for i in range(0, 0x3c00, 4) if after[i:i + 4] != before[i:i + 4])
    rows.append(chr(9).join([kind, ','.join('%x:%s' % (o, b.hex()) for o, b in inp.items()), vec.hex(), diff]))
pathlib.Path(__file__).with_name('golden_car_state.tsv').write_text(chr(10).join(rows) + chr(10))
print(len(rows), 'cases; with changes', sum(1 for x in rows if x.split(chr(9))[3]))

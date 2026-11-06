"""0x5820c0 (cdecl (body, matrix*, minT*, t*): for a mode-2 body sweeps its sample points through 0x581ea0, which walks the point
iterators 0x58d0f0 / 0x58d120 / 0x58d170) run unmodified, everything real. Row: body words, matrix, t-array in, minT in,
result, body diff, t-array out (20 floats), minT out."""
import pathlib, random, struct
from emu import Emu, OBJ_BASE

r = random.Random(0x5820c0)
e = Emu()
F = lambda *v: struct.pack('<%df' % len(v), *v)
BODY, MAT, TA, MT = [OBJ_BASE + 0x10000 + 0x2000 * i for i in range(4)]
val = lambda k: r.choice([0.0, 1.0, -1.0, r.uniform(-k, k)])
rows = []
for case in range(400):
    for a in (BODY, MAT, TA, MT): e.write(a, bytes(0x1000 if a == BODY else 0x100))
    mode = r.choice([2, 2, 2, 1, 3])
    im = r.choice([0, 0, 1, 1, 2])
    w = {0x188: struct.pack('<I', mode), 0xe04: struct.pack('<I', im), 0xdfc: struct.pack('<i', r.choice([0, 0, 2, 25])),
         0xdf4: struct.pack('<i', r.choice([0, 1, 3, 8, 20])), 0xdf8: struct.pack('<i', r.choice([0, 1, 2, 4])), 0xe00: struct.pack('<i', r.choice([0, 0, 1]))}
    for o in (0x3c4, 0x3c8, 0x3cc): w[o] = F(val(3))
    for o in (0x3d0, 0x3d4, 0x3d8): w[o] = F(val(1.5))
    for i in range(20): w[0xe08 + 12 * i] = F(val(2), val(2), val(2))
    for i in range(4):
        w[0xef8 + 12 * i] = F(val(2), val(2), val(2)) if 0xef8 + 12 * i < 0xf28 or True else b''
        w[0xf28 + 4 * i] = struct.pack('<I', r.choice([0, 1, 1]))
    for o, b in w.items(): e.write(BODY + o, b)
    mat = F(*[r.choice([1.0, 0.0, val(1)]) for _ in range(9)], *[val(4) for _ in range(3)]); e.write(MAT, mat)
    tin = F(*[val(2) for _ in range(20)]); e.write(TA, tin)
    mt = F(r.choice([1.0, 0.5, 0.0005]))
    e.write(MT, mt)
    before = e.read(BODY, 0x1000)
    e.call(0x5820c0, [BODY, MAT, MT, TA], 'iiii'); out = e.eax & 0xffffffff
    after = e.read(BODY, 0x1000)
    diff = ','.join('%x:%s' % (i, after[i:i + 4].hex()) for i in range(0, 0x1000, 4) if after[i:i + 4] != before[i:i + 4])
    rows.append(chr(9).join([','.join('%x:%s' % (o, b.hex()) for o, b in w.items()), mat.hex(), tin.hex(), mt.hex(), '%x' % out, diff, e.read(TA, 80).hex(), e.read(MT, 4).hex()]))
pathlib.Path(__file__).with_name('golden_sweep_hits.tsv').write_text(chr(10).join(rows) + chr(10))
from collections import Counter
print(len(rows), Counter(int(x.split(chr(9))[4], 16) for x in rows).most_common(6))

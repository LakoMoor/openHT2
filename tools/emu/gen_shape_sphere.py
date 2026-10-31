"""0x5f6820 kind 1 (sphere {centre, radius} at [vol+8]) run unmodified. thiscall (mode, start, end, rad), ecx = volume record.
Row: mode, sphere (16 bytes hex), start, end, rad, result, start after, end after."""
import pathlib, random, struct
from emu import Emu, OBJ_BASE

r = random.Random(0x5f6a2c)
e = Emu()
VOL, SPH, S, E = [OBJ_BASE + 0x10000 + 0x1000 * i for i in range(4)]
F = lambda *v: struct.pack('<%df' % len(v), *v)
e.write(0x702650, F(0, 0, 0))
rows = []
val = lambda k: r.choice([0.0, 1.0, -1.0, r.uniform(-k, k), r.uniform(-k, k)])
for case in range(400):
    mode = r.choice([0, 1, 2, 3, 4, 6])
    c = [val(5) for _ in range(3)]
    rad_s = r.choice([0.5, 1.0, 2.0, r.uniform(0.1, 4)])
    sph = F(*c, rad_s)
    kind = r.choice(['out', 'out', 'cross', 'inside', 'point', 'point_in'])
    if kind == 'out':
        s = [val(8) for _ in range(3)]; en = [val(8) for _ in range(3)]
    elif kind == 'cross':
        d = [r.uniform(-1, 1) for _ in range(3)]
        n = sum(x * x for x in d) ** 0.5 or 1
        d = [x / n for x in d]
        k1, k2 = r.uniform(2, 8), r.uniform(-8, 2)
        s = [c[i] - d[i] * k1 for i in range(3)]; en = [c[i] + d[i] * (-k2 if k2 < 0 else k2) for i in range(3)]
    elif kind == 'inside':
        s = [c[i] + r.uniform(-0.3, 0.3) for i in range(3)]; en = [val(8) for _ in range(3)]
    elif kind == 'point':
        s = [val(8) for _ in range(3)]; en = list(s)
    else:
        s = [c[i] + r.uniform(-0.3, 0.3) for i in range(3)]; en = list(s)
    rad = r.choice([0.0, 0.25, 1.0, r.uniform(0, 2)])
    e.write(VOL, struct.pack('<III', 1, 1, SPH)); e.write(SPH, sph)
    e.write(S, F(*s)); e.write(E, F(*en))
    e.call(0x5f6820, [mode, S, E, rad], 'iiif', this=VOL)
    rows.append(chr(9).join([str(mode), sph.hex(), F(*s).hex(), F(*en).hex(), F(rad).hex(), '%x' % e.eax, e.read(S, 12).hex(), e.read(E, 12).hex()]))
pathlib.Path(__file__).with_name('golden_shape_sphere.tsv').write_text(chr(10).join(rows) + chr(10))
from collections import Counter
print(len(rows), Counter(x.split(chr(9))[5] for x in rows))

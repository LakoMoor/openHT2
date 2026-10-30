"""0x54a330: piecewise linear curve (0.3 below 5, 1.0 above 20, linear in between)."""
import pathlib, random, struct
from emu import Emu
r = random.Random(0x54a330)
e = Emu()
rows = []
xs = [0.0, 4.999999, 5.0, 5.0000005, 12.5, 19.999998, 20.0, 20.000002, -3.0, 1e30, float('nan'), float('inf'), -float('inf')]
xs += [r.uniform(-5, 30) for _ in range(300)]
for x in xs:
    xb = struct.pack('<f', x)
    y = e.call(0x54a330, [x], 'f')
    rows.append('%s\t%s' % (xb.hex(), struct.pack('<d', y).hex()))
pathlib.Path(__file__).with_name('golden_curve_54a330.tsv').write_text(chr(10).join(rows) + chr(10))
print(len(rows))

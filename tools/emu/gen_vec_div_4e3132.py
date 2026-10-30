"""0x4e3132: out = a / b, component by component (z, y, x evaluated in that order)."""
import pathlib, random, struct
from emu import Emu, OBJ_BASE
r = random.Random(0x4e3132)
e = Emu()
out, a, b = OBJ_BASE + 0x100, OBJ_BASE + 0x200, OBJ_BASE + 0x300
sp = [0.0, -0.0, float('inf'), -float('inf'), float('nan'), 1e-38, 3e38]
rows = []
for _ in range(400):
    pick = lambda: r.choice(sp) if r.random() < 0.12 else r.uniform(-50, 50)
    va = struct.pack('<3f', *[pick() for _ in range(3)])
    vb = struct.pack('<3f', *[pick() for _ in range(3)])
    e.write(a, va); e.write(b, vb); e.write(out, bytes(12))
    e.call(0x4e3132, [out, a, b], 'iii')
    rows.append('\t'.join([va.hex(), vb.hex(), e.read(out, 12).hex()]))
pathlib.Path(__file__).with_name('golden_vec_div_4e3132.tsv').write_text(chr(10).join(rows) + chr(10))
print(len(rows))

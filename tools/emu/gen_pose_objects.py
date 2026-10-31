"""0x561d10 (pose set with inverse) and 0x5f25a0 (pose get) run unmodified. Row: kind, object bytes (0x70), pose bytes
(48), id (hex), object bytes after (0x64) for `set` / the 48 bytes written for `get`."""
import pathlib, random, struct
from emu import Emu, OBJ_BASE

r = random.Random(0x561d10)
e = Emu()
THIS, POSE, OUT = OBJ_BASE + 0x10000, OBJ_BASE + 0x20000, OBJ_BASE + 0x30000
F = lambda *v: struct.pack('<%df' % len(v), *v)
rows = []
for case in range(300):
    e.write(THIS, bytes(r.randrange(256) for _ in range(0x70)))
    mat = [r.uniform(-1.5, 1.5) for _ in range(9)] + [r.uniform(-50, 50) for _ in range(3)]
    if r.random() < 0.2:
        mat[0] = mat[4] = mat[8] = 1.0
    e.write(POSE, F(*mat))
    e.write(OUT, bytes(48))
    ob, pb = e.read(THIS, 0x70), e.read(POSE, 48)
    if case % 3:
        ident = r.randrange(0, 2 ** 32)
        e.call(0x561d10, [ident, POSE], 'ii', this=THIS)
        rows.append(chr(9).join(['set', ob.hex(), pb.hex(), '%x' % ident, e.read(THIS, 0x64).hex()]))
    else:
        e.call(0x5f25a0, [OUT], 'i', this=THIS)
        rows.append(chr(9).join(['get', ob.hex(), pb.hex(), '', e.read(OUT, 48).hex()]))
pathlib.Path(__file__).with_name('golden_pose_objects.tsv').write_text(chr(10).join(rows) + chr(10))
print(len(rows), 'cases')

"""0x5f2190 (thiscall (matrix, mode)): set / multiply / apply the node matrix at +0x40, run unmodified."""
import math, pathlib, random, struct
from emu import Emu, OBJ_BASE

r = random.Random(0x5f2190)
e = Emu()
N, M = OBJ_BASE + 0x10000, OBJ_BASE + 0x11000
rows = []


def mat():
    a, b, c = r.uniform(-3, 3), r.uniform(-1.5, 1.5), r.uniform(-3, 3)
    ca, sa, cb, sb, cc, sc = math.cos(a), math.sin(a), math.cos(b), math.sin(b), math.cos(c), math.sin(c)
    rot = [cb * cc, cb * sc, -sb, sa * sb * cc - ca * sc, sa * sb * sc + ca * cc, sa * cb, ca * sb * cc + sa * sc, ca * sb * sc - sa * cc, ca * cb]
    return struct.pack('<12f', *(rot + [r.uniform(-30, 30) for _ in range(3)]))


for case in range(300):
    mode = r.choice([0, 1, 2, 3])
    before, arg = mat(), mat()
    e.write(N, bytes(0x40) + before); e.write(M, arg)
    e.call(0x5f2190, [M, mode], 'ii', this=N)
    rows.append(chr(9).join([str(mode), before.hex(), arg.hex(), e.read(N + 0x40, 48).hex()]))
pathlib.Path(__file__).with_name('golden_node_set_transform.tsv').write_text(chr(10).join(rows) + chr(10))
print(len(rows), 'cases')

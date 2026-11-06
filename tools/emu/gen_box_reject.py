"""0x589eb0 (thiscall on the body, ret 8, (p*, q*)): the segment p-q in body space against the box +0x13f4..+0x1400, run unmodified
with the real 0x57dec0. Row: words (addr:hex), result."""
import pathlib, random, struct, math
from emu import Emu, OBJ_BASE

r = random.Random(0x589eb0)
e = Emu()
F = lambda *v: struct.pack('<%df' % len(v), *v)
U = lambda *v: struct.pack('<%dI' % len(v), *v)
BODY, P, Q = OBJ_BASE + 0x10000, OBJ_BASE + 0x1a000, OBJ_BASE + 0x1b000
val = lambda k: r.choice([0.0, 1.0, -1.0, r.uniform(-k, k), r.uniform(-k, k)])
def rot():
    a, b, c = [r.uniform(-3, 3) for _ in range(3)]
    ca, sa, cb, sb, cc, sc = math.cos(a), math.sin(a), math.cos(b), math.sin(b), math.cos(c), math.sin(c)
    rx = [[1, 0, 0], [0, ca, sa], [0, -sa, ca]]; ry = [[cb, 0, -sb], [0, 1, 0], [sb, 0, cb]]; rz = [[cc, sc, 0], [-sc, cc, 0], [0, 0, 1]]
    mul = lambda p, q: [[sum(p[i][k] * q[k][j] for k in range(3)) for j in range(3)] for i in range(3)]
    mm = mul(mul(rx, ry), rz)
    return [x for row in mm for x in row] + [r.uniform(-2, 2) for _ in range(3)]
rows = []
for case in range(600):
    e.write(BODY, bytes(0x2000)); w = {}
    put = lambda a, b: w.__setitem__(a, b)
    put(BODY + 0x5a0, U(r.choice([0, 1])))
    for base in (0x254, 0x730):
        for i, v in enumerate(rot()): put(BODY + base + 4 * i, F(v))
    x0, y0 = r.uniform(-6, 0), r.uniform(-6, 0)
    put(BODY + 0x13f4, F(x0)); put(BODY + 0x13f8, F(x0 + r.uniform(3, 12))); put(BODY + 0x1400, F(y0)); put(BODY + 0x13fc, F(y0 + r.uniform(3, 12)))
    put(P, F(val(3), val(3), val(3))); put(Q, F(val(3), val(3), val(3)))
    for o, b in w.items(): e.write(o, b)
    e.call(0x589eb0, [P, Q], 'ii', this=BODY)
    rows.append(chr(9).join([','.join('%x:%s' % (o, b.hex()) for o, b in w.items()), str(e.eax & 0xffffffff)]))
pathlib.Path(__file__).with_name('golden_box_reject.tsv').write_text(chr(10).join(rows) + chr(10))
from collections import Counter
print(Counter(x.split(chr(9))[1] for x in rows))

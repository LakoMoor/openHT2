"""0x591b50 (cdecl (p*, q*, body, rec*, t): the segment p-q, two 0x5917f0 casts; the point p against the planes (28-byte records at +0xf94) of the body, fills the cast record) run
unmodified with the real 0x58b4f0, 0x57dec0, 0x57de90, 0x58d1f0, 0x58d250, 0x4e6362, 0x4bdef0, 0x4cb2b0, 0x4bde00, 0x4bdd60, 0x610f00; the
error log 0x4e260c is a recording stub. Row: words (addr:hex), t, result, diff of rec (0x80) / body, events."""
import pathlib, random, struct, math
from emu import Emu, OBJ_BASE

r = random.Random(0x591b50)
e = Emu()
F = lambda *v: struct.pack('<%df' % len(v), *v)
U = lambda *v: struct.pack('<%dI' % len(v), *v)
BODY, P, Q, REC = OBJ_BASE + 0x10000, OBJ_BASE + 0x1a000, OBJ_BASE + 0x1a100, OBJ_BASE + 0x1b000
e.write(0x7025f0, F(1, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0)); e.write(0x702650, F(0, 0, 0))
ev = []
e.hook_ret(0x4e260c, lambda emu: (ev.append('err|%x' % emu.arg(0)), 0)[1])
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
    ev.clear()
    e.write(BODY, bytes(0x1800)); e.write(REC, bytes(0x100)); w = {}
    put = lambda a, b: w.__setitem__(a, b)
    put(BODY + 0x5a0, U(r.choice([0, 1])))
    for base in (0x254, 0x730, 0x1f4, 0x700):
        for i, v in enumerate(rot()): put(BODY + base + 4 * i, F(v))
    put(BODY + 0xcd0, F(r.choice([0.0, 0.5, 2.0, val(3)])))
    n = r.choice([0, 1, 2, 4, 6])
    put(BODY + 0x140c, U(n)); put(BODY + 0x1410, U(0))
    for i in range(n):
        nv = [val(1) for _ in range(3)]
        nn = math.sqrt(sum(x * x for x in nv)) or 1.0
        nv = [x / nn for x in nv] if r.random() < 0.7 else nv
        put(BODY + 0xf94 + 28 * i, F(*nv, val(1), val(2), val(2), val(2)))
    put(P, F(val(3), val(3), val(3))); put(Q, F(val(3), val(3), val(3)))
    t = r.choice([0.0, 0.5, 1.0, 3.0, val(4)])
    for o, b in w.items(): e.write(o, b)
    b0, r0 = e.read(BODY, 0x1800), e.read(REC, 0x100)
    e.call(0x591b50, [P, Q, BODY, REC, t], 'iiiif'); res = e.eax & 0xffffffff
    b1, r1 = e.read(BODY, 0x1800), e.read(REC, 0x100)
    d = lambda a, b, n: ','.join('%x:%s' % (i, b[i:i + 4].hex()) for i in range(0, n, 4) if a[i:i + 4] != b[i:i + 4])
    rows.append(chr(9).join([','.join('%x:%s' % (o, b.hex()) for o, b in w.items()), F(t).hex(), str(res), d(r0, r1, 0x100), d(b0, b1, 0x1800), ';'.join(ev)]))
pathlib.Path(__file__).with_name('golden_segment_hull.tsv').write_text(chr(10).join(rows) + chr(10))
from collections import Counter
print(Counter(x.split(chr(9))[2] for x in rows), sum(1 for x in rows if x.split(chr(9))[5]))

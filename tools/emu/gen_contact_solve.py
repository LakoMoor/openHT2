"""0x4e7b4d (cdecl (A0, A1, A2, A3, f18, f1c, f20, A24, A28, A2c, out30, out34), frame 0x1270: the contact solver) and 0x4e794f (the soft length limit,
called inside) run unmodified with the real vector helpers (0x4cb2b0, 0x4bde00, 0x610f00, 0x61f6e4, 0x4e10d0, 0x4bde50, 0x5ab400, 0x4e3132, 0x4bdd60,
0x4bddb0, 0x5ab4d0, 0x5ab620, 0x5ab750); stubs: sprintf 0x61cca9, the log 0x4e260c (event `log|text`) and the formatter 0x4e7a19 (event `fmt|...`).
Row: words (addr:hex), f18, f1c, f20 (bits), diff of the inputs (0x200), outputs (0x20), events."""
import pathlib, random, struct, math
from emu import Emu, OBJ_BASE

r = random.Random(0x4e7b4d)
e = Emu()
F = lambda *v: struct.pack('<%df' % len(v), *v)
U = lambda *v: struct.pack('<%dI' % len(v), *v)
IN, OUT = OBJ_BASE + 0x10000, OBJ_BASE + 0x11000
A0, A1, A2, A3, A24, A28, A2C, O30, O34 = IN, IN + 0x10, IN + 0x20, IN + 0x60, IN + 0xa0, IN + 0xb0, IN + 0xc0, OUT, OUT + 0x10
e.write(0x702650, F(0, 0, 0))
ev = []
hx = lambda a, n: e.read(a, n).hex()
e.hook_ret(0x61cca9, lambda emu: 0)
e.hook_ret(0x4e260c, lambda emu: (ev.append('log|%x' % (emu.arg(0) if emu.arg(0) in (0x673f88, 0x673fac) else 0)), 0)[1])
e.hook_ret(0x4e7a19, lambda emu: (ev.append('fmt|%x|%s|%s|%s' % (emu.arg(2), hx(emu.arg(3), 48), hx(emu.arg(4), 12), hx(emu.arg(5), 12))), 0)[1])
val = lambda k=3: r.choice([0.0, 1.0, -1.0, r.uniform(-k, k), r.uniform(-k, k)])
def rot():
    a, b, c = [r.uniform(-3, 3) for _ in range(3)]
    ca, sa, cb, sb, cc, sc = math.cos(a), math.sin(a), math.cos(b), math.sin(b), math.cos(c), math.sin(c)
    rx = [[1, 0, 0], [0, ca, sa], [0, -sa, ca]]; ry = [[cb, 0, -sb], [0, 1, 0], [sb, 0, cb]]; rz = [[cc, sc, 0], [-sc, cc, 0], [0, 0, 1]]
    mul = lambda p, q: [[sum(p[i][k] * q[k][j] for k in range(3)) for j in range(3)] for i in range(3)]
    mm = mul(mul(rx, ry), rz)
    return [x for row in mm for x in row] + [r.uniform(-3, 3) for _ in range(3)]
rows = []
for case in range(800):
    ev.clear()
    e.write(IN, bytes(0x200)); e.write(OUT, bytes(0x40)); w = {}
    put = lambda a, b: w.__setitem__(a, b)
    m2, m3 = rot(), rot()
    for i, v in enumerate(m2): put(A2 + 4 * i, F(v))
    for i, v in enumerate(m3): put(A3 + 4 * i, F(v))
    a0 = [r.uniform(-4, 4) for _ in range(3)]
    put(A0, F(*a0))
    d = [a0[i] - m2[9 + i] for i in range(3)]
    ln = math.sqrt(sum(x * x for x in d)) or 1.0
    nh = [x / ln for x in d]
    if r.random() < 0.3:
        k = r.uniform(0.2, 3) * r.choice([1, -1])
        a1 = [k * x + r.uniform(-1, 1) * 1e-3 for x in nh]
    else:
        a1 = [val() for _ in range(3)]
    put(A1, F(*a1))
    put(A24, F(*[val() for _ in range(3)])); put(A28, F(*[val() for _ in range(3)]))
    put(A2C, F(*[r.choice([1.0, 0.5, r.uniform(0.1, 2), r.uniform(-0.5, 2)]) for _ in range(3)]))
    f18, f1c, f20 = [r.choice([0.0, 0.5, 1.0, r.uniform(0, 2), r.uniform(-1, 2)]) for _ in range(3)]
    for o, b in w.items(): e.write(o, b)
    i0 = e.read(IN, 0x200)
    e.call(0x4e7b4d, [A0, A1, A2, A3, f18, f1c, f20, A24, A28, A2C, O30, O34], 'iiiifffiiiii')
    i1, o1 = e.read(IN, 0x200), e.read(OUT, 0x20)
    dd = lambda x, y: ','.join('%x:%s' % (i, y[i:i + 4].hex()) for i in range(0, len(x), 4) if x[i:i + 4] != y[i:i + 4])
    rows.append(chr(9).join([','.join('%x:%s' % (o, b.hex()) for o, b in w.items()), F(f18).hex(), F(f1c).hex(), F(f20).hex(), dd(i0, i1), o1.hex(), ';'.join(ev)]))
pathlib.Path(__file__).with_name('golden_contact_solve.tsv').write_text(chr(10).join(rows) + chr(10))
from collections import Counter
print(len(rows), Counter(t.split('|')[0] for x in rows for t in x.split(chr(9))[6].split(';') if t), Counter(x.split(chr(9))[5][:8] == '0' * 8 for x in rows))

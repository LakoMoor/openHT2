"""0x580a00 (cdecl (body, A0 vec*, n vec*): the contact response of a body) runs unmodified with the real 0x5ac2e0, 0x4e7b4d (and its helpers), 0x5ab4d0, 0x5ab400;
stubs: the virtuals +0x90 / +0x94 of the body (events `v90|vec|text`, `v94|vec|text`), sprintf 0x61cca9, the log 0x4e260c and the formatter 0x4e7a19.
Row: words (addr:hex), diff of body (0x1000), of the arguments (0x40), events."""
import pathlib, random, struct, math
from emu import Emu, OBJ_BASE, STUB

r = random.Random(0x580a00)
e = Emu()
F = lambda *v: struct.pack('<%df' % len(v), *v)
U = lambda *v: struct.pack('<%dI' % len(v), *v)
BODY, VT, ARG = OBJ_BASE + 0x10000, OBJ_BASE + 0x12000, OBJ_BASE + 0x13000
e.write(0x702650, F(0, 0, 0))
ev = []
hx = lambda a, n: e.read(a, n).hex()
for slot, name in ((0x90, 'v90'), (0x94, 'v94')):
    addr = STUB + 0x400 + 0x10 * (slot // 4)
    e.hook_thiscall(addr, (lambda nm: lambda emu: (ev.append('%s|%s|%x' % (nm, hx(emu.arg(0), 12), emu.arg(1))), 0)[1])(name), 2)
    e.set_u32(VT + slot, addr)
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
    e.write(BODY, bytes(0x1000)); e.write(ARG, bytes(0x40)); w = {}
    put = lambda a, b: w.__setitem__(a, b)
    put(BODY, U(VT))
    mt = rot()
    for i, v in enumerate(mt): put(BODY + 0xc64 + 4 * i, F(v))
    put(BODY + 0xcc4, F(r.choice([0.0, 0.5, 1.0, r.uniform(0.1, 3), r.uniform(-0.5, 2)])))
    put(BODY + 0x594, F(r.choice([0.0, 0.5, 1.0, r.uniform(0, 1.5)])))
    for off in (0x1a0, 0x1ac): put(BODY + off, F(*[val() for _ in range(3)]))
    put(BODY + 0xcd8, F(*[r.choice([1.0, 0.5, r.uniform(0.1, 2), r.uniform(-0.5, 2)]) for _ in range(3)]))
    a0 = [r.uniform(-4, 4) for _ in range(3)]
    put(ARG, F(*a0))
    nv = [val() for _ in range(3)]
    if r.random() < 0.7:
        n = math.sqrt(sum(x * x for x in nv)) or 1.0
        nv = [x / n for x in nv]
    put(ARG + 0x10, F(*nv))
    for o, b in w.items(): e.write(o, b)
    b0, a_0 = e.read(BODY, 0x1000), e.read(ARG, 0x40)
    e.call(0x580a00, [BODY, ARG, ARG + 0x10], 'iii')
    b1, a_1 = e.read(BODY, 0x1000), e.read(ARG, 0x40)
    dd = lambda x, y: ','.join('%x:%s' % (i, y[i:i + 4].hex()) for i in range(0, len(x), 4) if x[i:i + 4] != y[i:i + 4])
    rows.append(chr(9).join([','.join('%x:%s' % (o, b.hex()) for o, b in w.items()), dd(b0, b1), dd(a_0, a_1), ';'.join(ev)]))
pathlib.Path(__file__).with_name('golden_contact_a00.tsv').write_text(chr(10).join(rows) + chr(10))
from collections import Counter
print(len(rows), Counter(t.split('|')[0] for x in rows for t in x.split(chr(9))[3].split(';') if t))

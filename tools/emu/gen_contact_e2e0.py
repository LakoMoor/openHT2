"""0x57e2e0 (cdecl (body, rec, out*): contact placement) runs unmodified with the real 0x4e2749, 0x4bdef0, 0x4bde50, 0x4bdd60, 0x4bddb0; stubs: the range
report 0x4e2821 (event `rng|vec|text|limit`) and the virtual +0x90 of the body (event `v90|vec|text`).
Row: words (addr:hex), diff of body (0x1000), rec (0x100), out (0x30), events."""
import pathlib, random, struct, math
from emu import Emu, OBJ_BASE, STUB

r = random.Random(0x57e2e0)
e = Emu()
F = lambda *v: struct.pack('<%df' % len(v), *v)
U = lambda *v: struct.pack('<%dI' % len(v), *v)
BODY, VT, REC, OUT, CLK = [OBJ_BASE + 0x10000 + 0x2000 * i for i in range(5)]
e.set_u32(0x6974c4, CLK); e.write(0x6974c2, struct.pack('<h', 0))
ev = []
hx = lambda a, n: e.read(a, n).hex()
addr = STUB + 0x400 + 0x10 * (0x90 // 4)
e.hook_thiscall(addr, lambda emu: (ev.append('v90|%s|%x' % (hx(emu.arg(0), 12), emu.arg(1))), 0)[1], 2)
e.hook_ret(0x4e2821, lambda emu: (ev.append('rng|%s|%x|%x' % (hx(emu.arg(0), 12), emu.arg(1), emu.arg(2))), 0)[1])
val = lambda: r.choice([0.0, 1.0, -1.0, r.uniform(-4, 4), r.uniform(-4, 4)])
def rot():
    a, b, c = [r.uniform(-3, 3) for _ in range(3)]
    ca, sa, cb, sb, cc, sc = math.cos(a), math.sin(a), math.cos(b), math.sin(b), math.cos(c), math.sin(c)
    rx = [[1, 0, 0], [0, ca, sa], [0, -sa, ca]]; ry = [[cb, 0, -sb], [0, 1, 0], [sb, 0, cb]]; rz = [[cc, sc, 0], [-sc, cc, 0], [0, 0, 1]]
    mul = lambda p, q: [[sum(p[i][k] * q[k][j] for k in range(3)) for j in range(3)] for i in range(3)]
    mm = mul(mul(rx, ry), rz)
    return [x for row in mm for x in row] + [r.uniform(-3, 3) for _ in range(3)]
rows = []
for case in range(500):
    ev.clear()
    e.write(BODY, bytes(0x1000)); e.write(REC, bytes(0x100)); e.write(OUT, bytes(0x40)); w = {}
    put = lambda a, b: w.__setitem__(a, b)
    put(BODY, U(VT)); put(VT + 0x90, U(addr))
    for i, v in enumerate(rot()): put(BODY + 0xc64 + 4 * i, F(v))
    for i in range(3): put(BODY + 0xce4 + 4 * i, F(val()))
    put(BODY + 0x590, F(r.choice([0.0, 1.0, 0.5, val()])))
    put(REC, F(val()))
    for i in range(3): put(REC + 0x34 + 4 * i, F(val()))
    nv = [val() for _ in range(3)]
    if r.random() < 0.6:
        n = math.sqrt(sum(x * x for x in nv)) or 1.0
        nv = [x / n for x in nv]
    put(REC + 0x40, F(*nv))
    for o, b in w.items(): e.write(o, b)
    b0, r0, o0 = e.read(BODY, 0x1000), e.read(REC, 0x100), e.read(OUT, 0x30)
    e.call(0x57e2e0, [BODY, REC, OUT], 'iii')
    b1, r1, o1 = e.read(BODY, 0x1000), e.read(REC, 0x100), e.read(OUT, 0x30)
    d = lambda x, y: ','.join('%x:%s' % (i, y[i:i + 4].hex()) for i in range(0, len(x), 4) if x[i:i + 4] != y[i:i + 4])
    rows.append(chr(9).join([','.join('%x:%s' % (o, b.hex()) for o, b in w.items()), d(b0, b1), d(r0, r1), d(o0, o1), ';'.join(ev)]))
pathlib.Path(__file__).with_name('golden_contact_e2e0.tsv').write_text(chr(10).join(rows) + chr(10))
print(len(rows), sum(1 for x in rows if x.split(chr(9))[2]), sum(1 for x in rows if x.split(chr(9))[3]))

"""0x5f9b00 (thiscall, this = collision wrapper, (flags, rec) ret 8): runs unmodified with the real 0x5f25c0, 0x5ac2e0, 0x5aba40, 0x5abc10;
the virtual +0x34 of the child [this+0x40] is a stub (event `v34`, returns a prepared value).
Row: words (addr:hex), flags, v34 result, result, diff of rec (0x100), diff of this (0x100), diff of node (0x100), events."""
import pathlib, random, struct, math
from emu import Emu, OBJ_BASE, STUB

r = random.Random(0x5f9b00)
e = Emu()
F = lambda *v: struct.pack('<%df' % len(v), *v)
U = lambda *v: struct.pack('<%dI' % len(v), *v)
THIS, CH, VT, NODE, PAR, REC = [OBJ_BASE + 0x10000 + 0x1000 * i for i in range(6)]
e.write(0x7025f0, F(1, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0)); e.write(0x702650, F(0, 0, 0))
ev, st = [], {}
hx = lambda a, n: e.read(a, n).hex()
addr = STUB + 0x400 + 0x10 * (0x34 // 4)
def v34(emu):
    ev.append('v34|%x|%x|%s|%s' % (emu.ecx, emu.arg(0), hx(emu.arg(1) + 4, 48), hx(emu.arg(1) + 0x4c, 24)))
    return st['v34']
e.hook_thiscall(addr, v34, 2)
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
    for a in (THIS, CH, VT, NODE, PAR, REC): e.write(a, bytes(0x100))
    w = {}
    put = lambda a, b: w.__setitem__(a, b)
    put(THIS + 0x28, U(r.choice([0, 0, 0, 1, 2])))
    put(THIS + 0x50, U(r.choice([0, 0, 0, 5])))
    put(THIS + 0x40, U(r.choice([CH, CH, CH, 0])))
    put(THIS + 0x44, U(r.choice([NODE, NODE, NODE, 0])))
    put(THIS + 0x48, U(r.choice([0, 0x2b000001, 0x2b000002])))
    put(CH, U(VT)); put(VT + 0x34, U(addr))
    put(NODE + 0x2c, U(r.choice([0, 0, PAR]))); put(NODE + 0x38, U(r.choice([0, 1, 1, 2, 3])))
    put(PAR + 0x2c, U(0)); put(PAR + 0x38, U(r.choice([0, 1, 2])))
    for base in (NODE + 0x40, PAR + 0x40, REC + 4):
        for i, v in enumerate(rot()): put(base + 4 * i, F(v))
    for i in range(6): put(REC + 0x4c + 4 * i, F(val()))
    put(REC + 0x74, U(r.choice([0, 0x2b000001, 0x77])))
    put(REC + 0x78, U(r.choice([0, 0x2b000001, 0x2b000002, 0x77])))
    flags = r.choice([4, 4, 4, 5, 0, 1, 0xff])
    v34r = r.choice([0, 1, 1, 3])
    for o, b in w.items(): e.write(o, b)
    st['v34'] = v34r
    s0 = [e.read(a, 0x100) for a in (REC, THIS, NODE)]
    e.call(0x5f9b00, [flags, REC], 'ii', this=THIS); res = e.eax & 0xffffffff
    s1 = [e.read(a, 0x100) for a in (REC, THIS, NODE)]
    d = lambda x, y, n: ','.join('%x:%s' % (i, y[i:i + 4].hex()) for i in range(0, n, 4) if x[i:i + 4] != y[i:i + 4])
    rows.append(chr(9).join([','.join('%x:%s' % (o, b.hex()) for o, b in w.items()), str(flags), str(v34r), str(res)] + [d(a, b, 0x100) for a, b in zip(s0, s1)] + [';'.join(ev)]))
pathlib.Path(__file__).with_name('golden_cast_virt.tsv').write_text(chr(10).join(rows) + chr(10))
from collections import Counter
print(len(rows), Counter(int(x.split(chr(9))[3]) for x in rows), sum(1 for x in rows if x.split(chr(9))[7]), sum(1 for x in rows if x.split(chr(9))[4]))

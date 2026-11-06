"""0x5f9d20 (thiscall, this = collision wrapper, (a0, mat*, mat2*, stack) ret 0x10): runs unmodified with the real 0x5ef820, 0x5f25c0, 0x5aba40,
0x5ac2e0; stubs: the transform stack push 0x5ae580 (event `push`), its pop 0x5ae8a0 (event `pop`) and the virtual +0x38 of the child (event `v38`).
Row: words (addr:hex), a0, result of v38, result, diff of mat (0x40), this (0x100), node (0x100), events."""
import pathlib, random, struct, math
from emu import Emu, OBJ_BASE, STUB

r = random.Random(0x5f9d20)
e = Emu()
F = lambda *v: struct.pack('<%df' % len(v), *v)
U = lambda *v: struct.pack('<%dI' % len(v), *v)
THIS, CH, VT, NODE, PAR, MAT, MAT2, STK = [OBJ_BASE + 0x10000 + 0x1000 * i for i in range(8)]
e.write(0x7025f0, F(1, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0)); e.write(0x702650, F(0, 0, 0))
ev, st = [], {}
hx = lambda a, n: e.read(a, n).hex()
addr = STUB + 0x400 + 0x10 * (0x38 // 4)
def v38(emu):
    ev.append('v38|%x|%x|%s|%s|%x' % (emu.ecx, emu.arg(0), hx(emu.arg(1), 48), hx(emu.arg(2), 48), emu.arg(3)))
    return st['v38']
e.hook_thiscall(addr, v38, 4)
e.hook_thiscall(0x5ae580, lambda emu: (ev.append('push|%x|%s|%x' % (emu.ecx, hx(emu.arg(0), 48), emu.arg(1))), 0)[1], 2)
e.hook_thiscall(0x5ae8a0, lambda emu: (ev.append('pop|%x' % emu.ecx), 0)[1], 0)
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
    for a in (THIS, CH, VT, NODE, PAR, MAT, MAT2, STK): e.write(a, bytes(0x100))
    w = {}
    put = lambda a, b: w.__setitem__(a, b)
    put(THIS + 0x28, U(r.choice([0, 0, 0, 1, 2])))
    put(THIS + 0x40, U(r.choice([CH, CH, CH, 0])))
    put(THIS + 0x44, U(r.choice([NODE, NODE, NODE, 0])))
    put(CH, U(VT)); put(VT + 0x38, U(addr))
    put(NODE + 0x2c, U(r.choice([0, 0, PAR]))); put(NODE + 0x38, U(r.choice([0, 1, 1, 2, 3])))
    put(PAR + 0x2c, U(0)); put(PAR + 0x38, U(r.choice([0, 1, 2])))
    for base in (NODE + 0x40, PAR + 0x40, MAT, MAT2):
        for i, v in enumerate(rot()): put(base + 4 * i, F(v))
    a0 = r.choice([0, 3, 0x1234])
    v38r = r.choice([0, 1, 5])
    for o, b in w.items(): e.write(o, b)
    st['v38'] = v38r
    s0 = [e.read(a, n) for a, n in ((MAT, 0x40), (THIS, 0x100), (NODE, 0x100))]
    e.call(0x5f9d20, [a0, MAT, MAT2, STK], 'iiii', this=THIS); res = e.eax & 0xffffffff
    s1 = [e.read(a, n) for a, n in ((MAT, 0x40), (THIS, 0x100), (NODE, 0x100))]
    d = lambda x, y: ','.join('%x:%s' % (i, y[i:i + 4].hex()) for i in range(0, len(x), 4) if x[i:i + 4] != y[i:i + 4])
    rows.append(chr(9).join([','.join('%x:%s' % (o, b.hex()) for o, b in w.items()), str(a0), str(v38r), str(res)] + [d(a, b) for a, b in zip(s0, s1)] + [';'.join(ev)]))
pathlib.Path(__file__).with_name('golden_cast_virt2.tsv').write_text(chr(10).join(rows) + chr(10))
from collections import Counter
print(len(rows), Counter(int(x.split(chr(9))[3]) for x in rows), sum(1 for x in rows if x.split(chr(9))[7]), sum(1 for x in rows if x.split(chr(9))[4]))

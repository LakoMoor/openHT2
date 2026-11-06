"""0x58ae30 (body against its up to six neighbours, cdecl (body, matrix, set)) run unmodified with the real 0x57de90, 0x57dec0, 0x5aba40,
0x5ac2e0, 0x4e2749; stubs: 0x58a020 and 0x584c70 (log their arguments, add to the sum / the contact count).
Row: words (addr:hex), return value, diff of the set (0x1000), events."""
import pathlib, random, struct
from emu import Emu, OBJ_BASE

r = random.Random(0x58ae30)
e = Emu()
F = lambda *v: struct.pack('<%df' % len(v), *v)
U = lambda *v: struct.pack('<%dI' % len(v), *v)
BODY, SET, ARG1, CLK = OBJ_BASE + 0x10000, OBJ_BASE + 0x20000, OBJ_BASE + 0x28000, OBJ_BASE + 0x2a000
NB = lambda i: OBJ_BASE + 0x30000 + 0x2000 * i
e.write(0x7025f0, F(1, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0)); e.write(0x702650, F(0, 0, 0))
e.set_u32(0x6974c4, CLK); e.write(0x6974c2, struct.pack('<h', 0))
ev, st = [], {}
hx = lambda a, n: e.read(a, n).hex()
def a020(emu):
    ev.append('a020|%x|%s|%x|%x|%x' % (emu.arg(0), hx(emu.arg(1), 48), emu.arg(2), emu.arg(3), emu.arg(4)))
    return st['a020'].pop(0) if st['a020'] else 0
def c70(emu):
    ev.append('c70|%s|%s|%x|%x|%x|%x' % (hx(emu.arg(0), 12), hx(emu.arg(1), 12), emu.arg(2), emu.arg(3), emu.arg(4), emu.arg(5)))
    k = st['c70'].pop(0) if st['c70'] else 0
    c = int.from_bytes(e.read(emu.arg(3) + 0xb00, 4), 'little')
    e.set_u32(emu.arg(3) + 0xb00, c + k)
    return k
e.hook_ret(0x58a020, a020)
e.hook_ret(0x584c70, c70)
val = lambda k: r.choice([0.0, 1.0, -1.0, r.uniform(-k, k)])
def rot():
    import math
    a, b, c = [r.uniform(-3, 3) for _ in range(3)]
    ca, sa, cb, sb, cc, sc = math.cos(a), math.sin(a), math.cos(b), math.sin(b), math.cos(c), math.sin(c)
    rx = [[1, 0, 0], [0, ca, sa], [0, -sa, ca]]; ry = [[cb, 0, -sb], [0, 1, 0], [sb, 0, cb]]; rz = [[cc, sc, 0], [-sc, cc, 0], [0, 0, 1]]
    mul = lambda p, q: [[sum(p[i][k] * q[k][j] for k in range(3)) for j in range(3)] for i in range(3)]
    mm = mul(mul(rx, ry), rz)
    return [x for row in mm for x in row] + [r.uniform(-9, 9) for _ in range(3)]
rows = []
for case in range(400):
    ev.clear()
    for a in (BODY, SET, ARG1, CLK): e.write(a, bytes(0x2000 if a != CLK else 0x1000))
    for i in range(6): e.write(NB(i), bytes(0x2000))
    w = {}
    put = lambda a, b: w.__setitem__(a, b)
    put(CLK + 0xb90, struct.pack('<d', r.choice([0.016, 0.05, 0.3])))
    put(BODY + 0x5a0, U(r.choice([0, 1])))
    for i, v in enumerate(rot()): put(BODY + 0x1f4 + 4 * i, F(v)); 
    for i, v in enumerate(rot()): put(BODY + 0x700 + 4 * i, F(v))
    for i, v in enumerate(rot()): put(BODY + 0x254 + 4 * i, F(v))
    for i, v in enumerate(rot()): put(BODY + 0x730 + 4 * i, F(v))
    for o in (0xce4, 0xce8, 0xcec, 0xcc8): put(BODY + o, F(val(3)))
    count = r.randrange(7)
    for i in range(6):
        nb = NB(i) if i < count or r.random() < 0.1 else 0
        put(BODY + 0xdb4 + 4 * i, U(nb))
        put(BODY + 0xdcc + 4 * i, F(r.choice([0.0, 1.0, r.uniform(0, 40), r.uniform(0, 400)])))
    for i in range(6):
        put(NB(i) + 0x188, U(r.choice([2, 2, 1, 1, 3, 0])))
        put(NB(i) + 0x5a0, U(r.choice([0, 1])))
        for o in (0xce4, 0xce8, 0xcec, 0xcc8): put(NB(i) + o, F(val(3)))
        for i2, v in enumerate(rot()): put(NB(i) + 0x1f4 + 4 * i2, F(v))
        for i2, v in enumerate(rot()): put(NB(i) + 0x700 + 4 * i2, F(v))
    for i, v in enumerate(rot()): put(ARG1 + 4 * i, F(v))
    put(SET + 0xb00, struct.pack('<i', r.choice([0, 1, 4])))
    for o, b in w.items(): e.write(o, b)
    st['a020'] = [r.choice([0, 1, 2, 5]) for _ in range(12)]
    st['c70'] = [r.choice([0, 1, 1, 2, 3]) for _ in range(6)]
    l1, l2 = list(st['a020']), list(st['c70'])
    before = e.read(SET, 0x1000)
    e.call(0x58ae30, [BODY, ARG1, SET], 'iii'); res = e.eax
    after = e.read(SET, 0x1000)
    diff = ','.join('%x:%s' % (i, after[i:i + 4].hex()) for i in range(0, 0x1000, 4) if after[i:i + 4] != before[i:i + 4])
    rows.append(chr(9).join([','.join('%x:%s' % (o, b.hex()) for o, b in w.items()), str(res & 0xffffffff), diff, ';'.join(ev), ','.join(map(str, l1)), ','.join(map(str, l2))]))
pathlib.Path(__file__).with_name('golden_neighbours.tsv').write_text(chr(10).join(rows) + chr(10))
from collections import Counter
c = Counter(t.split('|')[0] for x in rows for t in x.split(chr(9))[3].split(';') if t)
print(len(rows), dict(c))

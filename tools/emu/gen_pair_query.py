"""0x58a020 (cdecl (body, matrix*, set, other, sign): the sample points of `body` against the other body, appends contacts) run unmodified
with the real 0x57de90, 0x57dec0, 0x58d0f0 family, 0x5aba40, 0x5ac2e0, 0x589eb0, 0x591b50 (0x5917f0 ...), 0x4e28b0, 0x57c9a0; stubs: the context
constructor 0x5ef5c0, the object cast 0x4e52fc (writes a prepared context), the sprintf 0x61cca9 and the log 0x4e260c (event `err`).
Row: words (addr:hex), sign, blobs, hits, result, diff of body (0x2600) / other (0x1800) / set (0x1000), events."""
import pathlib, random, struct, math
from emu import Emu, OBJ_BASE

r = random.Random(0x58a020)
e = Emu()
F = lambda *v: struct.pack('<%df' % len(v), *v)
U = lambda *v: struct.pack('<%dI' % len(v), *v)
A, O, SET, B1 = OBJ_BASE + 0x10000, OBJ_BASE + 0x14000, OBJ_BASE + 0x18000, OBJ_BASE + 0x1a000
e.write(0x7025f0, F(1, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0)); e.write(0x702650, F(0, 0, 0))
ev, st = [], {}
hx = lambda a, n: e.read(a, n).hex()
e.hook_thiscall(0x5ef5c0, lambda emu: (st.__setitem__('k', 0), e.write(emu.ecx, st['blobs'][0]), 0)[2], 0)
def cast(emu):
    ev.append('cast|%x|%s|%s' % (emu.arg(0), hx(emu.arg(1), 12), hx(emu.arg(2), 12)))
    st['k'] += 1
    e.write(emu.arg(3), st['blobs'][min(st['k'], len(st['blobs']) - 1)])
    return st['hits'].pop(0) if st['hits'] else 0
e.hook_ret(0x4e52fc, cast)
e.hook_ret(0x4e260c, lambda emu: (ev.append('err'), 0)[1])
e.hook_ret(0x61cca9, lambda emu: 0)
val = lambda k: r.choice([0.0, 1.0, -1.0, r.uniform(-k, k), r.uniform(-k, k)])
def rot():
    a, b, c = [r.uniform(-3, 3) for _ in range(3)]
    ca, sa, cb, sb, cc, sc = math.cos(a), math.sin(a), math.cos(b), math.sin(b), math.cos(c), math.sin(c)
    rx = [[1, 0, 0], [0, ca, sa], [0, -sa, ca]]; ry = [[cb, 0, -sb], [0, 1, 0], [sb, 0, cb]]; rz = [[cc, sc, 0], [-sc, cc, 0], [0, 0, 1]]
    mul = lambda p, q: [[sum(p[i][k] * q[k][j] for k in range(3)) for j in range(3)] for i in range(3)]
    mm = mul(mul(rx, ry), rz)
    return [x for row in mm for x in row] + [r.uniform(-2, 2) for _ in range(3)]
rows = []
for case in range(500):
    ev.clear()
    e.write(A, bytes(0x2600)); e.write(O, bytes(0x1800)); e.write(SET, bytes(0x1000)); e.write(B1, bytes(0x100)); w = {}
    put = lambda a, b: w.__setitem__(a, b)
    for body in (A, O):
        put(body + 0x5a0, U(r.choice([0, 1])))
        for base in (0x254, 0x730, 0x1f4, 0x700):
            for i, v in enumerate(rot()): put(body + base + 4 * i, F(v))
    for i, v in enumerate(rot()): put(B1 + 4 * i, F(v))
    put(A + 0xe04, U(r.choice([0, 0, 1])))
    put(A + 0xdf4, struct.pack('<i', r.choice([0, 2, 4, 6]))); put(A + 0xdf8, struct.pack('<i', r.choice([0, 3, 6])))
    for i in range(8): put(A + 0xe08 + 12 * i, F(val(3), val(3), val(3)))
    for i in range(8): put(A + 0xef8 + 12 * i, F(val(3), val(3), val(3))); put(A + 0xf28 + 4 * i, U(r.choice([0, 1])))
    for i in range(8): put(A + 0x1444 + 4 * i, U(r.choice([0, 0, 1])))
    put(A + 0x140, U(0x2a0b0000)); put(O + 0x188, U(r.choice([2, 2, 2, 1, 0])))
    put(O + 0x140, U(0x2a0b0000))
    for i, v in enumerate(rot()): put(O + 0x2b4 + 4 * i, F(v))
    x0, y0 = r.uniform(-6, 0), r.uniform(-6, 0)
    put(O + 0x13f4, F(x0)); put(O + 0x13f8, F(x0 + r.uniform(3, 12))); put(O + 0x1400, F(y0)); put(O + 0x13fc, F(y0 + r.uniform(3, 12)))
    put(O + 0xcd0, F(r.choice([0.0, 0.5, 2.0, val(3)])))
    n = r.choice([0, 1, 2, 4])
    put(O + 0x140c, U(n)); put(O + 0x1410, U(0))
    for i in range(n):
        nv = [val(1) for _ in range(3)]
        nn = math.sqrt(sum(x * x for x in nv)) or 1.0
        nv = [x / nn for x in nv] if r.random() < 0.7 else nv
        put(O + 0xf94 + 28 * i, F(*nv, val(1), val(2), val(2), val(2)))
    put(SET + 0xb00, struct.pack('<i', r.choice([0, 0, 3, 0x3e, 0x3f])))
    for o, b in w.items(): e.write(o, b)
    sign = r.choice([1, 1, -1, 2, 0])
    st['k'] = 0
    big = lambda: r.choice([val(3), val(3), r.uniform(-1, 1), 20000.0])
    st['blobs'] = [b''.join(F(r.choice([val(3), val(3), r.uniform(-1, 1)])) for _ in range(52)) for _ in range(3)]
    for k in range(1, 3):
        bl = bytearray(st['blobs'][k])
        for i in range(0x34 // 4, 0x4c // 4):
            bl[4 * i:4 * i + 4] = F(r.choice([val(3), val(3), 20000.0]) if r.random() < 0.1 else val(3))
        st['blobs'][k] = bytes(bl)
    st['hits'] = [r.choice([0, 1, 1, 3]) for _ in range(12)]
    hits0 = list(st['hits'])
    a0, o0, s0 = e.read(A, 0x2600), e.read(O, 0x1800), e.read(SET, 0x1000)
    e.call(0x58a020, [A, B1, SET, O, sign], 'iiiii'); res = e.eax & 0xffffffff
    a1, o1, s1 = e.read(A, 0x2600), e.read(O, 0x1800), e.read(SET, 0x1000)
    d = lambda x, y, n: ','.join('%x:%s' % (i, y[i:i + 4].hex()) for i in range(0, n, 4) if x[i:i + 4] != y[i:i + 4])
    rows.append(chr(9).join([','.join('%x:%s' % (o, b.hex()) for o, b in w.items()), str(sign), ';'.join(x.hex() for x in st['blobs']),
        ','.join(map(str, hits0)), str(res), d(a0, a1, 0x2600), d(o0, o1, 0x1800), d(s0, s1, 0x1000), ';'.join(ev)]))
pathlib.Path(__file__).with_name('golden_pair_query.tsv').write_text(chr(10).join(rows) + chr(10))
from collections import Counter
c = Counter(t.split('|')[0] for x in rows for t in x.split(chr(9))[8].split(';') if t)
print(len(rows), dict(c), Counter(int(x.split(chr(9))[4]) for x in rows))

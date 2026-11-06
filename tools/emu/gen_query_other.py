"""0x58b120 (cdecl (body, matrix*, scale*, out*, rec, flags): the body against everything else; picks the nearest contact of a local set)
run unmodified with the real 0x587c80 (set constructor), 0x5ab710, 0x5ab660, 0x5ab400; stubs: 0x589e70, 0x58ae30, 0x585150 (events
`pc` `nb` `col`, each applies a prepared patch of the set and returns a prepared result).
Row: words (addr:hex), flags, scale, script (3 calls: ret|off:hex,...), result, diff of body (0x1000), rec (0x100), cells (0x20), events."""
import pathlib, random, struct
from emu import Emu, OBJ_BASE

r = random.Random(0x58b120)
e = Emu()
F = lambda *v: struct.pack('<%df' % len(v), *v)
U = lambda *v: struct.pack('<%dI' % len(v), *v)
BODY, MAT, CELL, REC = OBJ_BASE + 0x10000, OBJ_BASE + 0x1a000, OBJ_BASE + 0x1b000, OBJ_BASE + 0x1c000
e.write(0x702650, F(0, 0, 0))
ev, st = [], {}
def mk(name, nargs):
    def fn(emu):
        a = [emu.arg(i) for i in range(nargs)]
        ev.append('%s|%x|%x|%s' % (name, a[0], a[1], '%x' % a[3] if nargs > 3 else ''))
        ret, patch = st['script'].pop(0)
        for off, b in patch:
            e.write(a[2] + off, b)
        return ret
    return fn
e.hook_ret(0x589e70, mk('pc', 4)); e.hook_ret(0x58ae30, mk('nb', 3)); e.hook_ret(0x585150, mk('col', 3))
val = lambda: r.choice([0.0, 1.0, -1.0, r.uniform(-5, 5), r.uniform(-5, 5)])
rows = []
for case in range(500):
    ev.clear()
    e.write(BODY, bytes(0x1000)); e.write(MAT, bytes(0x100)); e.write(CELL, bytes(0x20)); e.write(REC, bytes(0x100)); w = {}
    put = lambda a, b: w.__setitem__(a, b)
    put(BODY + 0xe04, U(r.choice([0, 0, 1])))
    for i in range(8): put(BODY + 0xf38 + 4 * i, U(r.choice([0, 1, 2, 3])))
    for i in range(12): put(REC + 4 * i, U(0xdead0000 + i))
    scale = r.choice([1.0, 0.5, 2.0, val()])
    put(CELL, F(scale))
    flags = r.choice([0, 0x200, 0x10, 0x220, 0x1ffff])
    base_n = [val() for _ in range(3)]
    nn = sum(x * x for x in base_n) ** 0.5 or 1.0
    base_n = [x / nn for x in base_n]
    script = []
    for call in range(3):
        k = r.choice([0, 0, 1, 2, 3, 5])
        patch = []
        ret = r.choice([0, 1, 1, 2, 7])
        if ret == 0 and r.random() < 0.7: k = 0 if r.random() < 0.5 else k
        cnt = 0
        for j in range(k):
            n = r.choice(range(0, 6))
            patch.append((12 * n, F(val(), val(), val())))
            nv = base_n if r.random() < 0.6 else ([x + r.uniform(-1e-5, 1e-5) for x in base_n] if r.random() < 0.5 else [val() for _ in range(3)])
            patch.append((0x300 + 12 * n, F(*nv)))
            patch.append((0x600 + 4 * n, U(0x2b000000 + r.randrange(16))))
            patch.append((0x700 + 4 * n, U(0x2c000000 + r.randrange(16))))
            patch.append((0x800 + 4 * n, F(r.choice([0.0, 0.25, 0.5, 1.0, r.uniform(0, 1.1)]))))
            patch.append((0xa00 + 4 * n, F(val())))
            patch.append((0xb04 + 4 * n, struct.pack('<i', r.choice([0, 1, 1, -1, 2]))))
            patch.append((0xc04 + 4 * n, U(r.randrange(8))))
            patch.append((0xd04 + 0x30 * n, b''.join(F(val()) for _ in range(12))))
            cnt = max(cnt, n + 1)
        if k:
            patch.append((0xb00, U(cnt)))
        script.append((ret, patch))
    for o, b in w.items(): e.write(o, b)
    st['script'] = [(a, list(b)) for a, b in script]
    b0, r0, c0 = e.read(BODY, 0x1000), e.read(REC, 0x100), e.read(CELL, 0x20)
    e.call(0x58b120, [BODY, MAT, CELL, CELL + 0x10, REC, flags], 'iiiiii'); res = e.eax & 0xffffffff
    b1, r1, c1 = e.read(BODY, 0x1000), e.read(REC, 0x100), e.read(CELL, 0x20)
    d = lambda x, y, n: ','.join('%x:%s' % (i, y[i:i + 4].hex()) for i in range(0, n, 4) if x[i:i + 4] != y[i:i + 4])
    sc = ';'.join('%d|%s' % (a, ','.join('%x:%s' % (o, b.hex()) for o, b in p)) for a, p in script)
    rows.append(chr(9).join([','.join('%x:%s' % (o, b.hex()) for o, b in w.items()), str(flags), sc, str(res), d(b0, b1, 0x1000), d(r0, r1, 0x100), d(c0, c1, 0x20), ';'.join(ev)]))
pathlib.Path(__file__).with_name('golden_query_other.tsv').write_text(chr(10).join(rows) + chr(10))
from collections import Counter
print(len(rows), Counter(int(x.split(chr(9))[3]) for x in rows).most_common(8), sum(1 for x in rows if x.split(chr(9))[5]))

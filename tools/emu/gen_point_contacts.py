"""0x589640 (cdecl (body, matrix*, set, flags): casts the body's sample points and records the contacts) run unmodified with the real
0x57de90, 0x57def0, 0x58d0f0 (0x58d120/0x58d170), 0x4bdef0, 0x610f00, 0x5ab620; stubs: the context constructor 0x5ef5c0, the casts 0x4e67d1 / 0x4e6ac4,
the room lookup 0x4e5ba7 and the virtual +0x1c (all log their arguments; the casts write a prepared 0xd0-byte context).
Row: words (addr:hex), flags, blobs (hex, ';'), hits, room, result, diff of body (0x3000) and set (0x1000), events."""
import pathlib, random, struct
from emu import Emu, OBJ_BASE, STUB

r = random.Random(0x589640)
e = Emu()
F = lambda *v: struct.pack('<%df' % len(v), *v)
U = lambda *v: struct.pack('<%dI' % len(v), *v)
BODY, SET, ARG1, VT = OBJ_BASE + 0x10000, OBJ_BASE + 0x20000, OBJ_BASE + 0x28000, OBJ_BASE + 0x2c000
e.write(0x7025f0, F(1, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0)); e.write(0x702620, F(0, 1, 0)); e.write(0x702630, F(0, 0, 1))
e.write(0x702640, F(1, 0, 0)); e.write(0x702650, F(0, 0, 0))
ev, st = [], {}
hx = lambda a, n: e.read(a, n).hex()
addr = STUB + 0x400 + 0x10 * (0x1c // 4)
e.hook_thiscall(addr, lambda emu: (ev.append('v1c|%x' % emu.arg(0)), 0)[1], 1)
e.set_u32(VT + 0x1c, addr)
def blob():
    st['k'] += 1
    return st['blobs'][min(st['k'] - 1, len(st['blobs']) - 1)]
e.hook_thiscall(0x5ef5c0, lambda emu: (st.__setitem__('k', 0), e.write(emu.ecx, st['blobs'][0]), 0)[2], 0)
def cast(name):
    def f(emu):
        ev.append('%s|%x|%s|%s' % (name, emu.arg(0), hx(emu.arg(1), 12), hx(emu.arg(2), 12)))
        e.write(emu.arg(3), blob())
        return st['hits'].pop(0) if st['hits'] else 0
    return f
e.hook_ret(0x4e67d1, cast('c67d1')); e.hook_ret(0x4e6ac4, cast('c6ac4'))
def room(emu):
    ev.append('5ba7|%x|%s|%s' % (emu.arg(0), hx(emu.arg(1), 12), hx(emu.arg(2), 12)))
    return st['room']
e.hook_ret(0x4e5ba7, room)
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
for case in range(500):
    ev.clear()
    e.write(BODY, bytes(0x3000)); e.write(SET, bytes(0x1000)); e.write(ARG1, bytes(0x100)); e.set_u32(BODY, VT)
    w = {}
    put = lambda a, b: w.__setitem__(a, b)
    put(BODY + 0x5a0, U(r.choice([0, 1]))); put(BODY + 0x320, U(0x2a0b0000)); put(BODY + 0x7f0, U(0x2a0c0000))
    for base in (0x1f4, 0x700):
        for i, v in enumerate(rot()): put(BODY + base + 4 * i, F(v))
    for i, v in enumerate(rot()): put(ARG1 + 4 * i, F(v))
    put(BODY + 0xe04, U(r.choice([0, 0, 1])))
    put(BODY + 0xdf4, struct.pack('<i', r.choice([0, 2, 5, 8]))); put(BODY + 0xdf8, struct.pack('<i', r.choice([0, 4, 8])))
    for i in range(8): put(BODY + 0xe08 + 12 * i, F(val(3), val(3), val(3)))
    for i in range(8): put(BODY + 0xef8 + 12 * i, F(val(3), val(3), val(3))); put(BODY + 0xf28 + 4 * i, U(r.choice([0, 1])))
    put(BODY + 0x1418, F(r.choice([0.0, 0.3, 0.8, val(2)])))
    put(BODY + 0x141c, F(val(1)))
    for i in range(3): put(BODY + 0x1420 + 4 * i, F(val(3)))
    for i in range(3): put(BODY + 0x142c + 4 * i, F(val(1)))
    for i in range(3): put(BODY + 0x1438 + 4 * i, F(val(1)))
    for i in range(9): put(BODY + 0x1444 + 4 * i, U(r.choice([0, 0, 1])))
    put(SET + 0xb00, struct.pack('<i', r.choice([0, 0, 3, 0x3d, 0x3e, 0x3f])))
    for o, b in w.items(): e.write(o, b)
    flags = r.choice([0, 0, 0x10, 0x30, 0x200, 0x210, 0x230, 0x20])
    st['k'] = 0
    st['blobs'] = [b''.join(F(r.choice([val(3), val(3), r.uniform(-1, 1)])) for _ in range(52)) for _ in range(4)]
    st['hits'] = [r.choice([0, 1, 1, 3]) for _ in range(12)]
    st['room'] = r.choice([0, 0x2a0d0000])
    hits0 = list(st['hits'])
    b0, s0 = e.read(BODY, 0x3000), e.read(SET, 0x1000)
    e.call(0x589640, [BODY, ARG1, SET, flags], 'iiii'); res = e.eax
    b1, s1 = e.read(BODY, 0x3000), e.read(SET, 0x1000)
    d = lambda a, b, n: ','.join('%x:%s' % (i, b[i:i + 4].hex()) for i in range(0, n, 4) if a[i:i + 4] != b[i:i + 4])
    rows.append(chr(9).join([','.join('%x:%s' % (o, b.hex()) for o, b in w.items()), str(flags), ';'.join(x.hex() for x in st['blobs']),
        ','.join(map(str, hits0)), '%x' % st['room'], str(res), d(b0, b1, 0x3000), d(s0, s1, 0x1000), ';'.join(ev)]))
pathlib.Path(__file__).with_name('golden_point_contacts.tsv').write_text(chr(10).join(rows) + chr(10))
from collections import Counter
c = Counter(t.split('|')[0] for x in rows for t in x.split(chr(9))[8].split(';') if t)
print(len(rows), dict(c), Counter(int(x.split(chr(9))[5]) for x in rows))

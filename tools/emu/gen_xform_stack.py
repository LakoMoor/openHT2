"""0x5ae580 (thiscall, this = scene transform stack, (mat*, kind) ret 8: push) and 0x5ae8a0 (thiscall: pop) run unmodified with the real 0x5aba40,
0x5ac480, 0x5ac5f0; the debug object [0x720908] (when [0x720904] != 0) is a stub (vtable +0x2c, event `dbg`), the error report 0x5d2ba0 is a stub.
Row: words (addr:hex), op (0 push / 1 pop), kind, dbg result, diff of the object (0x2200), diff of globals (0x702a10..0x702a78), counter, events."""
import pathlib, random, struct, math
from emu import Emu, OBJ_BASE, STUB

r = random.Random(0x5ae580)
e = Emu()
F = lambda *v: struct.pack('<%df' % len(v), *v)
U = lambda *v: struct.pack('<%dI' % len(v), *v)
OBJ, MAT, DBG, DVT = [OBJ_BASE + 0x10000 + 0x4000 * i for i in range(4)]
e.write(0x7025f0, F(1, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0))
ev, st = [], {}
hx = lambda a, n: e.read(a, n).hex()
addr = STUB + 0x400 + 0x10 * (0x2c // 4)
def dbg(emu):
    ev.append('dbg|%x|%x|%s' % (emu.arg(0), emu.arg(1), hx(emu.arg(2), 64)))
    return st['dbg']
e.hook_thiscall(addr, dbg, 3)
e.hook_ret(0x5d2ba0, lambda emu: (ev.append('err|%x|%x' % (emu.arg(0), emu.arg(1))), 0)[1])
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
    e.write(OBJ, bytes(0x2200)); e.write(MAT, bytes(0x100)); w = {}
    put = lambda a, b: w.__setitem__(a, b)
    op = r.choice([0, 0, 1])
    kind = r.choice([0, 1, 2])
    put(0x702a79, bytes([3])); put(0x67ef88, U(r.choice([0, 5, 0xfffffffe])))
    put(OBJ + 0x2020, U(r.choice([0, 1, 2, 5, 0x11, 0x12, 0x13]))); put(OBJ + 0x202c, U(r.choice([0, 1, 2, 5, 0x11, 0x12, 0x13])))
    put(OBJ + 0x2024, U(r.choice([0, 1, 3, 7])))
    na = struct.unpack('<I', w[OBJ + 0x202c])[0]; cc = struct.unpack('<I', w[OBJ + 0x2024])[0]
    for q in {na, max(na - 1, 0), na + 1}:
        for j, v in enumerate(rot()): put(OBJ + 0x1460 + 0x30 * q + 4 * j, F(v))
    for j, v in enumerate(rot()): put(OBJ + 0x1070 + 4 * j, F(v))
    for j, v in enumerate(rot()): put(OBJ + 0x10a0 + 0x30 * cc + 4 * j, F(v))
    for j, v in enumerate(rot()): put(MAT + 4 * j, F(v))
    use_dbg = r.random() < 0.4
    put(0x720904, U(1 if use_dbg else 0)); put(0x720908, U(DBG)); put(DBG, U(DVT)); put(DVT + 0x2c, U(addr))
    st['dbg'] = r.choice([0, 0, 1])
    for o, b in w.items(): e.write(o, b)
    o0, g0, c0 = e.read(OBJ, 0x2200), e.read(0x702a10, 0x70), e.read(0x67ef88, 4)
    if op == 0:
        e.call(0x5ae580, [MAT, kind], 'ii', this=OBJ)
    else:
        e.call(0x5ae8a0, [], '', this=OBJ)
    o1, g1, c1 = e.read(OBJ, 0x2200), e.read(0x702a10, 0x70), e.read(0x67ef88, 4)
    d = lambda x, y: ','.join('%x:%s' % (i, y[i:i + 4].hex()) for i in range(0, len(x), 4) if x[i:i + 4] != y[i:i + 4])
    rows.append(chr(9).join([','.join('%x:%s' % (o, b.hex()) for o, b in w.items()), str(op), str(kind), str(st['dbg']), d(o0, o1), d(g0, g1), d(c0, c1), ';'.join(ev)]))
pathlib.Path(__file__).with_name('golden_xform_stack.tsv').write_text(chr(10).join(rows) + chr(10))
print(len(rows), sum(1 for x in rows if x.split(chr(9))[4]), sum(1 for x in rows if x.split(chr(9))[7]))

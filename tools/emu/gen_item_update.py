"""0x582c30 (MovingItem virtual +0xcc, thiscall (flags, dt), ret 8: the per-step pose / velocity update) run unmodified with
the real 0x5ab4d0, 0x5ab620, 0x5ab710, 0x5ab400, 0x5aba40, 0x5abc40, 0x5abc90, 0x5ac2e0, 0x5818d0 (and 0x4e2fb4, 0x5de950),
0x5820c0 (and 0x581ea0, 0x58d0f0), 0x4e28b0, 0x4e27a5; stubs: virtuals +0x1c/+0x90/+0x94/+0xa0/+0xdc, 0x57c9e0, 0x4e5ba7,
0x57e2e0, 0x57e540, 0x588090, 0x580a00, 0x5808c0, 0x581990, 0x4e2821, the log 0x4e260c and sprintf.
Row: flags, dt, this words (hex), stub values (obj, 5ba7, 588090, vdc vec), diff of this (0x1000), events."""
import pathlib, random, struct
from emu import Emu, OBJ_BASE, STUB

r = random.Random(0x582c30)
e = Emu()
F = lambda *v: struct.pack('<%df' % len(v), *v)
U = lambda *v: struct.pack('<%dI' % len(v), *v)
THIS, VT, CLK = OBJ_BASE + 0x10000, OBJ_BASE + 0x14000, OBJ_BASE + 0x16000
e.write(0x7025f0, F(1, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0)); e.write(0x702620, F(0, 1, 0)); e.write(0x702630, F(0, 0, 1))
e.write(0x702640, F(1, 0, 0)); e.write(0x702650, F(0, 0, 0))
e.set_u32(0x6974c4, CLK); e.write(0x6974c2, struct.pack('<h', 0))
ev, st = [], {}
hx = lambda a, n: e.read(a, n).hex()
def virt(slot, nargs, fmt):
    addr = STUB + 0x400 + 0x10 * (slot // 4)
    def f(emu):
        ev.append(fmt(emu))
        if slot == 0xdc:
            e.write(emu.arg(0), st['vdc'])
        return 0
    e.hook_thiscall(addr, f, nargs)
    e.set_u32(VT + slot, addr)
virt(0x1c, 1, lambda emu: 'v1c|%x' % emu.arg(0))
virt(0x90, 2, lambda emu: 'v90|%s|%x' % (hx(emu.arg(0), 12), emu.arg(1)))
virt(0x94, 2, lambda emu: 'v94|%s|%x' % (hx(emu.arg(0), 12), emu.arg(1)))
virt(0xa0, 1, lambda emu: 'va0|%s' % hx(emu.arg(0), 0xd0))
virt(0xdc, 1, lambda emu: 'vdc')
def leaf(addr, name, nargs, ret=lambda: 0):
    def f(emu):
        ev.append('%s|%s' % (name, ','.join('%x' % emu.arg(i) for i in range(nargs))))
        return ret()
    e.hook_ret(addr, f)
leaf(0x57c9e0, 'c9e0', 1, lambda: st['obj'])
leaf(0x4e5ba7, '5ba7', 3, lambda: st['r5ba7'])
for a, n, k in ((0x57e2e0, 'e2e0', 3), (0x57e540, 'e540', 3), (0x580a00, '0a00', 3), (0x5808c0, '08c0', 3), (0x581990, '1990', 4)):
    leaf(a, n, k)
e.hook_thiscall(0x588090, lambda emu: (ev.append('8090|%x' % emu.ecx), st['r8090'])[1], 0)
e.hook_ret(0x4e2821, lambda emu: (ev.append('rng|%s|%x|%x' % (hx(emu.arg(0), 12), emu.arg(1), emu.arg(2))), 0)[1])
e.hook_ret(0x4e260c, lambda emu: (ev.append('err'), 0)[1])
e.hook_ret(0x61cca9, lambda emu: 0)
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
    e.write(THIS, bytes(0x1000)); e.set_u32(THIS, VT)
    w = {}
    k = r.choice([0, 0, 1, 2, 3])
    w[0x598] = struct.pack('<i', k); w[0x690] = struct.pack('<i', r.choice([k, k + 1, k - 1, 5]))
    w[0x188] = U(r.choice([1, 2, 3]))
    w[0x404] = U(r.choice([0, 0, 0, 7])); w[0x3f4] = U(r.choice([0, 0, 0x2a0b0000]))
    w[0x37c] = U(r.choice([0, 0, 4, 1, 5, 0x10]))
    w[0x68c] = U(r.choice([0, 0, 1]))
    w[0x688] = struct.pack('<i', r.choice([0, 1, -1, 0x4000000, 0x1000000, 0x2000000]))
    w[0x58c] = U(r.choice([0, 0, 1])); w[0x698] = U(r.choice([0, 1])); w[0xd88] = U(r.randrange(2 ** 32))
    w[0x5a4] = F(val(2)); w[0x5ac] = F(val(2)); w[0x5b0] = F(val(1)); w[0x3d8] = F(r.choice([0.0, 0.5, 0.65, 0.8, 1.0, val(1)]))
    w[0xc48] = F(r.uniform(-3, 100))
    w[0x7f4] = U(r.choice([0, 0x2a0c0000])); w[0xaa0] = U(0)
    for o in (0x1a0, 0x1a4, 0x1a8, 0x1ac, 0x1b0, 0x1b4, 0x1dc, 0x1e0, 0x1e4, 0x1e8, 0x1ec, 0x1f0, 0x3c4, 0x3c8, 0x3cc, 0x3d0, 0x3d4, 0x3d8 + 0x1000 - 0x1000, 0x5ec, 0x5f0, 0x5f4, 0x5f8, 0x5fc, 0x600,
              0xc4c, 0xc50, 0xc54, 0xc58, 0xc5c, 0xc60, 0xc88, 0xc8c, 0xc90, 0x7e4, 0x7e8, 0x7ec):
        if o != 0x3d8: w[o] = F(val(3))
    for o in range(0x5b8, 0x5b8 + 0xd0, 4): w[o] = F(val(3))
    for o in range(0xc64, 0xc64 + 0x30, 4): w[o] = F(r.choice([1.0, 0.0, val(1)]))
    for base in (0x224, 0x284, 0x6a0, 0x700, 0x730, 0x760, 0x1f4):
        for i, v in enumerate(rot()): w[base + 4 * i] = F(v)
    for i in range(4):
        for o in range(0x7f8 + 12 * i, 0x7f8 + 12 * i + 12, 4): w[o] = F(val(2))
        for o in range(0x86c + 0xd0 * i, 0x86c + 0xd0 * i + 0x18, 4): w[o] = F(val(2))
        w[0x8ac + 0xd0 * i] = U(r.randrange(4))
    w[0x188] = U(r.choice([2, 2, 2, 1, 3]))
    w[0xdf4] = struct.pack('<i', r.choice([0, 1, 3, 6])); w[0xdfc] = U(0); w[0xe04] = U(0)
    for i in range(6): w[0xe08 + 12 * i] = F(val(3), val(3), val(3))
    for o, b in w.items(): e.write(THIS + o, b)
    for o in range(0x7f8, 0x7f8 + 0x30, 4): pass
    flags = r.choice([0, 0, 4, 8, 12, 1])
    dt = r.choice([0.016, 0.0333, 0.05, 0.1, 0.25])
    st['obj'] = r.choice([0, 0, 0x2a0d0000]); st['r5ba7'] = r.choice([0, 0x2a0e0000]); st['r8090'] = r.choice([0, 1])
    st['vdc'] = F(val(1), val(1), val(1))
    e.write(CLK + 0xb98, struct.pack('<d', r.uniform(0, 100)))
    before = e.read(THIS, 0x1000)
    e.call(0x582c30, [flags, struct.unpack('<I', F(dt))[0]], 'ii', this=THIS)
    after = e.read(THIS, 0x1000)
    diff = ','.join('%x:%s' % (i, after[i:i + 4].hex()) for i in range(0, 0x1000, 4) if after[i:i + 4] != before[i:i + 4])
    rows.append(chr(9).join([str(flags), F(dt).hex(), ','.join('%x:%s' % (o, b.hex()) for o, b in w.items()), '%x' % st['obj'], '%x' % st['r5ba7'], str(st['r8090']),
        st['vdc'].hex(), e.read(CLK + 0xb98, 8).hex(), diff, ';'.join(ev)]))
pathlib.Path(__file__).with_name('golden_item_update.tsv').write_text(chr(10).join(rows) + chr(10))
from collections import Counter
c = Counter()
for x in rows:
    for t in x.split(chr(9))[9].split(';'):
        c[t.split('|')[0]] += 1
print(len(rows), dict(c))

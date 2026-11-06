"""0x57eae0 (cdecl (body A, contact record, body B): contact resolution of two bodies) runs unmodified with the real vector / matrix helpers,
0x5ac2e0, 0x4e8ae1-free path; stubs: the virtuals +0x90 / +0x94 of the bodies (events `v90|body|vec|text`, `v94|body|vec|text`), the report
0x4e2821 (event `rep|vec|text|limit`), the sprintf 0x61cca9 (no event) and the log 0x4e260c (event `log`).
Cases whose results contain NaN are skipped (degenerate input).
Row: words (addr:hex), diff of body A (0x1000), of body B (0x1000), of the record (0x100), events."""
import pathlib, random, struct, math
from emu import Emu, OBJ_BASE, STUB

r = random.Random(0x57eae0)
e = Emu()
F = lambda *v: struct.pack('<%df' % len(v), *v)
U = lambda *v: struct.pack('<%dI' % len(v), *v)
A, B, VT, REC = OBJ_BASE + 0x10000, OBJ_BASE + 0x12000, OBJ_BASE + 0x14000, OBJ_BASE + 0x15000
ev = []
hx = lambda a, n: e.read(a, n).hex()
for slot, name in ((0x90, 'v90'), (0x94, 'v94')):
    addr = STUB + 0x400 + 0x10 * (slot // 4)
    e.hook_thiscall(addr, (lambda nm: lambda emu: (ev.append('%s|%s|%s|%x' % (nm, 'a' if emu.ecx == A else 'b', hx(emu.arg(0), 12), emu.arg(1))), 0)[1])(name), 2)
    e.set_u32(VT + slot, addr)
e.hook_ret(0x4e2821, lambda emu: (ev.append('rep|%s|%x|%x' % (hx(emu.arg(0), 12), emu.arg(1), emu.arg(2))), 0)[1])
e.hook_ret(0x61cca9, lambda emu: 0)
e.hook_ret(0x4e260c, lambda emu: (ev.append('log'), 0)[1])
sc = lambda: r.choice([0.5, 1.0, 3.0, 20.0, 150.0, 2000.0])
val = lambda k: r.choice([0.0, 1.0, -1.0, r.uniform(-k, k), r.uniform(-k, k), r.uniform(-k, k)])
def rot():
    a, b, c = [r.uniform(-3, 3) for _ in range(3)]
    ca, sa, cb, sb, cc, sc_ = math.cos(a), math.sin(a), math.cos(b), math.sin(b), math.cos(c), math.sin(c)
    rx = [[1, 0, 0], [0, ca, sa], [0, -sa, ca]]; ry = [[cb, 0, -sb], [0, 1, 0], [sb, 0, cb]]; rz = [[cc, sc_, 0], [-sc_, cc, 0], [0, 0, 1]]
    mul = lambda p, q: [[sum(p[i][k] * q[k][j] for k in range(3)) for j in range(3)] for i in range(3)]
    mm = mul(mul(rx, ry), rz)
    return [x for row in mm for x in row]
rows = []
def has_nan(blob):
    return any((struct.unpack('<I', blob[i:i + 4])[0] & 0x7f800000) == 0x7f800000 and (struct.unpack('<I', blob[i:i + 4])[0] & 0x7fffff) for i in range(0, len(blob) - 3, 4))
skipped = 0
while len(rows) < 800:
    ev.clear()
    for a in (A, B): e.write(a, bytes(0x1000))
    e.write(REC, bytes(0x100)); w = {}
    put = lambda a, b: w.__setitem__(a, b)
    ta = [r.uniform(-5, 5) for _ in range(3)]
    tb = [t + r.uniform(-2, 2) for t in ta]
    pt = [t + r.uniform(-1.5, 1.5) for t in ta]
    k = sc()
    for body, t in ((A, ta), (B, tb)):
        put(body, U(VT))
        put(body + 0xc64, F(*(rot() + t)))
        put(body + 0x1a0, F(*[val(k) for _ in range(3)]))
        put(body + 0x1ac, F(*[val(k) for _ in range(3)]))
        put(body + 0xcc4, F(r.choice([0.0, 1.0, 10.0, r.uniform(0.1, 5), r.uniform(1, 2000)])))
        put(body + 0xcd8, F(*[r.choice([1.0, 0.5, r.uniform(0.1, 2)]) for _ in range(3)]))
        put(body + 0xce4, F(*[val(2) for _ in range(6)]))
        put(body + 0x590, F(r.choice([0.0, 0.5, 1.0, r.uniform(0, 2)])))
        put(body + 0xcd0, F(r.choice([1.0, 0.5, r.uniform(0.1, 3)])))
        put(body + 0x140, U(0x67d000))
    nv = [val(1) for _ in range(3)]
    if r.random() < 0.8:
        n = math.sqrt(sum(x * x for x in nv)) or 1.0
        nv = [x / n for x in nv]
    put(REC + 0x34, F(*pt))
    put(REC + 0x40, F(*nv))
    put(0x696be0, F(r.choice([0.0, 0.5, 1.0, 5.0, 100.0, r.uniform(0, 10)])))
    for o, b in w.items(): e.write(o, b)
    a0, b0, c0 = e.read(A, 0x1000), e.read(B, 0x1000), e.read(REC, 0x100)
    e.call(0x57eae0, [A, REC, B], 'iii')
    a1, b1, c1 = e.read(A, 0x1000), e.read(B, 0x1000), e.read(REC, 0x100)
    if has_nan(a1) or has_nan(b1) or has_nan(c1) or any(has_nan(bytes.fromhex(t.split('|')[2])) for t in ev if t.startswith('v9')) or any(has_nan(bytes.fromhex(t.split('|')[1])) for t in ev if t.startswith('rep')):
        skipped += 1
        continue
    dd = lambda x, y: ','.join('%x:%s' % (i, y[i:i + 4].hex()) for i in range(0, len(x), 4) if x[i:i + 4] != y[i:i + 4])
    rows.append(chr(9).join([','.join('%x:%s' % (o, b.hex()) for o, b in w.items()), dd(a0, a1), dd(b0, b1), dd(c0, c1), ';'.join(ev)]))
pathlib.Path(__file__).with_name('golden_contact_eae0.tsv').write_text(chr(10).join(rows) + chr(10))
from collections import Counter
print('skipped (NaN outputs, NaN sign/payload propagation of the x87 is not ported for degenerate data):', skipped)
print(len(rows), Counter(t.split('|')[0] for x in rows for t in x.split(chr(9))[4].split(';') if t))

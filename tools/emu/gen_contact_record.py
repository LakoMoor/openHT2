"""0x584820 (cdecl (a0*, a1*, set, body): records a contact in the contact set) run unmodified, including 0x5ef5c0, 0x57de90,
0x5f6820 (sphere), 0x4e570f/0x4e52fc/0x4e5698; the heap (0x632643), the volume destructor 0x5f4a90 and the virtual +0x34 of
the neighbour object are recording stubs. Row: mode, f88, body words, matrix, a0, a1, stub mat, stub pn, flags, set-n, result,
set diff, events."""
import pathlib, random, struct
from emu import Emu, OBJ_BASE, STUB

r = random.Random(0x584820)
e = Emu()
F = lambda *v: struct.pack('<%df' % len(v), *v)
BODY, SET, A0, A1, VT, HEAP = [OBJ_BASE + 0x10000 + 0x2000 * i for i in range(6)]
e.write(0x7025f0, F(1, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0)); e.write(0x702650, F(0, 0, 0))
ev, st = [], {}
e.hook_ret(0x632643, lambda emu: (ev.append('alloc'), HEAP)[1])
e.hook_thiscall(0x5f4a90, lambda emu: (ev.append('free'), 0)[1], 0)
V34 = STUB + 0x500
def v34(emu):
    rec = emu.arg(1)
    ev.append('cast|%x|%x|%s' % (emu.ecx, emu.arg(0), e.read(rec, 0x7c).hex()))
    e.write(rec + 4, st['mat']); e.write(rec + 0x34, st['pn'])
    return st['flags']
e.hook_thiscall(V34, v34, 2)
e.set_u32(VT + 0x34, V34)
val = lambda k: r.choice([0.0, 1.0, -1.0, r.uniform(-k, k)])
rows = []
for case in range(500):
    ev.clear()
    e.write(BODY, bytes(0x1000)); e.write(SET, bytes(0x1000)); e.write(HEAP, bytes(0x40))
    mode = r.choice([1, 2, 2, 2, 3]); f88 = r.choice([2, 3, 3])
    alt = r.choice([0, 0, 1])
    body = {0x188: struct.pack('<I', mode), 0xf88: struct.pack('<I', f88), 0x5a0: struct.pack('<I', alt),
            0xd78: F(val(2)), 0xd7c: F(val(2)), 0xd80: F(val(2)), 0xcd4: F(r.choice([0.5, 1.0, 2.0, 3.0]))}
    mat = F(*[r.choice([1.0, 0.0, val(1)]) for _ in range(9)] + [val(5) for _ in range(3)])
    body = {k: v for k, v in body.items()}
    for k, v in body.items(): e.write(BODY + k, v)
    e.write(BODY + (0x700 if alt else 0x1f4), mat)
    e.set_u32(BODY + 0x100, VT)
    p0 = F(*[val(4) for _ in range(3)])
    p1 = F(*[val(6) for _ in range(3)]) if r.random() < 0.7 else p0
    e.write(A0, p0); e.write(A1, p1)
    n = r.choice([0, 0, 1, 7, 0x3d, 0x3e])
    e.set_u32(SET + 0xb00, n)
    st['mat'] = F(1, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0) if r.random() < 0.6 else F(*[r.uniform(-1.5, 1.5) for _ in range(12)])
    st['pn'] = F(*[val(6) for _ in range(6)]); st['flags'] = r.choice([0, 1, 1, 3, 2])
    before = e.read(SET, 0x1000)
    e.call(0x584820, [A0, A1, SET, BODY], 'iiii'); out = e.eax & 0xffffffff
    after = e.read(SET, 0x1000)
    diff = ','.join('%x:%s' % (i, after[i:i + 4].hex()) for i in range(0, 0x1000, 4) if after[i:i + 4] != before[i:i + 4])
    rows.append(chr(9).join([str(mode), str(f88), str(alt), ','.join('%x:%s' % (k, v.hex()) for k, v in body.items()), mat.hex(), p0.hex(), p1.hex(),
        st['mat'].hex(), st['pn'].hex(), '%x' % st['flags'], str(n), '%x' % out, diff, ';'.join(ev)]))
pathlib.Path(__file__).with_name('golden_contact_record.tsv').write_text(chr(10).join(rows) + chr(10))
from collections import Counter
print(len(rows), Counter(x.split(chr(9))[11] for x in rows), Counter(x.split(chr(9))[0] for x in rows))
print(sum('alloc' in x for x in rows), 'sphere path;', sum('cast' in x for x in rows), 'cast path')

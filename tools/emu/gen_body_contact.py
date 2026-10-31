"""0x584570 (the body against its six neighbours, cdecl (body, p0*, p1*, target, rec)) run unmodified, including 0x5f6820 (sphere),
0x4e2749 and 0x57de90; the heap (0x632643), the shape destructor 0x5f4a90 and the neighbour cast 0x4e5a45 are recording stubs.
Row: body words, 6 neighbour word sets, clock/dt, target vec, p0/p1 in, stub hit data, flags, result, rec after (0x80), p0/p1 out, events."""
import pathlib, random, struct
from emu import Emu, OBJ_BASE

r = random.Random(0x584570)
e = Emu()
F = lambda *v: struct.pack('<%df' % len(v), *v)
BODY = OBJ_BASE + 0x10000
NB = [OBJ_BASE + 0x12000 + 0x2000 * i for i in range(6)]
REC, TGT, P0, P1, CLK, HEAP = [OBJ_BASE + 0x20000 + 0x1000 * i for i in range(6)]
e.write(0x7025f0, F(1, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0)); e.write(0x702650, F(0, 0, 0))
e.set_u32(0x6974c4, CLK); e.write(0x6974c2, struct.pack('<h', 0))
ev, st, heap = [], {}, [HEAP]
def alloc(emu):
    p = heap[0]; heap[0] += 0x20; ev.append('alloc'); return p
e.hook_ret(0x632643, alloc)
e.hook_thiscall(0x5f4a90, lambda emu: (ev.append('free'), 0)[1], 0)
def cast(emu):
    obj, p0, p1, rec, rad = [emu.arg(i) for i in range(5)]
    ev.append('cast|%x|%s|%s|%x' % (obj, e.read(p0, 12).hex(), e.read(p1, 12).hex(), rad))
    e.write(rec + 4, st['mat']); e.write(rec + 0x34, st['pn'])
    return st['flags']
e.hook_ret(0x4e5a45, cast)
val = lambda k: r.choice([0.0, 1.0, -1.0, r.uniform(-k, k)])
rows = []
for case in range(300):
    ev.clear(); heap[0] = HEAP
    for a in [BODY, REC, TGT, P0, P1, CLK] + NB:
        e.write(a, bytes(0x2000 if a in NB or a == BODY else 0x100))
    body = {0x5a0: 0, 0xcc8: F(r.choice([0.5, 1.0, 2.0]))[:4], 0xcd0: F(r.choice([0.0, 0.3, 1.0]))[:4]}
    for o in (0xce4, 0xce8, 0xcec):
        body[o] = F(val(3))
    for o in (0x1f4 + 0x24, 0x1f4 + 0x28, 0x1f4 + 0x2c):
        body[o] = F(val(5))
    body_m = [1, 0, 0, 0, 1, 0, 0, 0, 1]
    nbs = []
    nn = r.choice([0, 1, 2, 3, 4, 5, 6])
    ptrs = {}
    for i in range(nn):
        n = {0x188: struct.pack('<I', r.choice([1, 2, 2, 3])), 0xf88: struct.pack('<I', r.choice([2, 2, 3])), 0xcc8: F(r.choice([0.5, 1.0, 2.0])), 0x5a0: struct.pack('<I', 0)}
        for o in (0xce4, 0xce8, 0xcec):
            n[o] = F(val(3))
        for k, o in enumerate((0x1f4 + 0x24, 0x1f4 + 0x28, 0x1f4 + 0x2c)):
            n[o] = F(val(5) if r.random() < 0.5 else struct.unpack('<f', body[0x1f4 + 0x24 + 4 * k])[0] + r.uniform(-1, 1))
        nbs.append(n)
        for o, b in n.items():
            e.write(NB[i] + o, b if isinstance(b, bytes) else struct.pack('<I', b))
        ptrs[0xdb4 + 4 * i] = struct.pack('<I', NB[i])
        ptrs[0xdcc + 4 * i] = F(r.choice([0.0, 0.5, 4.0, 20.0, 200.0]))
    for o, b in {**body, **ptrs}.items():
        e.write(BODY + o, b if isinstance(b, bytes) else struct.pack('<I', b))
    dt = r.choice([0.016, 0.0333, 0.25, 0.5])
    e.write(CLK + 0xb90, struct.pack('<d', dt))
    tgt = F(val(5), val(5), val(5)); e.write(TGT + 0x24, tgt)
    p0in, p1in = F(val(5), val(5), val(5)), F(val(5), val(5), val(5)); e.write(P0, p0in); e.write(P1, p1in)
    e.write(REC, bytes(0x80))
    st['mat'] = F(1, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0); st['pn'] = F(*[val(5) for _ in range(6)]); st['flags'] = r.choice([0, 1, 1, 3])
    e.call(0x584570, [BODY, P0, P1, TGT, REC], 'iiiii')
    allb = {}
    rows.append(chr(9).join([
        ','.join('%x:%s' % (o, (b if isinstance(b, bytes) else struct.pack('<I', b)).hex()) for o, b in {**body, **ptrs}.items()),
        ';'.join(','.join('%x:%s' % (o, (b if isinstance(b, bytes) else struct.pack('<I', b)).hex()) for o, b in n.items()) for n in nbs),
        struct.pack('<d', dt).hex(), tgt.hex(), p0in.hex(), p1in.hex(), st['pn'].hex(), str(st['flags']), '%x' % e.eax,
        e.read(REC, 0x80).hex(), e.read(P0, 12).hex(), e.read(P1, 12).hex(), ';'.join(ev)]))
pathlib.Path(__file__).with_name('golden_body_contact.tsv').write_text(chr(10).join(rows) + chr(10))
from collections import Counter
print(len(rows), Counter(x.split(chr(9))[8] for x in rows), sum('alloc' in x for x in rows), 'with shape', sum('cast' in x for x in rows), 'with cast')

"""0x585150 (cdecl (body, matrix*, set): the body against the collected road-mesh nodes, appends contacts) run unmodified
with the real 0x4e2749, 0x56f650, 0x5e19d0/0x5f25c0, 0x57de90/0x57dec0, 0x5aba40, 0x5ac2e0, 0x584820, 0x584c70, 0x5818b0
and 0x4e27a5; the traversal 0x56f3f0 (node list is pre-written, only the count is returned), the binder 0x587e20, the heap,
the volume destructor and the object virtual +0x34 are recording stubs.
Row: scene writes (addr:hex,...), count, bind result, stub mat/pn/flags, result, set diff, body diff, events."""
import pathlib, random, struct
from emu import Emu, OBJ_BASE, STUB

r = random.Random(0x585150)
e = Emu()
F = lambda *v: struct.pack('<%df' % len(v), *v)
U = lambda *v: struct.pack('<%dI' % len(v), *v)
BODY, SET, MAT, VT, HEAP, CLK = [OBJ_BASE + 0x10000 + 0x2000 * i for i in range(6)]
NODES = OBJ_BASE + 0x30000
NODE_LIST, NODE_EXTRA = 0x6f2e58, 0x6f326c
ev, st = [], {}
e.write(0x7025f0, F(1, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0)); e.write(0x702650, F(0, 0, 0))
e.set_u32(0x6974c4, CLK); e.write(0x6974c2, struct.pack('<h', 0))
e.hook_ret(0x632643, lambda emu: (ev.append('alloc'), HEAP)[1])
e.hook_thiscall(0x5f4a90, lambda emu: (ev.append('free'), 0)[1], 0)
V34 = STUB + 0x500
def v34(emu):
    rec = emu.arg(1)
    ev.append('cast|%x|%s' % (emu.arg(0), e.read(rec, 0x7c).hex()))
    e.write(rec + 4, st['mat']); e.write(rec + 0x34, st['pn'])
    return st['flags']
e.hook_thiscall(V34, v34, 2)
e.set_u32(VT + 0x34, V34)
def begin(emu):
    ev.append('begin|%x|%s' % (emu.arg(0), ','.join('%x' % emu.arg(i) for i in range(1, 5))))
    e.set_u32(emu.arg(5), st['count'])
    return NODE_LIST
e.hook_ret(0x56f3f0, begin)
def bind(emu):
    ev.append('bind|%x|%s' % (emu.ecx, ','.join('%x' % emu.arg(i) for i in range(7))))
    return st['bindret']
e.hook_thiscall(0x587e20, bind, 7)
val = lambda k: r.choice([0.0, 1.0, -1.0, r.uniform(-k, k)])
rows = []
for case in range(400):
    ev.clear()
    writes = {}
    def put(a, b):
        writes[a] = b
        e.write(a, b)
    for a in (BODY, SET): put(a, bytes(0x1000 if a == BODY else 0x1000))
    put(HEAP, bytes(0x40)); put(NODES, bytes(0x4000)); writes.pop(NODES)
    put(NODE_LIST, bytes(0x80)); put(NODE_EXTRA, bytes(0x80))
    mode = r.choice([1, 1, 2, 3, 3]); f88 = r.choice([2, 3, 3]); alt = r.choice([0, 0, 1])
    body = {0x188: U(mode), 0xf88: U(f88), 0x5a0: U(alt), 0x320: U(NODES + 0x3800), 0x7f0: U(NODES + 0x3900),
            0xd6c: F(val(2)), 0xd70: F(val(2)), 0xd74: F(val(2)), 0xd78: F(val(2)), 0xd7c: F(val(2)), 0xd80: F(val(2)),
            0xd84: F(r.choice([0.0, 0.5, 1.0, 3.0])), 0xcc8: F(r.choice([0.5, 1.0, 2.0])), 0xcd4: F(r.choice([0.5, 1.0, 2.0])),
            0x37c: U(r.randrange(2 ** 32)), 0x100: U(VT)}
    rot = lambda: [r.choice([1.0, 0.0, val(1)]) for _ in range(9)]
    mat = F(*rot(), *[val(5) for _ in range(3)])
    bm = F(*rot(), *[val(5) for _ in range(3)]); bm2 = F(*rot(), *[val(5) for _ in range(3)])
    for k, v in body.items(): put(BODY + k, v)
    put(BODY + (0x700 if alt else 0x1f4), bm); put(BODY + (0x730 if alt else 0x254), bm2)
    mm = F(*rot(), *[val(5) for _ in range(3)]); put(MAT, mm)
    n = r.choice([1, 2, 3, 3, 3])
    st['count'] = n
    st['bindret'] = r.choice([0, 1, 1, 2])
    ptrs = 0
    for i in range(n):
        node = NODES + 0x400 * i
        kind = r.choice([9, 9, 9, 8]); k40 = r.choice([7, 7, 7, 6])
        verts_at = NODES + 0x1000 + 0x100 * i
        vc = r.choice([-1, 1, 2, 3, 3, 4])
        o2 = r.choice([0, 0, 5]); cnt1 = r.choice([1, 1, 2, 0]); scaled = r.choice([0, 50, 120, 300, -40])
        data_at = NODES + 0x2000 + 0x40 * i
        extra = r.choice([0, 0, NODES + 0x3000 + 0x100 * i])
        nd = {0x10: U(kind), 0x40: U(k40), 0x48: U(verts_at), 0x30: U(o2), 0x34: U(scaled & 0xffffffff), 0x3c: U(data_at - 4),
              0x38: U(cnt1), 0x44: U(vc & 0xffffffff)}
        for k, v in nd.items(): put(node + k, v)
        put(NODE_LIST + 4 * i, U(node)); put(NODE_EXTRA + 4 * i, U(extra))
        if extra:
            ex = {0x10: U(r.choice([6, 6, 5])), 0x2c: U(0), 0x38: U(r.choice([0, 1]))}
            for k, v in ex.items(): put(extra + k, v)
            put(extra + 0x40, F(*rot(), *[val(3) for _ in range(3)]))
        put(verts_at, F(*[val(2.5) for _ in range(12)]))
        put(data_at, U(r.randrange(2 ** 32), r.randrange(2 ** 32)))
    dt = r.choice([0.016, 0.0333, 0.25, 0.5]); put(CLK + 0xb90, struct.pack('<d', dt)); put(CLK + 0xb98, struct.pack('<d', r.uniform(0, 100)))
    st['mat'] = F(1, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0) if r.random() < 0.6 else F(*[r.uniform(-1.5, 1.5) for _ in range(12)])
    st['pn'] = F(*[val(6) for _ in range(6)]); st['flags'] = r.choice([1, 1, 3, 0])
    put(SET + 0xb00, U(r.choice([0, 0, 1, 7])))
    bb, sb = e.read(BODY, 0x1000), e.read(SET, 0x1000)
    e.call(0x585150, [BODY, MAT, SET], 'iii'); out = e.eax & 0xffffffff
    ba, sa = e.read(BODY, 0x1000), e.read(SET, 0x1000)
    dif = lambda b, a: ','.join('%x:%s' % (i, a[i:i + 4].hex()) for i in range(0, 0x1000, 4) if a[i:i + 4] != b[i:i + 4])
    rows.append(chr(9).join([','.join('%x:%s' % (a - OBJ_BASE if a >= OBJ_BASE else a, b.hex()) for a, b in writes.items()), str(n), str(st['bindret']),
        st['mat'].hex(), st['pn'].hex(), '%x' % st['flags'], '%x' % out, dif(sb, sa), dif(bb, ba), ';'.join(ev)]))
pathlib.Path(__file__).with_name('golden_collect_contacts.tsv').write_text(chr(10).join(rows) + chr(10))
from collections import Counter
print(len(rows), Counter(int(x.split(chr(9))[6], 16) for x in rows).most_common(8))
print(sum('bind' in x.split(chr(9))[9] for x in rows), 'with bind;', sum('cast' in x.split(chr(9))[9] for x in rows), 'with cast;', sum('alloc' in x.split(chr(9))[9] for x in rows), 'with sphere')

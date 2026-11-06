"""0x5d32f0 (thiscall, this = holder, (time) ret 4: schedule) runs unmodified with the real pool 0x5d2a70 / 0x5d2ac0, node constructor 0x5d3090,
the sorted insert 0x5d3660, the destructor 0x5d30c0 and the list walker 0x5d3630; stubs: the log 0x5d2e70 (event `e70|msg|arg`) and the node report
0x5d2ba0 (event `ba0|a|b`).
Row: words (addr:hex), time, diffs of holder block (0x400), pool block (0x100), pool header (0x14), events."""
import pathlib, random, struct
from emu import Emu, OBJ_BASE

r = random.Random(0x5d32f0)
e = Emu()
U = lambda *v: struct.pack('<%dI' % len(v), *v)
H, L, O1, O2 = OBJ_BASE + 0x10000, OBJ_BASE + 0x10100, OBJ_BASE + 0x10200, OBJ_BASE + 0x10300
PB, PM, POOL = OBJ_BASE + 0x20000, OBJ_BASE + 0x21000, 0x7200c0
ev = []
e.hook_ret(0x5d2e70, lambda emu: (ev.append('e70|%x|%x' % (emu.arg(0), emu.arg(1))), 0)[1])
e.hook_ret(0x5d2ba0, lambda emu: (ev.append('ba0|%x|%x' % (emu.arg(0), emu.arg(1))), 0)[1])
rows = []
for case in range(500):
    ev.clear()
    e.write(H, bytes(0x400)); e.write(PB, bytes(0x100)); e.write(PM, bytes(0x10)); w = {}
    put = lambda a, b: w.__setitem__(a, b)
    base = r.choice([0, 0, 5, 100])
    put(H + 0xc, U(r.choice([L, L, L, L, 0]))); put(H + 0x14, U(r.choice([0, 1, 2]))); put(H + 0x1c, U(0x1111))
    put(L, U(base))
    for o in (O1, O2): put(o + 0x14, U(r.choice([0, 1, 2]))); put(o + 0x1c, U(o & 0xffff))
    n_slots = 8
    k = r.choice([0, 0, 1, 2, 3, 4, 8])
    slots = r.sample(range(n_slots), k)
    arg = r.choice([0, 1, 3, 10, 0xfffffff0])
    nodes = []
    for s in slots:
        a = PB + 0x18 * s
        own = r.choice([H, H, O1, O2])
        t = (arg + base) & 0xffffffff if r.random() < 0.3 else r.choice([0, 1, 3, 5, 10, 105, 0x7fffffff, 0xfffffff0])
        put(a, U(0x651eb4)); put(a + 0xc, U(own)); put(a + 0x10, U(t)); put(a + 0x14, U(r.choice([0, 0, 0, 1])))
        nodes.append(a)
    r.shuffle(nodes)
    for i, a in enumerate(nodes):
        put(a + 4, U(nodes[i - 1] if i else 0)); put(a + 8, U(nodes[i + 1] if i + 1 < len(nodes) else 0))
    put(L + 0xc, U(nodes[0] if nodes else 0))
    for i in range(n_slots): put(PM + i, bytes([1 if i in slots else 0]))
    put(POOL, U(0x18, n_slots, PB, PM, r.randrange(n_slots)))
    for o, b in w.items(): e.write(o, b)
    s0 = [e.read(H, 0x400), e.read(PB, 0x100), e.read(PM, 0x10), e.read(POOL, 0x14)]
    e.call(0x5d32f0, [arg], 'i', this=H)
    s1 = [e.read(H, 0x400), e.read(PB, 0x100), e.read(PM, 0x10), e.read(POOL, 0x14)]
    d = lambda x, y: ','.join('%x:%s' % (i, y[i:i + 4].hex()) for i in range(0, len(x), 4) if x[i:i + 4] != y[i:i + 4])
    rows.append(chr(9).join([','.join('%x:%s' % (o, b.hex()) for o, b in w.items()), str(arg)] + [d(a, b) for a, b in zip(s0, s1)] + [';'.join(ev)]))
pathlib.Path(__file__).with_name('golden_event_queue.tsv').write_text(chr(10).join(rows) + chr(10))
from collections import Counter
print(len(rows), Counter(t.split('|')[0] for x in rows for t in x.split(chr(9))[-1].split(';') if t), sum(1 for x in rows if x.split(chr(9))[3]))

"""0x5d3790 (thiscall Q, (owner) ret 4: delete the owner's nodes from both lists of Q), 0x5d35b0 (thiscall Q, (holder): as above, then unlink the holder)
and 0x5d33b0 (thiscall holder: detach from [+0xc]) run unmodified with the real 0x5d3a60, 0x5d21b0, 0x5d3080, 0x5d30c0 and the pool; stub: the log
0x5d2e70 (event `e70|msg|arg`).
Row: words (addr:hex), op, owner/holder, diffs of Q (0x100), pool block (0x100), map (0x10), pool header (0x14), holders (0x100), events."""
import pathlib, random, struct
from emu import Emu, OBJ_BASE

r = random.Random(0x5d3790)
e = Emu()
U = lambda *v: struct.pack('<%dI' % len(v), *v)
Q, HB = OBJ_BASE + 0x10000, OBJ_BASE + 0x11000
OW = [OBJ_BASE + 0x12000 + 0x100 * i for i in range(3)]
PB, PM, POOL = OBJ_BASE + 0x20000, OBJ_BASE + 0x21000, 0x7200c0
ev = []
e.hook_ret(0x5d2e70, lambda emu: (ev.append('e70|%x|%x' % (emu.arg(0), emu.arg(1))), 0)[1])
rows = []
for case in range(600):
    ev.clear()
    e.write(Q, bytes(0x100)); e.write(HB, bytes(0x100)); e.write(PB, bytes(0x100)); e.write(PM, bytes(0x10)); w = {}
    put = lambda a, b: w.__setitem__(a, b)
    n_slots = 8
    slots = r.sample(range(n_slots), r.choice([0, 1, 3, 5, 8]))
    nodes = [PB + 0x18 * s for s in slots]
    r.shuffle(nodes)
    cut = r.randrange(len(nodes) + 1)
    chains = [nodes[:cut], nodes[cut:]]
    for ch in chains:
        for i, a in enumerate(ch):
            put(a, U(0x651eb4)); put(a + 0xc, U(r.choice(OW)))
            put(a + 4, U(ch[i - 1] if i else 0)); put(a + 8, U(ch[i + 1] if i + 1 < len(ch) else 0))
    put(Q + 0xc, U(chains[0][0] if chains[0] else 0)); put(Q + 8, U(chains[1][0] if chains[1] else 0))
    holders = [HB + 0x20 * i for i in range(r.choice([0, 1, 2, 3]))]
    for i, a in enumerate(holders):
        put(a + 0xc, U(Q)); put(a + 4, U(holders[i - 1] if i else 0)); put(a + 8, U(holders[i + 1] if i + 1 < len(holders) else 0))
    put(Q + 4, U(holders[0] if holders else 0))
    for i in range(n_slots): put(PM + i, bytes([1 if i in slots else 0]))
    put(POOL, U(0x18, n_slots, PB, PM, 0))
    op = case % 3
    if op == 0:
        arg = r.choice(OW)
    else:
        arg = r.choice(holders) if holders else HB
    put(0x720118, U(r.choice([arg, 0, 0x55])))
    for o, b in w.items(): e.write(o, b)
    blocks = [(Q, 0x100), (PB, 0x100), (PM, 0x10), (POOL, 0x14), (HB, 0x100)]
    s0 = [e.read(a, n) for a, n in blocks]
    if op == 0: e.call(0x5d3790, [arg], 'i', this=Q)
    elif op == 1: e.call(0x5d35b0, [arg], 'i', this=Q)
    else: e.call(0x5d33b0, [], '', this=arg)
    s1 = [e.read(a, n) for a, n in blocks]
    d = lambda x, y: ','.join('%x:%s' % (i, y[i:i + 4].hex()) for i in range(0, len(x), 4) if x[i:i + 4] != y[i:i + 4])
    rows.append(chr(9).join([','.join('%x:%s' % (o, b.hex()) for o, b in w.items()), str(op), str(arg)] + [d(a, b) for a, b in zip(s0, s1)] + [';'.join(ev)]))
pathlib.Path(__file__).with_name('golden_event_queue2.tsv').write_text(chr(10).join(rows) + chr(10))
print(len(rows), sum(1 for x in rows if x.split(chr(9))[3]), sum(1 for x in rows if x.split(chr(9))[-1]))

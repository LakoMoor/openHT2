"""Observe updateRating 0x41fcf0 with synthetic companies and capacities.

Capacity queries are supplied numerically; the original role predicate, smoothing,
company traversal, division and victory callbacks execute unchanged. No game assets.
"""
import pathlib
import random
import struct
from emu import Emu, OBJ_BASE

e = Emu()
sent = OBJ_BASE
nodes = [OBJ_BASE + 0x1000 * (i + 1) for i in range(8)]
members = OBJ_BASE + 0x10000
u = lambda a: struct.unpack('<I', e.read(a, 4))[0]
d = lambda a: struct.unpack('<d', e.read(a, 8))[0]
put = lambda a, v: e.write(a, struct.pack('<d', v))
events = []
capacities = {}

def capacity(emu):
    put(emu.arg(1), capacities[emu.arg(0)])
    put(emu.arg(2), 0.0)
    return 0

def human(emu):
    emu.set_u32(emu.arg(0), nodes[0])
    return emu.arg(0)

def event(emu):
    assert emu.arg(1) == 12
    events.append(nodes.index(emu.arg(0)))
    return 0

e.hook_ret(0x41b910, capacity)
e.hook_ret(0x477b60, human)
e.hook_ret(0x4015a0, event)
e.set_u32(0x68b990, sent)
r = random.Random(0x41fcf0)
rows = []
for case in range(240):
    n = 2 + case % 7
    kinds = [1] + [r.choice((2, 2, 2, 5, 6, 9)) for _ in range(n - 1)]
    caps = [r.choice((0.0, 100.0, 4900.0, 5100.0, 8000.0)) for _ in range(n)]
    if case % 12 == 0:
        caps = [0.0] * n
    if case % 12 == 1:
        caps = [5100.0, 4900.0] + [0.0] * (n - 2)
        kinds[1] = 2
    bosses = [-1] * n
    for k in range(1, n):
        if r.randrange(3) == 0:
            bosses[k] = 0
    earned = [r.uniform(-1000, 10000) for _ in range(n)]
    earning = [r.uniform(-20, 100) for _ in range(n)]
    old_fleet = [r.uniform(1, 10000) for _ in range(n)]
    old_share = [r.uniform(0, 0.5) for _ in range(n)]
    won = [int(case % 13 == 0 and k == 0) for k in range(n)]
    threshold = 1.05 if case % 11 == 0 else 0.51
    put(0x66c710, threshold)
    e.write(0x68d1a0, b'\0')
    e.set_u32(sent, nodes[0])
    for k in range(n):
        a = nodes[k]
        e.write(a, bytes(0x500))
        e.set_u32(a, nodes[k + 1] if k + 1 < n else sent)
        e.set_u32(a + 0x5c, kinds[k])
        e.set_u32(a + 0xb0, sent if bosses[k] < 0 else nodes[bosses[k]])
        team = [nodes[j] for j in range(n) if bosses[j] == k]
        start = members + k * 0x100
        e.set_u32(a + 0xa0, start)
        e.set_u32(a + 0xa4, start + 16 * len(team))
        for j, member in enumerate(team):
            e.set_u32(start + 16 * j, member)
        for off, value in ((0x2e0, 1 / 256), (0x2e8, 255 / 256), (0x2f0, earning[k]),
                           (0x2f8, earned[k]), (0x300, old_share[k]), (0x308, old_fleet[k])):
            put(a + off, value)
        e.write(a + 0x310, bytes([won[k]]))
        capacities[a] = caps[k]
    row = [n, threshold]
    for k in range(n):
        row.extend((kinds[k], caps[k], bosses[k], earned[k], earning[k], old_share[k], old_fleet[k], won[k]))
    events.clear()
    for repeat in range(3):
        e.call(0x41fcf0)
        row.extend((d(0x66c710), e.read(0x68d1a0, 1)[0], len(events), *events))
        for a in nodes[:n]:
            row.extend((d(a + 0x2f0), d(a + 0x2f8), d(a + 0x300), d(a + 0x308), e.read(a + 0x310, 1)[0]))
    rows.append(' '.join(map(str, row)))
path = pathlib.Path(__file__).resolve().parents[2] / 'crates/ht2-sim/tests/data/rating_golden.txt'
path.write_text('\n'.join(rows) + '\n', encoding='utf-8')
print(f'{len(rows)} scenarios, 3 calls each: {path}')

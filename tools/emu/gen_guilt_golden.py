"""Exercise accountForGuilt (0x442850), markAmbushes (0x42c0b0), and their share predicates.

Only logging and the scene's human-player lookup are stubbed. Synthetic vehicle/trailer
records, cargo and generator states keep this independent of map snapshots. Writes numeric
fixtures, never executable bytes or pseudocode. Run from any directory with Python + unicorn.
"""
import pathlib
import random
import struct

from emu import Emu, OBJ_BASE

e = Emu()
u = lambda a: struct.unpack('<I', e.read(a, 4))[0]
i = lambda a: struct.unpack('<i', e.read(a, 4))[0]
f = lambda a, x: e.write(a, struct.pack('<d', x))
player, human = OBJ_BASE, OBJ_BASE + 0x1000
vehicle, trailer = OBJ_BASE + 0x2000, OBJ_BASE + 0x3000
cargo, unused = OBJ_BASE + 0x4000, OBJ_BASE + 0x5000
goods = OBJ_BASE + 0x6000
ambush_list, ambush_base = OBJ_BASE + 0x7000, OBJ_BASE + 0x8000
ambush_kinds = (1, 2, 0, 1, 2, 2, 1)


def find_human(emu):
    emu.set_u32(emu.arg(0), human)
    return emu.arg(0)


e.hook_ret(0x477b60, find_human)
for address, nargs in ((0x401870, 2), (0x4018b0, 1)):
    e.hook_thiscall(address, lambda emu: 0, nargs)
e.set_u32(0x68ccb8, goods)
for m, flags in enumerate((0, 1, 2, 3)):
    e.set_u32(goods + 0x68 * m + 0x30, flags)
e.set_u32(0x68a970, ambush_list)
e.set_u32(0x68a974, ambush_list + 4 * len(ambush_kinds))
for j, kind in enumerate(ambush_kinds):
    a = ambush_base + j * 0x100
    e.set_u32(ambush_list + j * 4, a)
    e.set_u32(a + 0x24, kind)
assert [struct.unpack('<d', e.read(a, 8))[0] for a in (0x64b8a0, 0x64b8a8, 0x64b5f0, 0x64b838)] == [500000, 200000, 10000, 0.15]
r = random.Random(0x442850)
rows = []
seen = set()
mafia_active = 0
for case in range(720):
    share = (0.0, 0.15, 0.15000000000000002, 0.2, 0.20000000000000004, 0.51)[case % 6]
    flags = (0, 1, 2, 3)[case % 4]
    tow = case % 3  # 0: vehicle, 1: trailer, 2: invalid trailer falls back to vehicle
    own = case % 2
    score = (-1.0, 0.0, 123.0)[(case // 6) % 3]
    p = human if own else player
    count = case % 7
    loads = [(r.randrange(4), r.choice((-1, -1, 0, 1, 2, 3)),
              r.choice((-1.0, 0.0, 9999.999999, 10000.0, 10000.000001, 25000.0)),
              r.choice((-1, -1, 0, 1))) for _ in range(count)]
    # Boundary draws in addition to arbitrary full-width states. The first 23 outputs
    # are exactly these words since the corresponding subtractands are zero.
    table = [r.getrandbits(32) for _ in range(55)]
    if case % 4 == 0:
        table = [0] * 55
        for j in range(1, 24):
            table[j] = r.choice((0, 199999, 200000, 499999, 500000, 999999, 1000000, 0xffffffff))
    e.write(0x68b9c8, struct.pack('<55I', *table))
    e.set_u32(0x68baa4, 0)
    e.set_u32(0x68baa8, 31)
    for a in (player, human, vehicle, trailer):
        e.write(a, bytes(0x500))
    f(human + 0x300, share)
    f(human + 0xe0, score)
    e.set_u32(p + 0x204, vehicle)
    e.set_u32(p + 0x208, trailer if tow else 0)
    e.set_u32(trailer + 4, -1 if tow == 2 else 0)
    e.set_u32(vehicle + 0x28, flags)
    # An opposite alarm flag on the trailer catches accidentally reading it instead.
    e.set_u32(trailer + 0x28, flags ^ 2)
    for a in (vehicle, trailer):
        e.set_u32(a + 0x1c, unused)
        e.set_u32(a + 0x20, unused)
    carrier = trailer if tow == 1 else vehicle
    e.set_u32(carrier + 0x1c, cargo)
    e.set_u32(carrier + 0x20, cargo + 0x90 * count)
    e.write(cargo, bytes(0x90 * 7))
    for j, (m, illegal, price, decision) in enumerate(loads):
        a = cargo + j * 0x90
        e.set_u32(a, m)
        e.set_u32(a + 4, illegal)
        f(a + 0x40, price)
        e.set_u32(a + 0x88, decision)
    # Pre-existing flags must be overwritten, including when there is no cargo.
    e.write(p + 0xc8, b'\1\1')
    e.write(p + 0x108, b'\1')
    outputs = []
    for repeat in range(3):
        e.call(0x442850, [p], 'i')
        bits = [e.read(p + off, 1)[0] for off in (0xc8, 0xc9, 0x108)]
        decisions = [i(cargo + j * 0x90 + 0x88) for j in range(count)]
        state = [u(0x68baa4), u(0x68baa8), *struct.unpack('<55I', e.read(0x68b9c8, 220))]
        outputs.extend([*bits, *decisions, *state])
        seen.add(tuple(bits))
        for j in range(len(ambush_kinds)):
            e.write(ambush_base + j * 0x100 + 0xf0, b'\1')
        e.call(0x42c0b0)
        active = [e.read(ambush_base + j * 0x100 + 0xf0, 1)[0] for j in range(len(ambush_kinds))]
        mafia_active += sum(on for kind, on in zip(ambush_kinds, active) if kind == 1)
        state = [u(0x68baa4), u(0x68baa8), *struct.unpack('<55I', e.read(0x68b9c8, 220))]
        outputs.extend([*active, *state])
    row = [share, flags, tow, own, score, count, *table]
    for load in loads:
        row.extend(load)
    rows.append(' '.join(map(str, [*row, *outputs])))
assert seen == {(0, 0, 0), (0, 0, 1), (0, 1, 0), (0, 1, 1), (1, 0, 0), (1, 0, 1)}
assert mafia_active > 0
path = pathlib.Path(__file__).resolve().parents[2] / 'crates/ht2-sim/tests/data/guilt_golden.txt'
path.parent.mkdir(exist_ok=True)
path.write_text('\n'.join(rows) + '\n', encoding='utf-8')
print(f'{len(rows)} scenarios, 3 consecutive call pairs each; output flags: {sorted(seen)}; mafia activations: {mafia_active}')
print(path)

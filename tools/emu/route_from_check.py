"""Routes between places on roads (and nodes) by the original search (0x4858b0), for comparing with
ht2_sim::road::RoadNet::route_from. Places come from the original random start (0x422d60); every
fourth end is a node instead. Printed as
`mode | from road node t piece segment flags | to ... | cost | node:road ...`.

    python route_from_check.py <game folder>/ENV 300 > routes_from.txt
"""
import random
import struct
import sys

from dump_ai_world import load
from emu import OBJ_BASE

folder, count = sys.argv[1], int(sys.argv[2])
e = load(folder)
e.call(0x403940, max_insn=50_000_000)
u32 = lambda a: struct.unpack('<I', e.read(a, 4))[0]


def vec(begin):
    b, en = u32(begin), u32(begin + 4)
    return [u32(b + 4 * k) for k in range((en - b) // 4)]


nodes = vec(0x68cf00)
roads = {p: i for i, p in enumerate(vec(0x68cf38))}
OUT, COST, PLACE = OBJ_BASE + 0x100, OBJ_BASE + 0x200, OBJ_BASE + 0x300
rng = random.Random(7)


def place():
    if rng.random() < 0.25:
        return struct.pack('<iidiiii', -1, rng.randrange(len(nodes)), 0.0, 0, 0, 0, -1)
    e.call(0x422d60, [PLACE], 'i', max_insn=50_000_000)
    return bytes(e.read(PLACE, 32))


def text(p):
    road, node, t, piece, seg, flags, last = struct.unpack('<iidiiii', p)
    return f'{road} {node} {t!r} {piece} {seg} {flags}'


for _ in range(count):
    a, b = place(), place()
    mode = rng.choice([0, 1])
    e.write(OUT, bytes(12))
    e.write(COST, bytes(8))
    args = [OUT, mode] + list(struct.unpack('<8I', a)) + list(struct.unpack('<8I', b)) + [COST]
    try:
        e.call(0x4858b0, args, 'i' * len(args), max_insn=400_000_000)
    except Exception as ex:
        print(mode, '|', text(a), '|', text(b), '| error', ex)
        continue
    cost = struct.unpack('<d', e.read(COST, 8))[0]
    rb, re_ = u32(OUT), u32(OUT + 4)
    steps = []
    for s in range(rb, re_, 12):
        n, r, fl = struct.unpack('<iII', e.read(s, 12))
        steps.append(f'{n}:{roads.get(r, -1)}')
    print(mode, '|', text(a), '|', text(b), '|', repr(cost), '|', ' '.join(steps))

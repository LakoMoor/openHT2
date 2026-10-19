"""Routes between random node pairs by the original search (0x484c80) on a map's road network,
printed as `src dst mode cost node:road ...` for comparing with ht2_sim::road::RoadNet::route_between.

    python route_check.py <game folder>\\MENV 200 > routes.txt
"""
import random
import struct
import sys

from dump_ai_world import load
from emu import OBJ_BASE

folder, count = sys.argv[1], int(sys.argv[2])
e = load(folder)
u32 = lambda a: struct.unpack('<I', e.read(a, 4))[0]


def vec(begin):
    b, en = u32(begin), u32(begin + 4)
    return [u32(b + 4 * k) for k in range((en - b) // 4)]


nodes = vec(0x68cf00)
roads = {p: i for i, p in enumerate(vec(0x68cf38))}
OUT, COST = OBJ_BASE + 0x100, OBJ_BASE + 0x200
rng = random.Random(5)
for _ in range(count):
    src, dst = rng.randrange(len(nodes)), rng.randrange(len(nodes))
    mode = rng.choice([0, 1])
    e.write(OUT, bytes(12))
    e.write(COST, bytes(8))
    try:
        e.call(0x484c80, [OUT, mode, src, dst, COST], 'iiiii', max_insn=200_000_000)
    except Exception as ex:
        print(src, dst, mode, 'error', ex)
        continue
    cost = struct.unpack('<d', e.read(COST, 8))[0]
    b, en = u32(OUT), u32(OUT + 4)
    steps = []
    for a in range(b, en, 12):
        n, r, fl = struct.unpack('<III', e.read(a, 12))
        steps.append(f'{n}:{roads.get(r, -1)}')
    print(src, dst, mode, repr(cost), ' '.join(steps))

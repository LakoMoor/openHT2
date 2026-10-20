"""The cost of every road by the original (0x409980) for modes 0 and 1, both ways, printed as
`road mode against cost` (repr) for comparing bit for bit with RoadNet::road_cost.

    python road_cost_check.py <game folder>/ENV > road_costs.txt
"""
import struct
import sys

from dump_ai_world import load

e = load(sys.argv[1])
u32 = lambda a: struct.unpack('<I', e.read(a, 4))[0]
b, en = u32(0x68cf38), u32(0x68cf3c)
for k in range((en - b) // 4):
    road = u32(b + 4 * k)
    for mode in (0, 1):
        for against in (0, 1):
            cost = e.call(0x409980, [mode, road, against], 'iii', max_insn=20_000_000)
            print(k, mode, against, repr(cost))

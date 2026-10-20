"""Random start places by the original (0x422d60, after seeding the generator with 0x403940), printed
as `road node t piece segment flags` for comparing with RoadNet::random_start.

    python place_check.py <game folder>/ENV 300 > places.txt
"""
import struct
import sys

from dump_ai_world import load
from emu import OBJ_BASE

folder, count = sys.argv[1], int(sys.argv[2])
e = load(folder)
e.call(0x403940, max_insn=50_000_000)
OUT = OBJ_BASE + 0x100
for _ in range(count):
    e.call(0x422d60, [OUT], 'i', max_insn=50_000_000)
    road, node, t, piece, seg, flags, last = struct.unpack('<iidiiii', e.read(OUT, 32))
    print(road, node, repr(t), piece, seg, flags)

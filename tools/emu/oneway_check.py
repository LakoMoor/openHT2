"""The one-way test of a place by the original (0x533510, over getRoadParameters 0x438d70) for random
start places, printed as `road node t piece segment flags result`, for comparing with
RoadNet::place_is_one_way (roadnet_dump with a file name containing "oneway").

    python oneway_check.py <game folder>/ENV 300 > oneway.txt
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
    e.call(0x533510, [OUT], 'i', max_insn=50_000_000)
    print(road, node, repr(t), piece, seg, flags, e.eax & 0xff)

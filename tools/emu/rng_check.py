"""The road module's generator and destination draw run in the emulator, printed for comparing with
ht2_sim::road::AiRng and RoadNet::random_destination.

    python rng_check.py <game folder>\\MENV 50 > draws.txt
"""
import struct
import sys

from dump_ai_world import load
from emu import OBJ_BASE

folder, count = sys.argv[1], int(sys.argv[2])
e = load(folder)
try:
    e.call(0x403940, max_insn=50_000_000)    # seeds the generator (and sets up more of the module)
except Exception as ex:
    print('# 0x403940 stopped:', ex)
u32 = lambda a: struct.unpack('<I', e.read(a, 4))[0]
print('table', ' '.join(str(u32(0x68b9c8 + 4 * k)) for k in range(55)), u32(0x68baa4), u32(0x68baa8))
OUT, PLAYER = OBJ_BASE + 0x100, OBJ_BASE + 0x200
e.write(PLAYER, bytes(0x100))
for _ in range(count):
    e.call(0x456140, [OUT, 0, PLAYER, 0x68b9c8], 'iiii', max_insn=50_000_000)
    print('draw', struct.unpack('<i', e.read(OUT, 4))[0])

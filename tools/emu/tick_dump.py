"""AI ticks (0x42d6e0) from the state after a new game (the snapshot ai_traffic.py makes), with
an empty game scene: after each tick the place and route of every player, for comparing the
ported AI world.

    python tick_dump.py <game folder>/ENV <ticks> <dt> > ticks.txt
"""
import os
import struct
import sys

from dump_ai_world import load
from newgame_steps import setup

folder, ticks, dt = sys.argv[1], int(sys.argv[2]), float(sys.argv[3])
snap = os.path.join(os.environ.get('TEMP', '.'), 'ht2_ai_' + os.path.basename(os.path.normpath(folder)) + '.snap')
e = load(folder, full=True, events=[], snapshot=snap)
setup(e)
u = lambda a: struct.unpack('<I', e.read(a, 4))[0]
i32 = lambda a: struct.unpack('<i', e.read(a, 4))[0]
clock = u(u(0x6cec6c) + 0x54c)
roads = {u(u(0x68cf38) + 4 * i): i for i in range((u(0x68cf3c) - u(0x68cf38)) // 4)}


def dump(tick):
    sent = u(0x68b990)
    n = u(sent)
    k = 0
    while n != sent:
        p = n + 8
        t = struct.unpack('<d', e.read(p + 0x28, 8))[0]
        rb, re_ = u(p + 0x10), u(p + 0x14)
        route = ' '.join('%d:%d' % (i32(a), roads.get(u(a + 4), -1)) for a in range(rb, re_, 12))
        print('tick', tick, 'player', k, 'type', i32(p + 0x54), 'state', hex(i32(p + 0x5c)), 'flags', hex(u(p + 0x58)),
              'place', i32(p + 0x20), i32(p + 0x24), repr(t), i32(p + 0x30), i32(p + 0x34), i32(p + 0x38),
              'route', route, flush=True)
        n = u(n)
        k += 1


time = 0.0
for tick in range(1, ticks + 1):
    time += dt
    e.write(clock + 0xb98, struct.pack('<d', time))
    e.write(clock + 0xb90, struct.pack('<d', dt))
    e.call(0x42d6e0, max_insn=10**12)
    dump(tick)

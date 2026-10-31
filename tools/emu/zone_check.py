"""0x533340 (the zone object of a point: nearest place in a room, else on all roads, then the object of the
place's room name in the name table 0x71e820) on a real map. The name table is filled with every second room
of the list file (value 0x1000 + index). Prints `room x y z fallback result object road node t piece segment`.

    ROOMS=1 roadnet_dump <ENV> > rooms.txt;  FULL=1 python zone_check.py <game folder>/ENV rooms.txt 300 > zone.txt
(FULL=1 starts the whole AI world, about 4 minutes: it fills the room name map 0x4364f0 reads.)
"""
import random
import struct
import sys

from dump_ai_world import load
from emu import OBJ_BASE

folder, rooms_file, count = sys.argv[1], sys.argv[2], int(sys.argv[3])
rooms = [l.strip() for l in open(rooms_file) if l.strip()]
e = load(folder, full=bool(__import__("os").environ.get("FULL")))
BASE = OBJ_BASE + 0x100000
TABLE, NODES, STR = BASE, BASE + 0x4000, BASE + 0x40000
e.write(TABLE, bytes(2000))
e.set_u32(0x71e828, TABLE)
tails = {}
for i, n in enumerate(rooms):
    if i % 2:
        continue
    node, sp = NODES + 0x20 * i, STR + 0x80 * i
    e.write(sp, n.encode() + b'\0')
    e.write(node, bytes(0x20))
    e.set_u32(node + 0xc, sp)
    e.set_u32(node + 0x10, 0x1000 + i)
    b = sum(n.encode()) % 500
    if b in tails:
        e.set_u32(tails[b] + 8, node)
    else:
        e.set_u32(TABLE + 4 * b, node)
    tails[b] = node
r = random.Random(0x533340)
NAME, POS, OUT, PID = OBJ_BASE + 0x200, OBJ_BASE + 0x300, OBJ_BASE + 0x340, OBJ_BASE + 0x400
for _ in range(count):
    room = r.choice(rooms + ['NO_SUCH_ROOM'])
    q = struct.unpack('<3f', struct.pack('<3f', r.uniform(-1500, 1500), r.uniform(-1500, 1500), r.uniform(-20, 100)))
    fb = r.randrange(2)
    e.write(NAME, room.encode() + b'\0')
    e.write(POS, struct.pack('<3f', *q))
    e.write(PID, bytes(0x20))
    e.set_u32(OUT, 0xdead)
    e.call(0x533340, [NAME, POS, OUT, PID, fb], 'iiiii', max_insn=50_000_000)
    res = e.eax
    obj = struct.unpack('<I', e.read(OUT, 4))[0]
    road, node, t, piece, seg, flags, last = struct.unpack('<iidiiii', e.read(PID, 32))
    print(room, *[repr(x) for x in q], fb, res, obj, road, node, repr(t), piece, seg)

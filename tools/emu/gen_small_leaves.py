"""Small leaves of 0x54f040 run unmodified: 0x58d170 (cursor over a point list), 0x53f7b0 (second-kind vehicle check),
0x57de90 (matrix copy chosen by a flag)."""
import pathlib, random, struct
from emu import Emu, OBJ_BASE

r = random.Random(0x58d170)
e = Emu()
O, D = OBJ_BASE + 0x10000, OBJ_BASE + 0x14000
rows = []
for case in range(400):
    which = r.choice(['58d170', '53f7b0', '57de90'])
    blob = bytearray(r.randrange(256) for _ in range(0x1000))
    if which == '58d170':
        struct.pack_into('<I', blob, 0xdf8, r.randrange(0, 8))
        struct.pack_into('<I', blob, 0xe00, r.choice([0, 1, 2, 3, 5, 9]))
        for i in range(8):
            struct.pack_into('<I', blob, 0xf28 + 4 * i, r.choice([0, 0, 1, 0xdead]))
    elif which == '53f7b0':
        struct.pack_into('<I', blob, 0x188, r.choice([2, 2, 1]))
        struct.pack_into('<I', blob, 0xdf0, r.choice([2, 2, 3]))
    else:
        struct.pack_into('<I', blob, 0x5a0, r.choice([0, 1]))
    e.write(O, bytes(blob)); e.write(D, bytes(0x40))
    arg = O
    if which == '53f7b0':
        arg = r.choice([O, 0])
        res = e.call(0x53f7b0, [arg], 'i')
    elif which == '58d170':
        res = e.call(0x58d170, [D], 'i', this=O)
    else:
        res = e.call(0x57de90, [D], 'i', this=O)
    rows.append(chr(9).join([which, bytes(blob).hex(), '%x' % arg, '%x' % e.eax, e.read(D, 0x40).hex(), e.read(O + 0xdf8, 12).hex()]))
pathlib.Path(__file__).with_name('golden_small_leaves.tsv').write_text(chr(10).join(rows) + chr(10))
print(len(rows), 'cases')

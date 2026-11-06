"""0x587c80 (constructor of the contact set, thiscall, area 0x1910) run unmodified on a garbage-filled area.
Row: constant vector (0x702650, hex), changed dwords vs the garbage (hex runs `off:hex`)."""
import pathlib, random, struct
from emu import Emu, OBJ_BASE

r = random.Random(0x587c80)
e = Emu()
B = OBJ_BASE + 0x10000
SIZE = 0x1910
rows = []
for case in range(20):
    vec = struct.pack('<3f', *[r.choice([0.0, 1.5, -2.25, r.uniform(-3, 3)]) for _ in range(3)])
    e.write(0x702650, vec)
    e.write(0x7025f0, struct.pack('<12f', *[r.choice([1.0, 0.0, r.uniform(-1, 1)]) for _ in range(12)]))
    ident = e.read(0x7025f0, 48)
    junk = bytes(r.randrange(1, 256) for _ in range(SIZE))
    e.write(B, junk)
    e.call(0x587c80, [], '', this=B)
    after = e.read(B, SIZE)
    diff = ','.join('%x:%s' % (i, after[i:i + 4].hex()) for i in range(0, SIZE, 4) if after[i:i + 4] != junk[i:i + 4])
    rows.append(chr(9).join([vec.hex(), ident.hex(), junk.hex(), diff]))
pathlib.Path(__file__).with_name('golden_contact_set_ctor.tsv').write_text(chr(10).join(rows) + chr(10))
print(len(rows), len(rows[0]))

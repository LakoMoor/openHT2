"""0x57df70 (thiscall, one stack arg): copy the 12 floats at this+0x224 to the buffer, run unmodified."""
import pathlib, random, struct
from emu import Emu, OBJ_BASE

r = random.Random(0x57df70)
e = Emu()
O, B = OBJ_BASE + 0x10000, OBJ_BASE + 0x12000
rows = []
for case in range(100):
    blob = bytes(r.randrange(256) for _ in range(0x300))
    e.write(O, blob); e.write(B, bytes(0x40))
    e.call(0x57df70, [B], 'i', this=O)
    rows.append(chr(9).join([blob.hex(), e.read(B, 0x40).hex()]))
pathlib.Path(__file__).with_name('golden_object_matrix_copy.tsv').write_text(chr(10).join(rows) + chr(10))
print(len(rows), 'cases')

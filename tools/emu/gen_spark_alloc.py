"""0x5940d0: take the head of the free list [this+0x68] and push it on the active list [this+0x6c]."""
import pathlib, random, struct
from emu import Emu, OBJ_BASE
r = random.Random(0x5940d0)
e = Emu()
THIS, NODES = OBJ_BASE + 0x1000, OBJ_BASE + 0x2000
rows = []
for case in range(60):
    n = r.choice([0, 1, 2, 5])
    nodes = [NODES + 0x40 * i for i in range(n)]
    for i, a in enumerate(nodes):
        e.write(a, struct.pack('<I', nodes[i + 1] if i + 1 < n else 0))
    active = r.choice([0, NODES + 0x1000])
    e.write(THIS + 0x68, struct.pack('<I', nodes[0] if n else 0))
    e.write(THIS + 0x6c, struct.pack('<I', active))
    e.call(0x5940d0, [], '', this=THIS)
    ret = e.eax
    out = [ret, struct.unpack('<I', e.read(THIS + 0x68, 4))[0], struct.unpack('<I', e.read(THIS + 0x6c, 4))[0]] + [struct.unpack('<I', e.read(a, 4))[0] for a in nodes]
    rows.append(chr(9).join([str(n), str(active), ','.join(map(str, out))]))
pathlib.Path(__file__).with_name('golden_spark_alloc.tsv').write_text(chr(10).join(rows) + chr(10))
print(len(rows))

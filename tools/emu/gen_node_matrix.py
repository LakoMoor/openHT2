"""0x5f25c0 (node world matrix, thiscall (out), recursive over the parent chain [+0x2c], mode [+0x38]) run unmodified.
Row: chain length, per node (mode, parent flag, matrix hex), result, out matrix, mode words after."""
import pathlib, random, struct
from emu import Emu, OBJ_BASE

r = random.Random(0x5f25c0)
e = Emu()
NODES = [OBJ_BASE + 0x10000 + 0x200 * i for i in range(5)]
OUT = OBJ_BASE + 0x20000
F = lambda *v: struct.pack('<%df' % len(v), *v)
rows = []
for case in range(200):
    n = r.randrange(1, 5)
    desc = []
    for i in range(n):
        e.write(NODES[i], bytes(0x100))
        mode = r.choice([0, 1, 2, 2, 3, 7])
        mat = F(*[r.uniform(-2, 2) for _ in range(12)])
        e.write(NODES[i] + 0x40, mat)
        e.set_u32(NODES[i] + 0x38, mode)
        e.set_u32(NODES[i] + 0x2c, NODES[i + 1] if i + 1 < n else 0)
        desc.append((mode, mat))
    e.write(OUT, bytes(0x30))
    e.call(0x5f25c0, [OUT], 'i', this=NODES[0])
    modes = [struct.unpack('<I', e.read(NODES[i] + 0x38, 4))[0] for i in range(n)]
    rows.append(chr(9).join([str(n), ';'.join('%d,%s' % (m_, mat.hex()) for m_, mat in desc), '%x' % e.eax, e.read(OUT, 0x30).hex(), ','.join(map(str, modes))]))
pathlib.Path(__file__).with_name('golden_node_matrix.tsv').write_text(chr(10).join(rows) + chr(10))
print(len(rows))

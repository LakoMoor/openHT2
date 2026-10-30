"""Scene node constructors run unmodified: 0x5ef6d0 (base node), 0x5f19b0 (node with pointer array),
0x551920 (child light node), 0x5518d0 (light with a transform). Only malloc is replaced."""
import pathlib, random, struct
from emu import Emu, OBJ_BASE

r = random.Random(0x5ef6d0)
e = Emu()
e.install_crt_hooks() if False else None
HEAP = 0x2a800000
nxt = [HEAP]


def malloc(em):
    a = nxt[0]
    nxt[0] += (em.arg(0) + 15) & ~15
    return a


e.hook_ret(0x632643, malloc)
e.write(0x7025f0, struct.pack('<12f', 1, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0))
N = OBJ_BASE + 0x10000
NAME = OBJ_BASE + 0x1000
SZ = 0x100
rows = []
for case in range(300):
    kind = r.choice(['base', 'array', 'child', 'lamp'])
    before = bytes(r.randrange(256) for _ in range(SZ))
    e.write(N, before)
    nxt[0] = HEAP
    e.write(HEAP, bytes(0x400))
    name = r.choice([b'', b'ThrowLamp', b'x', b'abcdefgh', b'Spot light 12'])
    e.write(NAME, name + b'\0')
    parent = N + 0x80
    nm = NAME if name else 0
    a1, a2 = r.choice([0, 1, 6, 0x11, 0x15]), r.choice([0, 1, 5, 12])
    if kind == 'base':
        e.call(0x5ef6d0, [nm, a1], 'ii', this=N)
    elif kind == 'array':
        e.call(0x5f19b0, [nm, a1, a2], 'iii', this=N)
    elif kind == 'child':
        e.call(0x551920, [nm, parent], 'ii', this=N)
    else:
        e.call(0x5518d0, [], '', this=N)
    heap = e.read(HEAP, nxt[0] - HEAP) if nxt[0] > HEAP else b''
    rows.append(chr(9).join([kind, name.hex(), str(a1), str(a2), before.hex(), e.read(N, SZ).hex(), heap.hex(), str(nxt[0] - HEAP), bytes(e.read(0x68d1c0, 32)).split(bytes(1))[0].hex()]))
pathlib.Path(__file__).with_name('golden_light_ctors.tsv').write_text(chr(10).join(rows) + chr(10))
print(len(rows), 'cases')

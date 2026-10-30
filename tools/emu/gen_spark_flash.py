"""Spark flash 0x594c90 -> 0x5949f0 -> 0x594b60. Heap 0x632643, the light's base constructors
0x5f19b0 / 0x5ad4c0 and the scene registration (vtable slot +0x14) are replaced."""
import pathlib, random, struct
from emu import Emu, OBJ_BASE
r = random.Random(0x594c90)
e = Emu()
HEAP, CLOCK, POS, SCENE, VT = OBJ_BASE + 0x40000, OBJ_BASE + 0x3000, OBJ_BASE + 0x2000, OBJ_BASE + 0x3800, OBJ_BASE + 0x3900
heap = [HEAP]
st = {'reg': None, 'lamp': []}


def malloc(em):
    a = heap[0]
    heap[0] += (em.arg(0) + 15) & ~15
    return a


def lamp_base(em):
    st['lamp'].append(em.ecx)
    return em.ecx


def register(em):
    st['reg'] = (em.ecx, em.arg(0), em.arg(1))
    return 0


e.hook_ret(0x632643, malloc)
e.hook_thiscall(0x5f19b0, lamp_base, 3)
e.hook_thiscall(0x5ad4c0, lamp_base, 0)
e.write(0x1f000300, bytes([0xc2, 0x08, 0x00]))
e.hook_thiscall(0x1f000300, register, 2)
e.write(VT + 0x14, struct.pack('<I', 0x1f000300))
e.write(SCENE, struct.pack('<I', VT))
F = lambda *v: struct.pack('<%df' % len(v), *v)
fbits = lambda x: struct.unpack('<I', F(x))[0]
rows = []
for case in range(100):
    heap[0] = HEAP
    st.update(reg=None, lamp=[])
    pos = F(*[r.uniform(-30, 30) for _ in range(3)])
    p = r.choice([r.uniform(-0.2, 1.6), 0.05, 0.04999999, 1.0, 1.0000001, 0.5])
    a5, a6 = r.randrange(1 << 32), r.randrange(1 << 32)
    t = r.uniform(0, 500)
    scene = SCENE if r.random() < 0.7 else 0
    tail = (OBJ_BASE + 0x5000) if r.random() < 0.8 else 0
    prev = (OBJ_BASE + 0x5100) if r.random() < 0.6 else 0
    e.write(CLOCK + 0x210, struct.pack('<I', scene)); e.write(CLOCK + 0xb98, struct.pack('<d', t))
    e.write(POS, pos)
    e.write(0x6f66b0, struct.pack('<I', tail))
    e.write(OBJ_BASE + 0x5000, struct.pack('<II', 0x11111111, prev)); e.write(OBJ_BASE + 0x5100, struct.pack('<II', 0x22222222, 0x33333333))
    e.write(HEAP, b'\xcc' * 0x400)
    if tail == 0:
        pass
    e.call(0x594c90, [CLOCK, POS, p, a5, a6], 'iifii')
    nodes = heap[0] - HEAP
    out = [e.read(HEAP, nodes).hex(), e.read(OBJ_BASE + 0x5000, 8).hex(), e.read(OBJ_BASE + 0x5100, 8).hex(),
           e.read(0x6f66b0, 4).hex(), ','.join(map(str, st['reg'])) if st['reg'] else '-']
    rows.append(chr(9).join([pos.hex(), F(p).hex(), str(a5), str(a6), struct.pack('<d', t).hex(), str(1 if scene else 0), str(tail), str(prev)] + out))
pathlib.Path(__file__).with_name('golden_spark_flash.tsv').write_text(chr(10).join(rows) + chr(10))
print(len(rows), 'cases;', sum(1 for x in rows if x.split(chr(9))[8] != ''), 'with objects')

"""Golden tests for the collision volumes: polygon records (0x5e69a0) and the segment test (0x5f6820)."""
import math
import os
import struct
import sys

sys.path.insert(0, os.path.dirname(__file__))
from harness import Harness, pack, rs_f32  # noqa: E402
from emu import OBJ_BASE, HEAP_BASE  # noqa: E402

h = Harness('golden_shape', ['use ht2_sim::shape;'])
e = h.e
STREAM = []


def fread(emu):
    buf, size, count = emu.arg(0), emu.arg(1), emu.arg(2)
    n = size * count
    data = bytes(STREAM[:n])
    del STREAM[:n]
    emu.write(buf, data)
    return count


e.hook_ret(0x61e20f, fread)
CONST = [(0x7025f0, struct.pack('<12f', 1, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0)), (0x702650, bytes(12))]
VOL, POLYS, START, END = OBJ_BASE + 0x1000, OBJ_BASE + 0x2000, OBJ_BASE + 0x3000, OBJ_BASE + 0x3010
r = h.rnd


def polygon(cx, cy, cz, tilt):
    n = r.randint(3, 6)
    rad = r.uniform(1, 20)
    a0 = r.uniform(0, 6.28)
    pts = []
    for i in range(n):
        a = a0 + i * 2 * math.pi / n * (1 if r.random() < 0.8 else -1)
        x, y = cx + rad * math.cos(a), cy + rad * math.sin(a)
        pts.append((x, y, cz + tilt[0] * (x - cx) + tilt[1] * (y - cy) + r.choice([0, 0, r.uniform(-0.01, 0.01)])))
    return pts


def build_polys(polys):
    """Polygon records made by the original reader; returns the writes for them."""
    e.write(HEAP_BASE, bytes(0x10000))
    e.heap_next = HEAP_BASE
    writes = []
    for k, pts in enumerate(polys):
        STREAM.extend(struct.pack('<I', len(pts)) + b''.join(struct.pack('<3f', *p) for p in pts))
        e.call(0x5e69a0, [STREAM_OBJ], 'i', this=POLYS + 0x30 * k)
    writes.append((POLYS, e.read(POLYS, 0x30 * len(polys))))
    writes.append((HEAP_BASE, e.read(HEAP_BASE, e.heap_next - HEAP_BASE)))
    return writes


STREAM_OBJ = OBJ_BASE + 0x8000
for case in range(300):
    if case < 60:
        # the reader itself: plane, centre and radius
        pts = polygon(r.uniform(-500, 500), r.uniform(-500, 500), r.uniform(-50, 50), (r.uniform(-0.5, 0.5), r.uniform(-0.5, 0.5)))
        verts = b''.join(struct.pack('<3f', *p) for p in pts)

        def call(e, pts=pts):
            e.write(HEAP_BASE, bytes(0x1000))
            e.heap_next = HEAP_BASE
            STREAM.extend(struct.pack('<I', len(pts)) + b''.join(struct.pack('<3f', *p) for p in pts))
            e.call(0x5e69a0, [STREAM_OBJ], 'i', this=POLYS)
        h.case(CONST + [(HEAP_BASE, verts)], call, f'shape::poly_init(&mut m, {POLYS:#x}, {HEAP_BASE:#x}, {len(pts)})',
               [(POLYS + 2, 0x2e)])
        continue
    kind = r.choice([6, 6, 6, 5, 0])
    mode = r.choice([4, 5, 1, 0])
    cx, cy, cz = r.uniform(-500, 500), r.uniform(-500, 500), r.uniform(-50, 50)
    w = list(CONST)
    if kind == 6:
        tilt = (r.uniform(-0.3, 0.3), r.uniform(-0.3, 0.3))
        polys = [polygon(cx + r.uniform(-15, 15), cy + r.uniform(-15, 15), cz + r.uniform(-1, 1), tilt) for _ in range(r.randint(1, 4))]
        w += build_polys(polys)
        w += [(VOL, struct.pack('<iiI', 6, len(polys), POLYS))]
    elif kind == 5:
        n = [r.uniform(-0.3, 0.3), r.uniform(-0.3, 0.3), 1.0]
        ln = math.sqrt(sum(x * x for x in n))
        n = [x / ln for x in n]
        d = -(n[0] * cx + n[1] * cy + n[2] * cz)
        w += [(POLYS, struct.pack('<4f', *n, d)), (VOL, struct.pack('<iiI', 5, 0, POLYS))]
    else:
        w += [(VOL, struct.pack('<iiI', 0, 0, 0))]
    px, py = cx + r.uniform(-25, 25), cy + r.uniform(-25, 25)
    top, bot = cz + r.uniform(-1, 10), cz - r.uniform(-1, 10)
    if r.random() < 0.2:
        top, bot = bot, top
    s = (px, py, top)
    t = (px + r.choice([0, 0, r.uniform(-3, 3)]), py + r.choice([0, 0, r.uniform(-3, 3)]), bot)
    w += [(START, struct.pack('<3f', *s)), (END, struct.pack('<3f', *t))]
    rad = r.uniform(0.1, 1.0)
    h.case(w, lambda e, mode=mode, rad=rad: e.call(0x5f6820, [mode, START, END, rad], 'iiif', this=VOL),
           f'shape::fn_005f6820(&mut m, {VOL:#x}, {mode}, {START:#x}, {END:#x}, {rs_f32(rad)})',
           [(START, 0x1c)], ret=True, ret_int=True)
h.save()

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
CONST = [(0x7025f0, struct.pack('<12f', 1, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0)), (0x702650, bytes(12)), (0x702630, struct.pack('<3f', 0, 0, 1))]
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
# walls along a polyline (kind 7, 0x5f5a80): segments moving across them
for case in range(150):
    cx, cy, cz = r.uniform(-500, 500), r.uniform(-500, 500), r.uniform(-50, 50)
    n = r.randint(2, 5)
    a0 = r.uniform(0, 6.28)
    pts, x, y = [], cx, cy
    for i in range(n):
        pts.append((x, y, cz + r.uniform(-3, 3)))
        a = a0 + r.uniform(-1.2, 1.2)
        step = r.uniform(2, 30)
        x, y = x + step * math.cos(a), y + step * math.sin(a)
    k = r.randrange(n - 1)
    f = r.uniform(-0.2, 1.2)
    bx, by = pts[k][0] + (pts[k + 1][0] - pts[k][0]) * f, pts[k][1] + (pts[k + 1][1] - pts[k][1]) * f
    ang = r.uniform(0, 6.28)
    d1, d2 = r.uniform(-4, 4), r.uniform(-4, 4)
    s = (bx + d1 * math.cos(ang), by + d1 * math.sin(ang), cz + r.uniform(-5, 5))
    t = (bx + d2 * math.cos(ang), by + d2 * math.sin(ang), cz + r.uniform(-5, 5))
    if r.random() < 0.1:
        t = s
    mode = r.choice([4, 5, 1, 0])
    w = list(CONST) + [(POLYS, b''.join(struct.pack('<3f', *p) for p in pts)), (VOL, struct.pack('<iiI', 7, n, POLYS)),
                       (START, struct.pack('<3f', *s)), (END, struct.pack('<3f', *t))]
    h.case(w, lambda e, mode=mode: e.call(0x5f6820, [mode, START, END, 0.5], 'iiif', this=VOL),
           f'shape::fn_005f6820(&mut m, {VOL:#x}, {mode}, {START:#x}, {END:#x}, 0.5)',
           [(START, 0x1c)], ret=True, ret_int=True)
# sphere sweeps (mode bit 2) against polygons (faces 0x5f5370, edges and corners 0x5f4b30) and planes
for case in range(200):
    cx, cy, cz = r.uniform(-500, 500), r.uniform(-500, 500), r.uniform(-50, 50)
    kind = r.choice([6, 6, 6, 5])
    w = list(CONST)
    if kind == 6:
        tilt = (r.uniform(-0.6, 0.6), r.uniform(-0.6, 0.6))
        polys = [polygon(cx + r.uniform(-6, 6), cy + r.uniform(-6, 6), cz + r.uniform(-1, 1), tilt) for _ in range(r.randint(1, 3))]
        w += build_polys(polys)
        w += [(VOL, struct.pack('<iiI', 6, len(polys), POLYS))]
    else:
        n = [r.uniform(-0.3, 0.3), r.uniform(-0.3, 0.3), 1.0]
        ln = math.sqrt(sum(x * x for x in n))
        n = [x / ln for x in n]
        d = -(n[0] * cx + n[1] * cy + n[2] * cz)
        w += [(POLYS, struct.pack('<4f', *n, d)), (VOL, struct.pack('<iiI', 5, 0, POLYS))]
    rad = r.uniform(0.2, 2.5)
    px, py = cx + r.uniform(-30, 30), cy + r.uniform(-30, 30)
    s = (px, py, cz + r.uniform(0, 8))
    t = (px + r.uniform(-6, 6), py + r.uniform(-6, 6), cz - r.uniform(-2, 6))
    mode = r.choice([6, 7, 2, 3])
    w += [(START, struct.pack('<3f', *s)), (END, struct.pack('<3f', *t))]
    h.case(w, lambda e, mode=mode, rad=rad: e.call(0x5f6820, [mode, START, END, rad], 'iiif', this=VOL),
           f'shape::fn_005f6820(&mut m, {VOL:#x}, {mode}, {START:#x}, {END:#x}, {rs_f32(rad)})',
           [(START, 0x1c)], ret=True, ret_int=True)
# sweeps aimed just outside a polygon's edges and corners
for case in range(150):
    cx, cy, cz = r.uniform(-500, 500), r.uniform(-500, 500), r.uniform(-50, 50)
    tilt = (r.uniform(-0.4, 0.4), r.uniform(-0.4, 0.4))
    pts = polygon(cx, cy, cz, tilt)
    w = list(CONST) + build_polys([pts]) + [(VOL, struct.pack('<iiI', 6, 1, POLYS))]
    rad = r.uniform(0.3, 2.0)
    k = r.randrange(len(pts))
    a, b = pts[k], pts[(k + 1) % len(pts)]
    f = r.choice([0.0, 1.0, r.uniform(0, 1)])
    ex, ey, ez = a[0] + (b[0] - a[0]) * f, a[1] + (b[1] - a[1]) * f, a[2] + (b[2] - a[2]) * f
    ox, oy = ex - cx, ey - cy
    lo = math.hypot(ox, oy) or 1.0
    off = rad * r.uniform(0.2, 1.3)
    px, py = ex + ox / lo * off, ey + oy / lo * off
    s = (px + r.uniform(-1, 1), py + r.uniform(-1, 1), ez + rad + r.uniform(0.5, 4))
    t = (px + r.uniform(-1, 1), py + r.uniform(-1, 1), ez - r.uniform(0, 3))
    mode = r.choice([6, 2])
    w += [(START, struct.pack('<3f', *s)), (END, struct.pack('<3f', *t))]
    h.case(w, lambda e, mode=mode, rad=rad: e.call(0x5f6820, [mode, START, END, rad], 'iiif', this=VOL),
           f'shape::fn_005f6820(&mut m, {VOL:#x}, {mode}, {START:#x}, {END:#x}, {rs_f32(rad)})',
           [(START, 0x1c)], ret=True, ret_int=True)
# spheres swept against walls along a polyline, across the faces and past the ends
for case in range(200):
    cx, cy, cz = r.uniform(-500, 500), r.uniform(-500, 500), r.uniform(-50, 50)
    n = r.randint(2, 4)
    a0 = r.uniform(0, 6.28)
    pts, x, y = [], cx, cy
    for i in range(n):
        pts.append((x, y, cz + r.uniform(-3, 3)))
        a = a0 + r.uniform(-1.0, 1.0)
        step = r.uniform(3, 25)
        x, y = x + step * math.cos(a), y + step * math.sin(a)
    k = r.randrange(n - 1)
    f = r.choice([r.uniform(-0.3, 1.3), r.uniform(-0.15, 0.05), r.uniform(0.95, 1.15)])
    bx, by = pts[k][0] + (pts[k + 1][0] - pts[k][0]) * f, pts[k][1] + (pts[k + 1][1] - pts[k][1]) * f
    ang = r.uniform(0, 6.28)
    d1, d2 = r.uniform(-5, 5), r.uniform(-5, 5)
    s = (bx + d1 * math.cos(ang), by + d1 * math.sin(ang), cz + r.uniform(-5, 5))
    t = (bx + d2 * math.cos(ang), by + d2 * math.sin(ang), cz + r.uniform(-5, 5))
    mode = r.choice([6, 7, 2])
    rad = r.uniform(0.3, 2.5)
    w = list(CONST) + [(POLYS, b''.join(struct.pack('<3f', *p) for p in pts)), (VOL, struct.pack('<iiI', 7, n, POLYS)),
                       (START, struct.pack('<3f', *s)), (END, struct.pack('<3f', *t))]
    h.case(w, lambda e, mode=mode, rad=rad: e.call(0x5f6820, [mode, START, END, rad], 'iiif', this=VOL),
           f'shape::fn_005f6820(&mut m, {VOL:#x}, {mode}, {START:#x}, {END:#x}, {rs_f32(rad)})',
           [(START, 0x1c)], ret=True, ret_int=True)
h.save()

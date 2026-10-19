"""Golden tests for the chassis step functions (chassis.rs)."""
import os
import sys

sys.path.insert(0, os.path.dirname(__file__))
from harness import Harness, pack  # noqa: E402
from autofuzz import state, CAR  # noqa: E402

h = Harness('golden_chassis', ['use ht2_sim::chassis;'])
r = h.rnd
T = f'{CAR:#x}'
WATCH = [(CAR, 0x4000)]
CONST = [(0x702620 + 4, pack('f', 1.0)), (0x702630 + 8, pack('f', 1.0)), (0x702640, pack('f', 1.0))]
CONST += [(0x7025f0 + 4 * i, pack('f', float(v))) for i, v in enumerate([1, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0])]


def unit(r):
    import math
    v = [r.uniform(-1, 1) for _ in range(3)]
    n = math.sqrt(sum(x * x for x in v)) or 1.0
    return [x / n for x in v]


for _ in range(80):
    w, _f = state(0x4efc10, r, fill=[(0x10, 0x40), (0x40, 0x58), (0xd40, 0xd4c)], overrides={0xdd4: lambda r: ('i', r.randint(0, 6))})
    n = unit(r) if r.random() < 0.9 else [0.0, 0.0, 1.0]
    w += [(CAR + 0xd0 + 4 * i, pack('f', n[i])) for i in range(3)]
    h.case(CONST + w, lambda e: e.call(0x4efc10, this=CAR), f'chassis::fn_004efc10(&mut m, {T})', WATCH)
h.save()

# 0x4f2670
h = Harness('golden_chassis_wheels', ['use ht2_sim::chassis;'])
r = h.rnd
P2, P3, P4, P5, P6, P8 = [CAR + 0x3a00 + 0x40 * k for k in range(6)]


def rotm(a):
    import math
    t, u = r.uniform(-3, 3), r.uniform(-1, 1)
    c, s = math.cos(t), math.sin(t)
    v = [c, s, 0, -s, c, 0, 0, 0, 1, r.uniform(-5, 5), r.uniform(-5, 5), r.uniform(-5, 5)]
    return [(a + 4 * i, pack('f', x)) for i, x in enumerate(v)]


def vec3(a, lo=-3, hi=3):
    return [(a + 4 * i, pack('f', r.uniform(lo, hi))) for i in range(3)]


for k in range(160):
    mode = 1 if k < 40 else r.choice([2, 3, 2])
    w, _f = state(0x4f2670, r, fill=[(0x100, 0x100 + 0xf8 * 6), (0x1844, 0x1844 + 72)],
                  overrides={0xdd4: lambda r: ('i', r.randint(0, 6)), 0x2638: lambda r, mode=mode: ('i', mode)})
    for i in range(6):
        w.append((CAR + 0x1bd4 + 4 * i, pack('i', r.choice([0, 1, 1]))))
        w.append((CAR + 0x1cc0 + 12 * i, pack('f', r.uniform(-1, 1))))
    n = unit(r)
    w += [(CAR + 0xd0 + 4 * i, pack('f', n[i])) for i in range(3)]
    p7 = r.uniform(-2, 2)
    import struct
    sp7 = struct.unpack('<f', pack('f', p7))[0]
    w += rotm(P2) + rotm(P3) + rotm(P4) + vec3(P5) + vec3(P6) + vec3(P8)
    h.case(CONST + w, lambda e, p7=p7: e.call(0x4f2670, [P2, P3, P4, P5, P6, p7, P8], 'iiiiifi', this=CAR),
           f'chassis::fn_004f2670(&mut m, {T}, {P2:#x}, {P3:#x}, {P4:#x}, {P5:#x}, {P6:#x}, {sp7!r}f32, {P8:#x})', [(CAR, 0x3a00)])
h.save()

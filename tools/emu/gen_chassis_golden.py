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

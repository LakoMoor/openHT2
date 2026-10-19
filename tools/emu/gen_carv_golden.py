"""Golden tests for the small helpers of _Car_V (carv.rs)."""
import os
import struct
import sys

sys.path.insert(0, os.path.dirname(__file__))
from harness import Harness, pack  # noqa: E402
from autofuzz import state, CAR  # noqa: E402

h = Harness('golden_carv', ['use ht2_sim::carv;'])
r = h.rnd
T = f'{CAR:#x}'
WATCH = [(CAR, 0x4000)]
UP = [(0x702630 + 8, pack('f', 1.0))]


def num(x):
    return repr(float(struct.unpack('<f', struct.pack('<f', x))[0])) + 'f32'


# 4e28b0: vector at VEC, limit
VEC = CAR + 0x3800
for _ in range(60):
    v = [r.uniform(-3, 3) for _ in range(3)]
    lim = r.uniform(0, 3)
    w = [(VEC + 4 * i, pack('f', v[i])) for i in range(3)]
    h.case(w, lambda e, lim=lim: e.call(0x4e28b0, [VEC, lim], 'if'), f'carv::fn_004e28b0(&m, {VEC:#x}, {num(lim)})', WATCH, ret=True, ret_int=True)
# 4f0000 (needs the up vector constant), 4f0090
for _ in range(60):
    w, _f = state(0x4f0000, r, fill=[(0x10, 0x40)])
    h.case(UP + w, lambda e: e.call(0x4f0000, this=CAR), f'carv::fn_004f0000(&m, {T})', WATCH, ret=True, ret_int=True)
for _ in range(80):
    w, _f = state(0x4f0090, r, overrides={0x3cc8: lambda r: ('i', r.randint(-6, 6)), 0x3ccc: lambda r: ('i', r.randint(1, 6))})
    h.case(w, lambda e: e.call(0x4f0090, this=CAR), f'carv::fn_004f0090(&m, {T})', WATCH, ret=True)
# 4f0450: copies of previous-frame state
FILL = [(0x10, 0x40), (0x40, 0x58), (0x64, 0x64 + 0x30), (0xf4, 0xf8), (0x900, 0x960), (0xdd8, 0xdd8 + 0x20), (0xf8, 0xf8 + 4 * 0xf8)]
for _ in range(30):
    w, _f = state(0x4f0450, r, fill=FILL)
    h.case(w, lambda e: e.call(0x4f0450, this=CAR), f'carv::fn_004f0450(&mut m, {T})', WATCH)
# 4f2490: wheel flags
for _ in range(80):
    w, _f = state(0x4f2490, r, overrides={0xdd4: lambda r: ('i', r.randint(0, 6))})
    for i in range(8):
        w.append((CAR + 0x19c4 + i, pack('b', r.choice([0, 1]))))
    h.case(w, lambda e: e.call(0x4f2490, this=CAR), f'carv::fn_004f2490(&mut m, {T})', WATCH)
# motion.rs: 4f0800, 4f0530, 4f0ce0 (need the constant vectors: Y = (0,1,0))
Y = [(0x702620 + 4, pack('f', 1.0))]
h2 = Harness('golden_motion', ['use ht2_sim::motion;'])
r2 = h2.rnd
for name, addr in (('fn_004f0800', 0x4f0800), ('fn_004f0530', 0x4f0530), ('fn_004f0ce0', 0x4f0ce0)):
    for _ in range(80):
        w, _f = state(addr, r2, fill=[(0x10, 0x40), (0x40, 0x58), (0xd0, 0xdc), (0x900, 0x930), (0x1f48, 0x1f54), (0xa38, 0xa44)],
                      overrides={0xdd4: lambda r: ('i', r.randint(0, 5))})
        # per-wheel state used by 4f0ce0: the short at +0x110 + 0xf8*i
        for i in range(6):
            w.append((CAR + 0x110 + 0xf8 * i, pack('h', r2.choice([0, 1, 2]))))
        if r2.random() < 0.3:
            w.append((CAR + 0xdc0, pack('f', 0.0)))
        h2.case(Y + w, lambda e, addr=addr: e.call(addr, this=CAR), f'motion::{name}(&mut m, {T})', WATCH)
h2.save()
h.save()

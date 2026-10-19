"""Golden tests for the rigid-body step of _Car_V (rigid.rs)."""
import math
import os
import struct
import sys

sys.path.insert(0, os.path.dirname(__file__))
from harness import Harness, pack  # noqa: E402
from autofuzz import state, CAR  # noqa: E402

h = Harness('golden_rigid', ['use ht2_sim::rigid;'])
r = h.rnd
T = f'{CAR:#x}'
WATCH = [(CAR, 0x4000)]
AXES = [(0x7025f0, struct.pack('<12f', 1, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0)), (0x702620, struct.pack('<3f', 0, 1, 0)),
        (0x702630, struct.pack('<3f', 0, 0, 1)), (0x702640, struct.pack('<3f', 1, 0, 0)), (0x702650, bytes(12))]

for _ in range(60):
    w, _f = state(0x4f0160, r, overrides={0x1a20: lambda r: ('f', r.uniform(-0.6, 0.6)), 0x1f60: lambda r: ('f', r.uniform(2, 6)),
                                          0x1f78: lambda r: ('f', r.uniform(-1, 5))})
    h.case(w, lambda e: e.call(0x4f0160, this=CAR), f'rigid::fn_004f0160(&mut m, {T})', WATCH)
for addr, name in ((0x4f0410, 'fn_004f0410'), (0x4f0430, 'fn_004f0430')):
    for _ in range(20):
        w, _f = state(addr, r)
        h.case(w, lambda e, addr=addr: e.call(addr, this=CAR), f'rigid::{name}(&m, {T})', WATCH, ret=True)
for _ in range(100):
    w, _f = state(0x4f01b0, r, fill=[(0x900, 0x930), (0xa38, 0xa44), (0xaa4, 0xab0), (0xabc, 0xac8), (0x18a4, 0x18b0), (0x18bc, 0x18c8), (0xd0, 0xdc)],
                  overrides={0x1a20: lambda r: ('f', r.uniform(-0.6, 0.6)), 0x1a24: lambda r: ('f', r.choice([r.uniform(-0.6, 0.6), r.uniform(1.5, 1.6)])),
                             0x1f60: lambda r: ('f', r.uniform(2, 6))})
    h.case(AXES + w, lambda e: e.call(0x4f01b0, this=CAR), f'rigid::fn_004f01b0(&mut m, {T})', WATCH)
for _ in range(40):
    w, _f = state(0x4ef620, r, fill=[(0x9f0, 0xa44), (0xd40, 0xd4c)])
    h.case(AXES + w, lambda e: e.call(0x4ef620, this=CAR), f'rigid::fn_004ef620(&mut m, {T})', WATCH)
for _ in range(100):
    w, _f = state(0x4ef7c0, r, fill=[(0x10, 0x60), (0xd0, 0xdc), (0x1f48, 0x1f54)],
                  overrides={0x2638: lambda r: ('i', r.choice([0, 1, 2, 3]))})
    if r.random() < 0.2:
        w += [(CAR + 0x40, bytes(12))]
    h.case(AXES + w, lambda e: e.call(0x4ef7c0, this=CAR), f'rigid::fn_004ef7c0(&mut m, {T})', WATCH)
h.save()

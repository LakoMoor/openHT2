"""Golden tests for the TechSystem class and the clock (0x4eb460..0x4eb720, 0x4e2749, 0x4e27a5)."""
import os
import struct
import sys

sys.path.insert(0, os.path.dirname(__file__))
from harness import Harness, pack, OBJ_BASE, CLOCK_OBJ  # noqa: E402

h = Harness('golden_tech_system', ['use ht2_sim::{tech_system as ts, globals};'])
r = h.rnd
THIS = OBJ_BASE + 0x200
WATCH = [(THIS, 0x34)]


def random_object():
    w = []
    for i in range(3):
        a = r.uniform(-5, 20)
        b = a + r.choice([-1, 1]) * r.uniform(0, 30) if r.random() < 0.9 else a
        w.append((THIS + i * 4, pack('f', a)))
        w.append((THIS + 0xc + i * 4, pack('f', b)))
        w.append((THIS + 0x18 + i * 4, pack('f', r.choice([0.0, r.uniform(0.01, 5)]))))
    w.append((THIS + 0x24, pack('f', r.uniform(-0.2, 1.3))))
    w.append((THIS + 0x28, pack('i', r.randint(0, 2))))
    w.append((THIS + 0x2c, pack('i', r.randint(0, 3))))
    w.append((THIS + 0x30, pack('i', r.randint(0, 2))))
    return w


def num(x):
    return repr(float(struct.unpack('<f', struct.pack('<f', x))[0])) + 'f32'


for _ in range(25):
    h.case(random_object(), lambda e: e.call(0x4eb460, this=THIS), 'ts::reset(&mut m, 0x%x)' % THIS, WATCH)
for _ in range(40):
    idx = r.randint(-1, 3)
    lo, hi, rate = r.uniform(-3, 10), r.uniform(-3, 10), r.choice([0.0, r.uniform(0.1, 9)])
    h.case(random_object(), lambda e, idx=idx, lo=lo, hi=hi, rate=rate: e.call(0x4eb580, [idx, lo, hi, rate], 'ifff', this=THIS),
           f'ts::define(&mut m, 0x{THIS:x}, {idx}, {num(lo)}, {num(hi)}, {num(rate)})', WATCH)
for _ in range(60):
    level = r.randint(0, 2)
    h.case(random_object(), lambda e, level=level: e.call(0x4eb4a0, [level], 'i', this=THIS),
           f'ts::set_upgrade(&mut m, 0x{THIS:x}, {level})', WATCH)
for _ in range(30):
    h.case(random_object(), lambda e: e.call(0x4eb5e0, this=THIS), f'ts::value(&m, 0x{THIS:x})', WATCH, ret=True)
for _ in range(40):
    v = r.uniform(-8, 25)
    h.case(random_object(), lambda e, v=v: e.call(0x4eb600, [v], 'f', this=THIS), f'ts::set_value(&mut m, 0x{THIS:x}, {num(v)})', WATCH)
for _ in range(10):
    h.case(random_object(), lambda e: e.call(0x4eb680, this=THIS), f'ts::full_value(&m, 0x{THIS:x})', WATCH, ret=True)
    h.case(random_object(), lambda e: e.call(0x4eb690, this=THIS), f'ts::fraction(&m, 0x{THIS:x})', WATCH, ret=True)
for _ in range(20):
    v = r.uniform(-1, 2)
    h.case(random_object(), lambda e, v=v: e.call(0x4eb6a0, [v], 'f', this=THIS), f'ts::set_fraction(&mut m, 0x{THIS:x}, {num(v)})', WATCH)
for _ in range(30):
    v = r.uniform(-1, 3)
    h.case(random_object(), lambda e, v=v: e.call(0x4eb6e0, [v], 'f', this=THIS), f'ts::consume(&mut m, 0x{THIS:x}, {num(v)})', WATCH)
    h.case(random_object(), lambda e, v=v: e.call(0x4eb720, [v], 'f', this=THIS), f'ts::set_from_use(&mut m, 0x{THIS:x}, {num(v)})', WATCH)
# clock: normal and fixed-step mode
for _ in range(20):
    t, dt = r.uniform(0, 500), r.choice([0.0, 0.016, 0.03, 0.2, 0.3, 1.5])
    h.case([], lambda e: e.call(0x4e27a5), 'globals::time(&m)', [(CLOCK_OBJ, 0x10)], ret=True, time=t, dt=dt)
    h.case([], lambda e: e.call(0x4e2749), 'globals::dt(&m)', [(CLOCK_OBJ, 0x10)], ret=True, time=t, dt=dt)
for _ in range(10):
    step = r.choice([1, 16, 33, 100, 250, 1000])
    t = r.uniform(0, 100)
    w = [(0x6974c2, pack('h', step)), (0x6974bc, pack('f', t))]
    h.case(w, lambda e: e.call(0x4e27a5), 'globals::time(&m)', [(0x6974bc, 4)], ret=True)
    h.case(w, lambda e: e.call(0x4e2749), 'globals::dt(&m)', [(0x6974bc, 4)], ret=True)
h.save()

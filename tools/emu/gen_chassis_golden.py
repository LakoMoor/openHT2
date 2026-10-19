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

# simple snapshot helpers
h = Harness('golden_snapshots', ['use ht2_sim::chassis;'])
r = h.rnd
CLK = CAR + 0x3f00
for name, addr in (('fn_004f0720', 0x4f0720), ('fn_004f07e0', 0x4f07e0), ('fn_004f09f0', 0x4f09f0)):
    for _ in range(60):
        w, _f = state(addr, r, fill=[(0x10, 0x60), (0x900, 0x930), (0xd58, 0xd64), (0x9f0, 0xa44), (0xdc0, 0xdcc), (0x2670, 0x2690), (0x29d8, 0x29e0), (0x64, 0x64 + 72), (0x1cbc, 0x1cbc + 72)],
                      overrides={0xdd4: lambda r: ('i', r.randint(0, 6)), 0x8cc: lambda r: ('i', 3)})
        w.append((0x697868 + 3 * 0x970 + 0x810, pack('f', r.uniform(-2, 3))))
        w += [(CLK + 0x18 + 8 * k, pack('d', r.uniform(-50, 50))) for k in range(3)]
        for i in range(6):
            w.append((CAR + 0x3cdc + 20 * i, pack('i', r.choice([0, 1, 5]))))
        call = f'snap::{name}(&mut m, {T}, {CLK:#x})' if name == 'fn_004f09f0' else f'chassis::{name}(&mut m, {T})'
        call = call.replace('snap::', 'chassis::')
        args = [CLK] if name == 'fn_004f09f0' else []
        h.case(CONST + w, lambda e, addr=addr, args=args: e.call(addr, args, 'i' * len(args), this=CAR), call, [(CAR, 0x4000)])
h.save()

# 0x4f3000
h = Harness('golden_tyres', ['use ht2_sim::chassis;'])
r = h.rnd
for _ in range(300):
    w, _f = state(0x4f3000, r, fill=[(0x1c, 0x28), (0xa38, 0xa44), (0xa44, 0xa44 + 72), (0xaa4, 0xaa4 + 72), (0x1904, 0x1904 + 72), (0xd20, 0xd20 + 32), (0x2670, 0x2670 + 32), (0x1964, 0x1964 + 32), (0x1a7c, 0x1a7c + 32), (0x1c94, 0x1c94 + 4), (0x1c98, 0x1c98 + 32)],
                  overrides={0xdd4: lambda r: ('i', r.randint(0, 6)), 0x29dc: lambda r: ('f', r.choice([0.0, 0.05, 0.5, 1.0]))})
    for i in range(6):
        w.append((CAR + 0x19a4 + 2 * i, pack('h', r.choice([0, 0, 0, 1]))))
        w.append((CAR + 0x1bb4 + 4 * i, pack('i', r.choice([0, 1, 1]))))
        w.append((CAR + 0x2670 + 4 * i, pack('f', r.uniform(0.2, 1.2))))
        w.append((CAR + 0xd20 + 4 * i, pack('f', r.uniform(500, 30000))))
        w.append((CAR + 0x1a7c + 4 * i, pack('f', r.uniform(-1, 1))))
        w.append((CAR + 0x1964 + 4 * i, pack('f', r.uniform(0, 2))))
    w.append((CAR + 0x1c94, pack('f', r.uniform(-60000, 60000))))
    w.append((CAR + 0xdc8, pack('f', r.uniform(-3, 3))))
    h.case(CONST + w, lambda e: e.call(0x4f3000, this=CAR), f'chassis::fn_004f3000(&mut m, {T})', [(CAR, 0x4000)], ret=True)
h.save()

# 0x4f35f0
h = Harness('golden_wheel_spin', ['use ht2_sim::chassis;'])
r = h.rnd
for _ in range(200):
    w, _f = state(0x4f35f0, r, fill=[(0xa38, 0xa44), (0xaa4, 0xaa4 + 72), (0x18a4, 0x18a4 + 72), (0x1904, 0x1904 + 72), (0xa44, 0xa44 + 72), (0x1b74, 0x1b74 + 32), (0xe9c, 0xe9c + 32), (0xebc, 0xebc + 32), (0x1c44, 0x1c44 + 64), (0x1c88, 0x1c8c), (0x1c84, 0x1c88), (0x1c, 0x28), (0x2640, 0x2644)],
                  overrides={0xdd4: lambda r: ('i', r.randint(0, 6)), 0x29dc: lambda r: ('f', r.choice([0.0, 0.05, 0.5, 1.0])), 0x1c1c: lambda r: ('i', r.randint(0, 8))})
    for i in range(6):
        w.append((CAR + 0x19a4 + 2 * i, pack('h', r.choice([0, 0, 1]))))
        w.append((CAR + 0x1bb4 + 4 * i, pack('i', r.choice([0, 1, 1]))))
        w.append((CAR + 0x2670 + 4 * i, pack('f', r.uniform(0.2, 1.2))))
        w.append((CAR + 0xd20 + 4 * i, pack('f', r.uniform(500, 30000))))
        w.append((CAR + 0x1a7c + 4 * i, pack('f', r.uniform(-1, 1))))
        w.append((CAR + 0x1964 + 4 * i, pack('f', r.uniform(0, 2))))
        w.append((CAR + 0x1b74 + 4 * i, pack('f', r.uniform(0.3, 0.6))))
        w.append((CAR + 0xebc + 4 * i, pack('f', r.uniform(-60, 60))))
        w.append((CAR + 0xe9c + 4 * i, pack('f', r.uniform(-9, 9))))
    for base in (0x3f58, 0x3f8c):
        for k in range(6):
            w.append((CAR + base + 4 * k, pack('f', r.uniform(0, 5))))
        w.append((CAR + base + 0x18, pack('f', r.uniform(0.5, 3))))
        w.append((CAR + base + 0x24, pack('f', r.uniform(0, 1))))
        w.append((CAR + base + 0x28, pack('i', 0)))
        w.append((CAR + base + 0x2c, pack('i', 1)))
    w.append((CAR + 0x1c94, pack('f', r.uniform(-60000, 60000))))
    w.append((CAR + 0xdc8, pack('f', r.uniform(-30, 30))))
    h.case(CONST + w, lambda e: e.call(0x4f35f0, this=CAR), f'chassis::fn_004f35f0(&mut m, {T})', [(CAR, 0x4000)], dt=r.uniform(0.005, 0.05))
h.save()

# 0x4f6b70, 0x4eba10
h = Harness('golden_recorder', ['use ht2_sim::chassis;'])
r = h.rnd
for _ in range(60):
    w, _f = state(0x4f6b70, r, fill=[(0xf8, 0xf8 + 0xf8 * 7), (0x10, 0x60), (0x900, 0x930), (0xd58, 0xd64), (0x9f0, 0xa44), (0xdc0, 0xdcc), (0x2670, 0x2690), (0x29d8, 0x29e0), (0x64, 0x64 + 72), (0x1cbc, 0x1cbc + 72), (0xe9c, 0xe9c + 32), (0x1a30, 0x1a30 + 32), (0xdd8, 0xdd8 + 32)],
                  overrides={0xdd4: lambda r: ('i', r.randint(0, 6)), 0x8cc: lambda r: ('i', 3)})
    w.append((0x697868 + 3 * 0x970 + 0x810, pack('f', r.uniform(-2, 3))))
    w += [(CLK + 0x18 + 8 * k, pack('d', r.uniform(-50, 50))) for k in range(3)]
    for i in range(6):
        w.append((CAR + 0x3cdc + 20 * i, pack('i', r.choice([0, 1, 5]))))
        w.append((CAR + 0x19a4 + 2 * i, pack('h', r.choice([0, 1, -1]))))
        w.append((CAR + 0x19c4 + i, pack('b', r.choice([0, 1, 255]))))
    h.case(CONST + w, lambda e: e.call(0x4f6b70, [CLK], 'i', this=CAR), f'chassis::fn_004f6b70(&mut m, {T}, {CLK:#x})', [(CAR, 0x4000)])
for _ in range(20):
    w, _f = state(0x4eba10, r, fill=[(0x2700, 0x2800)])
    h.case(CONST + w, lambda e: e.call(0x4eba10, this=CAR + 0x2700), f'chassis::fn_004eba10(&mut m, {CAR + 0x2700:#x})', [(CAR, 0x4000)], time=r.uniform(1, 100))
h.save()

# 0x4fca10
import math as _m
h = Harness('golden_body', ['use ht2_sim::chassis;'])
r = h.rnd


def rot_matrix(r):
    ax, ay, az = r.uniform(-0.4, 0.4), r.uniform(-0.4, 0.4), r.uniform(-3, 3)
    cz, sz, cx, sx, cy, sy = _m.cos(az), _m.sin(az), _m.cos(ax), _m.sin(ax), _m.cos(ay), _m.sin(ay)
    Rz = [[cz, sz, 0], [-sz, cz, 0], [0, 0, 1]]
    Rx = [[1, 0, 0], [0, cx, sx], [0, -sx, cx]]
    Ry = [[cy, 0, -sy], [0, 1, 0], [sy, 0, cy]]
    mm = lambda a, b: [[sum(a[i][k] * b[k][j] for k in range(3)) for j in range(3)] for i in range(3)]
    return mm(mm(Ry, Rx), Rz)


OUT = CAR + 0x3a00
for k in range(300):
    w, _f = state(0x4fca10, r, fill=[(0xc6c, 0xc84), (0xd40, 0xd4c), (0x1cbc, 0x1cbc + 48), (0x1f7c, 0x1f8c), (0xa08, 0xa14)],
                  overrides={0xdd4: lambda r: ('i', 4), 0x2640: lambda r: ('f', r.uniform(2000, 12000)), 0xdc0: lambda r: ('f', r.uniform(0, 30))})
    M = rot_matrix(r)
    pos = [r.uniform(-100, 100) for _ in range(3)]
    mat = [M[i][j] for i in range(3) for j in range(3)] + pos
    inv = [M[j][i] for i in range(3) for j in range(3)]
    tinv = [-sum(pos[k] * M[j][k] for k in range(3)) for j in range(3)]
    w += [(CAR + 0x10 + 4 * i, pack('f', v)) for i, v in enumerate(mat)]
    w += [(CAR + 0x900 + 4 * i, pack('f', v)) for i, v in enumerate(inv + tinv)]
    nz = [M[2][j] for j in range(3)]
    w += [(CAR + 0xd0 + 4 * i, pack('f', nz[i] + r.uniform(-0.05, 0.05))) for i in range(3)]
    ry = [M[1][j] for j in range(3)]
    w += [(CAR + 0xa38 + 4 * i, pack('f', ry[i])) for i in range(3)]
    w += [(CAR + 0xa2c + 4 * i, pack('f', ry[i] + r.uniform(-0.1, 0.1))) for i in range(3)]
    w += [(CAR + 0x2214 + 4 * i, pack('f', pos[i] + r.uniform(-1, 1))) for i in range(3)]
    w.append((CAR + 0x2220, pack('f', r.uniform(-1, 1))))
    w.append((CAR + 0x2640, pack('f', r.uniform(2000, 12000))))
    w.append((CAR + 0xdc0, pack('f', r.uniform(0, 30))))
    w.append((CAR + 0xdd4, pack('i', 4)))
    for o in list(range(0xc6c, 0xc84, 4)) + list(range(0xd40, 0xd4c, 4)) + list(range(0xa08, 0xa14, 4)):
        w.append((CAR + o, pack('f', r.uniform(-2000, 2000) if o < 0xc84 else r.uniform(-5, 5))))
    for wh in range(4):
        wp = [r.uniform(-2, 2), r.uniform(-3, 3), r.uniform(-1, 0)]
        w += [(CAR + 0x104 + 0xf8 * wh + 4 * i, pack('f', wp[i])) for i in range(3)]
        w.append((CAR + 0xe18 + 4 * wh, pack('f', r.uniform(-0.3, 1.5))))
        w.append((CAR + 0x1b74 + 4 * wh, pack('f', r.uniform(0.3, 0.6))))
        w.append((CAR + 0x19a4 + 2 * wh, pack('h', r.choice([0, 0, 0, 1]))))
    w.append((CAR + 0x19b4, pack('b', 1 if r.random() < 0.05 else 0)))
    w.append((CAR + 0x19ba, pack('b', r.choice([0, 1, 1, 1]))))
    w.append((CAR + 0xe38, pack('f', r.uniform(0, 4))))
    for o in (0x19b5, 0x19b6, 0x19b7, 0x19b8):
        w.append((CAR + o, pack('b', 1 if r.random() < 0.2 else 0)))
    h.case(CONST + w, lambda e: e.call(0x4fca10, [OUT], 'i', this=CAR), f'chassis::fn_004fca10(&mut m, {T}, {OUT:#x})', [(CAR, 0x4000)], dt=r.uniform(0.005, 0.05))
h.save()

# 0x501930
h = Harness('golden_damage', ['use ht2_sim::chassis;'])
r = h.rnd
SIN = [(0x696ec0 + 4 * i, pack('f', _m.sin((i * 3.1415926 + i * 3.1415926) / 360.0))) for i in range(360)]
for _ in range(200):
    w, _f = state(0x501930, r, fill=[(0x1d60, 0x1da0), (0xe9c, 0xebc), (0x1e84, 0x1ea4), (0x1e1c, 0x1e28)])
    w.append((CAR + 0xdd4, pack('i', r.randint(0, 6))))
    for i in range(6):
        w.append((CAR + 0x3cdc + 20 * i, pack('i', r.choice([0, 1, 2]))))
        w.append((CAR + 0x3ce0 + 20 * i, pack('i', r.choice([0, 1, 2]))))
        w.append((CAR + 0x3e1c + 20 * i, pack('i', r.choice([0, 1]))))
        w.append((CAR + 0xe9c + 4 * i, pack('f', r.uniform(-3.2, 3.2))))
    w.append((CAR + 0x3cb4, pack('i', r.choice([0, 1, 2]))))
    w.append((CAR + 0x1e04, pack('i', r.randint(0, 8))))
    base = CAR + 0x3f58
    for k in range(6):
        w.append((base + 4 * k, pack('f', r.uniform(0, 5))))
    w += [(base + 0x24, pack('f', r.uniform(0, 1))), (base + 0x28, pack('i', 0)), (base + 0x2c, pack('i', 1))]
    h.case(CONST + SIN + w, lambda e: e.call(0x501930, this=CAR), f'chassis::fn_00501930(&mut m, {T})', [(CAR, 0x4000)])
h.save()

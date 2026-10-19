"""Golden tests for the wheel/world contact code (contact.rs)."""
import os
import struct
import sys

sys.path.insert(0, os.path.dirname(__file__))
from harness import Harness, pack  # noqa: E402
from autofuzz import state, CAR  # noqa: E402
import world_stub  # noqa: E402

T = f'{CAR:#x}'
CONST = [(0x702620 + 4, pack('f', 1.0)), (0x702630 + 8, pack('f', 1.0)), (0x702640, pack('f', 1.0))]
CONST += [(0x7025f0 + 4 * i, pack('f', float(v))) for i, v in enumerate([1, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0])]
OBJ, VTBL, SURF, ROOM = CAR + 0x5000, CAR + 0x5100, CAR + 0x5200, CAR + 0x5300
PT, OUTP = CAR + 0x5400, CAR + 0x5410

h = Harness('golden_contact', ['use ht2_sim::contact;'])
world_stub.install(h.e)
r = h.rnd
for _ in range(150):
    w = []
    for i in range(3):
        w.append((CAR + 0x8f4 + 4 * i, pack('f', r.uniform(-5, 5) if i < 2 else r.uniform(0, 3))))
        w.append((PT + 4 * i, pack('f', r.uniform(-5, 5) if i < 2 else r.uniform(-3, 3))))
    has = r.random() < 0.9
    w.append((CAR + 0x1a10, pack('u', OBJ if has else 0)))
    w.append((CAR + 0x3c54, pack('i', 0)))
    w += world_stub.make_obj(OBJ, VTBL, r.uniform(-1, 2), SURF, ROOM, r.choice([0, 0, 2]))
    h.case(CONST + w, lambda e: e.call(0x5039a0, [PT, OUTP], 'ii', this=CAR),
           f'contact::fn_005039a0(&mut m, &mut contact::StubWorld, {T}, {PT:#x}, {OUTP:#x})', [(CAR, 0x6000)], ret=True, ret_int=True)
REC = CAR + 0x5600
for _ in range(150):
    w, _f = state(0x503ac0, r, fill=[(0xf8, 0xf8 + 0xf8 * 7)])
    w.append((CAR + 0xdd4, pack('i', r.randint(0, 6))))
    for i in range(6):
        wb = CAR + 0xf8 * (i + 1)
        w.append((CAR + 0x1bf4 + 4 * i, pack('i', r.choice([0, 1]))))
        w.append((wb + 0x18, pack('h', r.choice([0, 1]))))
        w.append((wb + 0x84, pack('u', r.choice([0, SURF]))))
        w += [(wb + 0xc + 4 * k, pack('f', v)) for k, v in enumerate([r.uniform(-0.2, 0.2), r.uniform(-0.2, 0.2), 1.0])]
    for k in range(52):
        w.append((REC + 4 * k, pack('f', r.uniform(-5, 5))))
    h.case(CONST + w, lambda e: e.call(0x503ac0, [REC], 'i', this=CAR),
           f'contact::fn_00503ac0(&mut m, {T}, {REC:#x})', [(CAR, 0x6000)], ret=True, ret_int=True)
h.save()

# --- surface model -----------------------------------------------------------
e0 = Harness('golden_surface', ['use ht2_sim::contact;'])
world_stub.install(e0.e)
e0.e.mu.mem_map(0x7f0000, 0x1000)  # where the port keeps the rand() state
e0.e.call(0x501fb0, this=0x6976f8)
e0.e.call(0x5021c0, this=0x697688)
e0.e.call(0x503dd0)
TABLES = [(0x697688, bytes(e0.e.read(0x697688, 0x38 * 2 + 56 * 4))), (0x697678, bytes(e0.e.read(0x697678, 16)))]
SURFC = CONST + TABLES + [(0x696bdc, pack('f', 1.4)), (0x7f0004, pack('i', 12345))]
KICK = CAR + 0x5800
r = e0.rnd
for k in range(400):
    w, _f = state(0x502780, r, fill=[(0x2900, 0x2900 + 0x30 * 3)])
    wheel = r.randint(0, 5)
    f4 = 4 * wheel
    surfkind = r.randint(1, 13)
    cnt = r.choice([0, 1, 2, 4])
    w += [(SURF, pack('i', r.choice([1, 1, 1, 0]))), (SURF + 4, pack('i', surfkind)), (SURF + 8, pack('i', cnt)), (SURF + 12, pack('u', SURF + 0x20))]
    w += [(SURF + 0x20 + 4 * i, pack('f', r.uniform(0.1, 1.5))) for i in range(6)]
    w += [(REC + 4 * i, pack('f', r.uniform(-3, 3))) for i in range(52)]
    w.append((REC + 0x64, pack('u', SURF if r.random() < 0.95 else 0)))
    w.append((0x69784c, pack('i', r.choice([0, 1]))))
    w.append((0x6ef1fc, pack('u', KICK)))
    w += [(KICK + 0xb58 + 8 * i, pack('d', r.uniform(-5, 5))) for i in range(3)]
    w += [(CAR + 0x34 + 4 * i, pack('f', r.uniform(-5, 5))) for i in range(3)]
    w += [(CAR + o + f4, pack('f', v)) for o, v in ((0xe3c, r.uniform(-0.05, 0.05)), (0xe5c, r.uniform(-2, 2)), (0x1964, r.uniform(0, 15)),
                                                     (0x1a7c, r.uniform(-1, 1)), (0x1a5c, r.uniform(-1, 1)), (0x1a30, r.uniform(-0.5, 0.5)), (0x1f9c, r.uniform(0.1, 0.5)))]
    w += [(CAR + 0x19a4 + 2 * wheel, pack('h', r.choice([0, 0, 1]))), (CAR + 0x19c4 + wheel, pack('b', r.choice([0, 1]))),
          (CAR + 0x19d4 + f4, pack('i', r.choice([0, 1]))), (CAR + 0x2740, pack('h', r.choice([0, 1]))), (CAR + 0x276c, pack('i', r.choice([0, 1, 2]))),
          (CAR + 0x3f4c, pack('i', r.choice([0, 1]))), (CAR + 0x2640, pack('f', r.uniform(2000, 12000))),
          (CAR + 0x1030 + 0xf8 * wheel, pack('h', r.choice([0, 1]))), (CAR + 0x29d0, pack('i', r.randint(0, 3))), (CAR + 0x2700 + 0xc0, pack('i', r.randint(0, 5)))]
    for zi in range(3):
        cx, cy = r.uniform(-4, 4), r.uniform(-4, 4)
        pts = [(cx, cy), (cx + 3, cy), (cx + 3, cy + 3), (cx, cy + 3)]
        w += [(CAR + 0x2900 + 0x30 * zi + 12 * j + 4 * c, pack('f', v)) for j, p in enumerate(pts) for c, v in enumerate((p[0], p[1], 0.0))]
    w += [(CAR + 0x1018 + 0xf8 * wheel + 4 * i, pack('f', r.uniform(-5, 5))) for i in range(3)]
    for base in (0x3f24,):
        for kk in range(6):
            w.append((CAR + base + 4 * kk, pack('f', r.choice([1.0, 2.0, r.uniform(0, 3)]))))
        w += [(CAR + base + 0x24, pack('f', r.choice([0.0, 1.0, r.uniform(0, 1)]))), (CAR + base + 0x28, pack('i', 0)), (CAR + base + 0x2c, pack('i', 1))]
    e0.case(SURFC + w, lambda e, wheel=wheel: e.call(0x502780, [REC, wheel], 'ii', this=CAR),
            f'contact::fn_00502780(&mut m, {T}, {REC:#x}, {wheel})', [(CAR, 0x6000)], ret=True, ret_int=True, dt=r.uniform(0.005, 0.05))
for k in range(300):
    w, _f = state(0x503500, r)
    wheel = r.randint(0, 5)
    f4 = 4 * wheel
    w += [(SURF, pack('i', 1)), (SURF + 4, pack('i', r.randint(1, 13))), (SURF + 8, pack('i', r.choice([0, 2, 3]))), (SURF + 12, pack('u', SURF + 0x20))]
    w += [(SURF + 0x20 + 4 * i, pack('f', r.uniform(0.1, 1.5))) for i in range(6)]
    w += [(REC + 4 * i, pack('f', r.uniform(-3, 3))) for i in range(52)]
    w.append((REC + 0x64, pack('u', SURF if r.random() < 0.95 else 0)))
    w += [(CAR + o + f4, pack('f', v)) for o, v in ((0x1964, r.uniform(0, 15)), (0x1984, r.uniform(0, 30)), (0x2690, r.uniform(0, 1)), (0x2670, r.uniform(0, 1)),
                                                     (0xd20, r.uniform(500, 30000)), (0x1b74, r.choice([0.0, 0.4])))]
    w += [(CAR + 0x1904 + 12 * wheel + 4 * i, pack('f', r.uniform(-1, 1))) for i in range(3)]
    w += [(CAR + 0x2640, pack('f', r.uniform(2000, 12000))), (CAR + 0x34b4, pack('f', r.uniform(0, 2))), (CAR + 0x19a4 + 2 * wheel, pack('h', r.choice([0, 0, 1]))),
          (CAR + 0x3cdc + 20 * wheel, pack('i', r.choice([0, 0, 1, 2]))), (CAR + 0x3ce0 + 20 * wheel, pack('i', r.choice([1, 2, 3]))),
          (CAR + 0x3ce8 + 20 * wheel, pack('f', r.uniform(0, 3))), (CAR + 0x26c0 + f4, pack('i', r.choice([0, 1])))]
    kind = r.choice([0, 0, 0, r.randint(1, 13)])
    e0.case(SURFC + w, lambda e, wheel=wheel, kind=kind: e.call(0x503500, [REC, wheel, kind], 'iii', this=CAR),
            f'contact::fn_00503500(&mut m, {T}, {REC:#x}, {wheel}, {kind})', [(CAR, 0x6000)], dt=r.uniform(0.005, 0.05), time=r.uniform(1, 50))
e0.save()

# --- zones and the wheel query -------------------------------------------------
e1 = Harness('golden_wheel_query', ['use ht2_sim::contact;'])
world_stub.install(e1.e)
e1.e.mu.mem_map(0x7f0000, 0x1000)
e1.e.hook_ret(0x6037e0, lambda e: 0)   # performance timer (statistics only)
e1.e.call(0x501fb0, this=0x6976f8)
e1.e.call(0x5021c0, this=0x697688)
e1.e.call(0x503dd0)
r = e1.rnd
ZONE, P2, P3 = CAR + 0x5900, CAR + 0x5940, CAR + 0x5950
for k in range(200):
    cx, cy = r.uniform(-2, 2), r.uniform(-2, 2)
    sx, sy = r.uniform(1, 3), r.uniform(1, 3)
    pts = [(cx, cy, r.uniform(-0.2, 0.2)), (cx + sx, cy, 0.0), (cx + sx, cy + sy, 0.1), (cx, cy + sy, 0.0)]
    if r.random() < 0.5:
        pts = pts[::-1]
    w = [(ZONE + 12 * j + 4 * c, pack('f', v)) for j, p in enumerate(pts) for c, v in enumerate(p)]
    w += [(P2 + 4 * i, pack('f', r.uniform(-5, 5))) for i in range(3)] + [(P3 + 4 * i, pack('f', r.uniform(-5, 5))) for i in range(3)]
    e1.case(CONST + w, lambda e: e.call(0x502270, [ZONE, P2, P3], 'iii'), f'contact::fn_00502270(&mut m, {ZONE:#x}, {P2:#x}, {P3:#x})',
            [(CAR, 0x6000)], ret=True, ret_int=True)
CENTRE = CAR + 0x5a00
import math as _m
for k in range(300):
    w, _f = state(0x503e30, r, fill=[(0x10, 0x40), (0xf8, 0xf8 + 0xf8 * 7)])
    wheel = r.randint(0, 3)
    f4 = 4 * wheel
    t = r.uniform(-0.3, 0.3)
    M = [_m.cos(t), _m.sin(t), 0, -_m.sin(t), _m.cos(t), 0, 0, 0, 1, r.uniform(-3, 3), r.uniform(-3, 3), r.uniform(0, 2)]
    w += [(CAR + 0x10 + 4 * i, pack('f', v)) for i, v in enumerate(M)]
    w += [(CENTRE + 4 * i, pack('f', v)) for i, v in enumerate((r.uniform(-3, 3), r.uniform(-3, 3), r.uniform(-0.5, 1.5)))]
    w += world_stub.make_obj(OBJ, VTBL, r.uniform(-0.3, 0.6), SURF, OBJ, r.choice([0, 0, 2]))
    w += [(SURF, pack('i', 1)), (SURF + 4, pack('i', r.randint(1, 13))), (SURF + 8, pack('i', r.choice([0, 2]))), (SURF + 12, pack('u', SURF + 0x20))]
    w += [(SURF + 0x20 + 4 * i, pack('f', r.uniform(0.1, 1.5))) for i in range(6)]
    wb = CAR + 0xf8 * (wheel + 1)
    w += [(wb + 0x18, pack('h', r.choice([0, 1]))), (wb + 0x1a, pack('h', r.choice([0, 0, 1]))), (wb + 0x84, pack('u', r.choice([0, SURF])))]
    w += [(wb + 0xc + 4 * i, pack('f', v)) for i, v in enumerate((0.0, 0.0, 1.0))]
    w += [(CAR + 0x1bf4 + 4 * i, pack('i', r.choice([1, 1, 0]))) for i in range(6)]
    w += [(CAR + 0xf8 * (i + 1) + 0x84, pack('u', r.choice([0, SURF]))) for i in range(6) if i != wheel]
    w += [(CAR + o, pack(kd, v)) for o, kd, v in ((0x3c54, 'i', 0), (0x2708, 'h', 0), (0x1a10, 'u', OBJ), (0x8f4, 'f', 0.0), (0x8f8, 'f', 0.0), (0x8fc, 'f', 3.0),
                                                   (0xdc0, 'f', r.uniform(0, 20)), (0x1b74 + f4, 'f', r.uniform(0.3, 0.6)), (0x1f9c + f4, 'f', r.uniform(0.1, 0.4)),
                                                   (0x275c, 'i', r.choice([0, 1])), (0x8cc, 'i', 3), (0xdd4, 'i', 4), (0x2640, 'f', r.uniform(2000, 12000)))]
    w += [(0x697868 + 3 * 0x970 + 0xb8, pack('f', r.uniform(1.5, 2.5))), (0x697868 + 3 * 0x970 + 0xbc, pack('f', r.uniform(1.5, 2.5)))]
    w += [(CAR + 0x3f24 + 4 * kk, pack('f', 1.0)) for kk in range(6)] + [(CAR + 0x3f24 + 0x24, pack('f', 1.0)), (CAR + 0x3f24 + 0x28, pack('i', 0)), (CAR + 0x3f24 + 0x2c, pack('i', 1))]
    w += [(CAR + 0x29d0, pack('i', 0)), (0x6ef1fc, pack('u', KICK)), (CAR + 0x2700 + 0xc0, pack('i', 0))]
    w += [(CAR + 0x3cdc + 20 * i, pack('i', 0)) for i in range(6)] + [(CAR + 0x3ce0 + 20 * i, pack('i', 1)) for i in range(6)]
    room = r.choice([0, OBJ, OBJ])
    nosurf = r.choice([0, 0, 1])
    e1.case(SURFC + w, lambda e, wheel=wheel, room=room, nosurf=nosurf: e.call(0x503e30, [nosurf, CENTRE, room, wheel], 'iiii', this=CAR),
            f'contact::fn_00503e30(&mut m, &mut contact::StubWorld, {T}, {nosurf}, {CENTRE:#x}, {room:#x}, {wheel})', [(CAR, 0x6000)],
            ret=True, ret_int=True, dt=r.uniform(0.005, 0.05),
            ignore=[(base + o, n) for base in (wb + 0x20, CAR + 0x3510 + 0xd0 * wheel) for o, n in ((0x70, 4), (0x7c, 0x44))])
e1.save()

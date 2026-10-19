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

"""Golden test for the vehicle constructor `_Car_V::_Car_V` (0x4ebd00).

The wheel positions in `vehicle.tech` (+0x1a8) and the model matrices (0x6d2200 + vid*0x5d0) are
filled in by the vehicle loader from the models; the test makes up plausible ones."""
import math as _m
import os
import struct
import sys

sys.path.insert(0, os.path.dirname(__file__))
from harness import Harness, pack, rs_f32  # noqa: E402
from emu import HEAP_BASE  # noqa: E402
from carstate import CAR, IDS  # noqa: E402
import world_stub  # noqa: E402

T = f'{CAR:#x}'
ROOM, VTBL, SURF = CAR + 0x5000, CAR + 0x5100, CAR + 0x5200
h = Harness('golden_ctor', ['use ht2_sim::{ctor, contact};'])
e = h.e
world_stub.install(e)
world_stub.install_rooms(e)
e.mu.mem_map(0x7f0000, 0x1000)
e.hook_ret(0x6037e0, lambda e: 0)
e.call(0x501fb0, this=0x6976f8)
e.call(0x5021c0, this=0x697688)
e.call(0x503dd0)
CONST = [(0x7025f0, struct.pack('<12f', 1, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0)), (0x702620, struct.pack('<3f', 0, 1, 0)),
         (0x702630, struct.pack('<3f', 0, 0, 1)), (0x702640, struct.pack('<3f', 1, 0, 0)), (0x702650, bytes(12)),
         (0x6974a8, struct.pack('<3f', 1, 1, 1))]
CONST += [(0x697688, bytes(e.read(0x697688, 0x38 * 2 + 56 * 4))), (0x697678, bytes(e.read(0x697678, 16))), (0x696bdc, pack('f', 1.4)),
          (0x7f0004, pack('i', 12345)), (0x6749f0, bytes(e.read(0x6749f0, 4))), (0x6ce33c, bytes(e.read(0x6ce33c, 8))),
          (0x696ec0, b''.join(struct.pack('<f', _m.sin((i * 3.1415926 + i * 3.1415926) / 360.0)) for i in range(360))),
          (0x697670, pack('i', 0)), (0x69784c, pack('i', 0)), (0x6ef1fc, pack('u', 0))]
PREFILL = int(os.environ.get('HT2_CTOR_PREFILL', 'dd'), 16)
r = h.rnd
for k in range(int(os.environ.get('HT2_CTOR_CASES', '40'))):
    vid = r.choice(IDS)
    tech = 0x697868 + vid * 0x970
    n = r.choice([4, 4, 6])
    w = [(tech, bytes(e.read(tech, 0x970)))]
    w += [(tech + 0x860, pack('i', n)), (tech + 0x864, pack('i', 0))]
    track, base = r.uniform(1.6, 2.2), r.uniform(3.0, 5.0)
    for i in range(8):
        if i < n:
            x = (i % 2 - 0.5) * track
            y = base * 0.5 - (i // 2) * base / (n // 2 - 1)
            z = r.uniform(-0.6, -0.3)
        else:
            x = y = z = 0.0
        w += [(tech + 0x1a8 + 12 * i, struct.pack('<3f', x, y, z))]
    a = r.uniform(-0.05, 0.05)
    model = [_m.cos(a), _m.sin(a), 0, -_m.sin(a), _m.cos(a), 0, 0, 0, 1, r.uniform(-0.1, 0.1), r.uniform(-0.3, 0.3), r.uniform(0.0, 0.5)]
    w += [(0x6d2200 + 0x1b0 + vid * 0x5d0, struct.pack('<12f', *model))]
    t = r.uniform(-3.1, 3.1)
    pos = [r.uniform(-50, 50), r.uniform(-50, 50), r.uniform(0, 10)]
    M = [_m.cos(t), _m.sin(t), 0, -_m.sin(t), _m.cos(t), 0, 0, 0, 1] + pos
    M = [struct.unpack('<f', struct.pack('<f', v))[0] for v in M]
    ground = pos[2] - r.uniform(0.2, 1.5) if r.random() < 0.8 else pos[2] - 50
    w += world_stub.make_obj(ROOM, VTBL, ground, SURF, ROOM, 0)
    w += [(SURF, pack('i', 1)), (SURF + 4, pack('i', r.choice([1, 1, 2, 3, 6]))), (SURF + 8, pack('i', 0)), (SURF + 12, pack('u', SURF + 0x20))]
    w += [(0x696bda, pack('h', r.choice([0, 1])))]
    if PREFILL:
        w += [(CAR, bytes([PREFILL]) * 0x4030)]
    ign = []
    for i in range(8):
        wb = CAR + 0xf8 * (i + 1)
        ign += [(b + o, nn) for b in (wb + 0x20, CAR + 0x3510 + 0xd0 * i, CAR + 0x2b50 + 0xf8 * i + 0x20, CAR + 0x1018 + 0xf8 * i + 0x20)
                for o, nn in ((0x70, 4), (0x7c, 0x44))]
    mat = ', '.join(rs_f32(v) for v in M)
    h.case(CONST + w,
           lambda e, M=M, vid=vid: e.call(0x4ebd00, [ROOM] + M + [vid], 'i' + 'f' * 12 + 'i', this=CAR, max_insn=40_000_000),
           f'ctor::fn_004ebd00(&mut m, &mut contact::StubWorld, {T}, {ROOM:#x}, [{mat}], {vid})',
           [(CAR, 0x4030), (HEAP_BASE, 0x100)], time=r.uniform(5, 40), dt=r.uniform(0.01, 0.04), ignore=ign)
h.save()

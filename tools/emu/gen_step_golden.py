"""Golden test for the whole vehicle step `_Car_V::run1` (0x4fdc80)."""
import math as _m
import os
import struct
import sys

sys.path.insert(0, os.path.dirname(__file__))
from harness import Harness, pack  # noqa: E402
from carstate import CAR, RUST_FILTERS, make_filters, update_writes, HEAP_WATCH  # noqa: E402
import world_stub  # noqa: E402

T = f'{CAR:#x}'
OBJ, VTBL, SURF = CAR + 0x5000, CAR + 0x5100, CAR + 0x5200
KICK = CAR + 0x5800
h = Harness('golden_step', ['use ht2_sim::{step, contact, filter};'])
e = h.e
world_stub.install(e)
e.mu.mem_map(0x7f0000, 0x1000)
e.hook_ret(0x6037e0, lambda e: 0)
e.call(0x501fb0, this=0x6976f8)
e.call(0x5021c0, this=0x697688)
e.call(0x503dd0)
CONST = [(0x7025f0, struct.pack('<12f', 1, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0)), (0x702620, struct.pack('<3f', 0, 1, 0)),
         (0x702630, struct.pack('<3f', 0, 0, 1)), (0x702640, struct.pack('<3f', 1, 0, 0)), (0x702650, bytes(12))]
CONST += [(0x697688, bytes(e.read(0x697688, 0x38 * 2 + 56 * 4))), (0x697678, bytes(e.read(0x697678, 16))), (0x696bdc, pack('f', 1.4)),
          (0x7f0004, pack('i', 12345)), (0x6749f0, bytes(e.read(0x6749f0, 4))), (0x6ce33c, bytes(e.read(0x6ce33c, 8))),
          (0x696ec0, b''.join(struct.pack('<f', _m.sin((i * 3.1415926 + i * 3.1415926) / 360.0)) for i in range(360))),
          (0x697670, pack('i', 0)), (0x69784c, pack('i', 0)), (0x6ef1fc, pack('u', KICK))]
# The original leaves the query result of wheels that borrow nothing uninitialised; cases where
# that matters (a leading wheel without ground) are left out, see contact::UNINIT.
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_ESP, UC_X86_REG_EAX
QUERY_RESULTS = []


def _on_query(uc, address, size, user):
    ret = struct.unpack('<I', uc.mem_read(uc.reg_read(UC_X86_REG_ESP), 4))[0]
    def _ret(uc2, a2, s2, u2):
        QUERY_RESULTS.append(uc2.reg_read(UC_X86_REG_EAX))
        uc2.hook_del(hk[0])
    hk = [uc.hook_add(UC_HOOK_CODE, _ret, begin=ret, end=ret)]


e.mu.hook_add(UC_HOOK_CODE, _on_query, begin=0x503e30, end=0x503e30)
r = h.rnd
for k in range(160):
    if len(h.cases) >= 120:
        break
    QUERY_RESULTS.clear()
    now = r.uniform(5, 40)
    w, vid, n = update_writes(e, r, now)
    t = r.uniform(-0.2, 0.2)
    M = [_m.cos(t), _m.sin(t), 0, -_m.sin(t), _m.cos(t), 0, 0, 0, 1, r.uniform(-3, 3), r.uniform(-3, 3), r.uniform(0.3, 1.0)]
    inv = [M[0], M[3], M[6], M[1], M[4], M[7], M[2], M[5], M[8]]
    tinv = [-sum(M[9 + kk] * M[3 * j + kk] for kk in range(3)) for j in range(3)]
    w += [(CAR + 0x10 + 4 * i, pack('f', v)) for i, v in enumerate(M)]
    w += [(CAR + 0x900 + 4 * i, pack('f', v)) for i, v in enumerate(inv + tinv)]
    n = 4
    w += [(CAR + 0xdd4, pack('i', n))]
    for i in range(6):
        wb = CAR + 0xf8 * (i + 1)
        mx, my = (i % 2) * 2.0 - 1.0, 1.8 - 1.8 * (i // 2) + r.uniform(-0.01, 0.01)
        w += [(CAR + 0x1cbc + 12 * i + 4 * c, pack('f', v)) for c, v in enumerate((mx, my, r.uniform(-0.2, 0.0)))]
        w += [(wb + 4 * c, pack('f', v)) for c, v in enumerate((mx, my, 0.0))]
        w += [(wb + 0x18, pack('h', r.choice([0, 1, 1]))), (wb + 0x1a, pack('h', 0)), (wb + 0x84, pack('u', r.choice([0, SURF])))]
        w += [(wb + 0xc + 4 * c, pack('f', v)) for c, v in enumerate((0.0, 0.0, 1.0))]
        w += [(wb + 0x8 - 4 + 0xf8 * 0, pack('f', my))]
        w += [(CAR + o + 4 * i, pack('f', v)) for o, v in ((0x1b74, r.uniform(0.3, 0.6)), (0x1f9c, r.uniform(0.1, 0.4)), (0x1964, r.uniform(0, 10)),
                                                             (0x1984, r.uniform(0, 20)), (0x1a7c, r.uniform(-1, 1)), (0x1a5c, r.uniform(-1, 1)),
                                                             (0xd20, r.uniform(1000, 20000)), (0x2670, r.uniform(0.3, 1.0)), (0x1d60, 0.5), (0x1d80, 0.5),
                                                             (0x1e84, 1.0), (0xebc, r.uniform(-5, 5)), (0xe9c, r.uniform(-3, 3)))]
        w += [(CAR + 0x1bf4 + 4 * i, pack('i', 1)), (CAR + 0x19a4 + 2 * i, pack('h', r.choice([0, 0, 1]))), (CAR + 0x1030 + 0xf8 * i, pack('h', 0))]
        w += [(CAR + 0x3cdc + 20 * i, pack('i', 0)), (CAR + 0x3ce0 + 20 * i, pack('i', 0)), (CAR + 0x3e1c + 20 * i, pack('i', 0))]
        w += [(CAR + 0x64 + 12 * i + 4 * c, pack('f', v)) for c, v in enumerate((mx, my, 0.0))]
    # the ground under the wheels: the leading wheels must touch it, otherwise the original reads
    # uninitialised results for the others (see contact::UNINIT)
    w += world_stub.make_obj(OBJ, VTBL, M[11] - r.uniform(0.35, 0.6), SURF, OBJ, 0)
    w += [(SURF, pack('i', 1)), (SURF + 4, pack('i', r.choice([1, 1, 2, 3, 6]))), (SURF + 8, pack('i', 0)), (SURF + 12, pack('u', SURF + 0x20))]
    for base in (0x3f24, 0x3f58):
        w += [(CAR + base + 4 * kk, pack('f', 1.0)) for kk in range(6)] + [(CAR + base + 0x24, pack('f', 1.0)), (CAR + base + 0x28, pack('i', 0)), (CAR + base + 0x2c, pack('i', 1))]
    vel = [r.uniform(-5, 5), r.uniform(-5, 15), r.uniform(-1, 1)]
    w += [(CAR + 0x40 + 4 * i, pack('f', v)) for i, v in enumerate(vel)] + [(CAR + 0x4c + 4 * i, pack('f', r.uniform(-500, 500))) for i in range(3)]
    w += [(CAR + 0x58 + 4 * i, pack('f', r.uniform(-0.3, 0.3))) for i in range(3)]
    w += [(CAR + 0xd0 + 4 * i, pack('f', v)) for i, v in enumerate((0.0, 0.0, 1.0))] + [(CAR + 0xc4 + 4 * i, pack('f', v)) for i, v in enumerate((0.0, 0.0, 1.0))]
    w += [(CAR + 0xd40 + 4 * i, pack('f', v)) for i, v in enumerate((0.0, 0.0, -9.81))]
    w += [(CAR + 0x1f48 + 4 * i, pack('f', v)) for i, v in enumerate((r.uniform(5000, 20000), r.uniform(1000, 5000), r.uniform(5000, 20000)))]
    w += [(CAR + 0x34 + 4 * i, pack('f', M[9 + i])) for i in range(3)] + [(CAR + 0x8f4 + 4 * i, pack('f', M[9 + i])) for i in range(3)]
    w += [(CAR + o, pack(kd, v)) for o, kd, v in ((0x3c54, 'i', 0), (0x2708, 'h', 0), (0x1a10, 'u', OBJ), (0xf4, 'u', OBJ), (0xdc0, 'f', r.uniform(0, 20)), (0x275c, 'i', 0),
                                                   (0x2640, 'f', r.uniform(3000, 12000)), (0x29d0, 'i', 0), (0x276c, 'i', 0), (0x3f4c, 'i', 0),
                                                   (0x8bc, 'i', 1), (0x8c4, 'i', 1), (0x8b8, 'i', 0), (0x19b9, 'b', 0), (0x19b4, 'b', 0), (0x19b5, 'b', 0),
                                                   (0x19b6, 'b', 0), (0x19b7, 'b', 0), (0x19b8, 'b', 0), (0x19ba, 'b', 0), (0xe38, 'f', 1.5), (0x27c0, 'i', r.choice([0, 0, 1])),
                                                   (0x2748, 'h', 0), (0x3cb4, 'i', 0), (0x1e04, 'i', 5), (0x1f60, 'f', 3.5), (0x278c, 'f', 0.0), (0x1abc, 'i', 0),
                                                   (0x2700 + 0xc0, 'i', 0), (0x3cc8, 'i', 0), (0x3ccc, 'i', 1), (0x1f7c, 'f', 0.3), (0x17e0, 'f', 0.0),
                                                   (0x2804, 'f', 100.0), (0x2808, 'f', 0.0), (0x280c, 'f', 0.0), (0x27c8, 'f', 0.0), (0x27cc, 'f', 1.0), (0x27d0, 'f', 0.5))]
    w.insert(0, (0x697868 + vid * 0x970, bytes(e.read(0x697868 + vid * 0x970, 0x970))))
    w += [(0x697868 + vid * 0x970 + 0xb8, pack('f', 2.0)), (0x697868 + vid * 0x970 + 0xbc, pack('f', 2.0))]
    w += [(CAR + 0x1f64 + 4 * i, pack('f', 2.0)) for i in range(4)]
    w += [(CAR + 0x1e1c + 4 * i, pack('f', r.uniform(0.2, 1.0))) for i in range(3)]  # shift times (0x501930 copies them)
    ign = []
    for i in range(6):
        wb = CAR + 0xf8 * (i + 1)
        # fields of the contact records the original leaves uninitialised (and their copies)
        ign += [(base + o, nn) for base in (wb + 0x20, CAR + 0x3510 + 0xd0 * i, CAR + 0x2b50 + 0xf8 * i + 0x20) for o, nn in ((0x70, 4), (0x7c, 0x44))]
    h.case(CONST + w, lambda e: e.call(0x4fdc80, this=CAR, max_insn=20_000_000), f'step::fn_004fdc80(&mut m, &mut contact::StubWorld, {T})',
           [(CAR, 0x6000), HEAP_WATCH], time=now, dt=r.uniform(0.01, 0.04), prepare=make_filters, setup_rust=RUST_FILTERS, ignore=ign)
    if 0 in QUERY_RESULTS[:2]:
        h.cases.pop()
h.save()

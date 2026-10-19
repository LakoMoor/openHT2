"""Golden test for the engine part of the _Car_V constructor (0x4ebd00).

The original constructor is run in the emulator with a fake world object; it stops at the first
part it cannot run without the game world, which is after the engine has been set up."""
import os
import struct
import sys

sys.path.insert(0, os.path.dirname(__file__))
from unicorn import UcError  # noqa: E402
from harness import Harness, pack, OBJ_BASE  # noqa: E402
from carstate import CAR, TECH_TABLE  # noqa: E402

h = Harness('golden_init', ['use ht2_sim::init;'])
WORLD = OBJ_BASE + 0x200000
VT = WORLD + 0x100
STUBFN = WORLD + 0x400
consts = [(0x7025f0, struct.pack('<12f', 1, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0)),
          (0x702630, struct.pack('<3f', 0, 0, 1)), (0x702620, struct.pack('<3f', 0, 1, 0)),
          (0x702640, struct.pack('<3f', 1, 0, 0)),
          (STUBFN, bytes([0x31, 0xc0, 0xc2, 0x08, 0x00])), (WORLD, struct.pack('<I', VT)),
          (VT, struct.pack('<64I', *([STUBFN] * 64)))]


def run_ctor(vid):
    def call(e):
        args = [WORLD] + [1, 0, 0, 0, 1, 0, 0, 0, 1, 100.0, 200.0, 300.0] + [vid]
        try:
            e.call(0x4ebd00, args, 'i' + 'f' * 12 + 'i', this=CAR, max_insn=5_000_000)
        except UcError:
            pass            # expected: the constructor needs the real world after the engine part
        return 0
    return call


def ranges(e):
    n0, n1 = (struct.unpack('<i', e.read(CAR + o + 0x1c, 4))[0] for o in (0x1b30, 0x1b50))
    t0, v0, t1, v1 = (struct.unpack('<I', e.read(CAR + o, 4))[0] for o in (0x1b30 + 8, 0x1b30 + 0xc, 0x1b50 + 8, 0x1b50 + 0xc))
    return [(CAR + 0x8cc, 4), (CAR + 0x1b30, 0x44), (CAR + 0x1b74, 0x40), (CAR + 0x1c14, 0x84), (CAR + 0x1cb8, 4), (CAR + 0x2640, 4),
            (CAR + 0x3bdc, 4), (CAR + 0x3ebc, 0x16c), (t0, n0 * 4), (v0, n0 * 4), (t1, n1 * 4), (v1, n1 * 4)]


def heap_start(e):
    t0 = struct.unpack('<I', e.read(CAR + 0x1b30 + 8, 4))[0]
    return f'm.set_heap_next({t0:#x});'


for vid in (0, 5, 8, 18, 19, 21, 22, 25, 27, 31, 33, 36, 38, 40):
    base = TECH_TABLE + vid * 0x970
    rec = h.e.read(base, 0x970)
    h.case([(base, rec)] + consts, run_ctor(vid), f'init::init_engine(&mut m, {CAR:#x}, {vid})', ranges,
           time=10.0, setup_rust=heap_start)
h.save()

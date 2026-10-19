"""Golden tests for the small gearbox helpers (0x4ff150..0x4ff790)."""
import os
import struct
import sys

sys.path.insert(0, os.path.dirname(__file__))
from harness import Harness, pack  # noqa: E402
from carstate import CAR, WATCH, RUST_FILTERS, car_writes, make_filters  # noqa: E402

h = Harness('golden_gearbox_helpers', ['use ht2_sim::{gearbox as gb, filter};'])
r = h.rnd
T = f'{CAR:#x}'


def num(x):
    return repr(float(struct.unpack('<f', struct.pack('<f', x))[0])) + 'f32'


def go(call, rust, ret=False, time=None, extra_writes=(), filters=False):
    w, vid, n = car_writes(h.e, r)
    w = list(w) + list(extra_writes)
    t = r.uniform(0, 30) if time is None else time
    h.case(w, call, rust, WATCH, ret=ret, time=t,
           prepare=make_filters if filters else None, setup_rust=RUST_FILTERS if filters else '')


for _ in range(40):
    g = r.randint(0, 14)
    go(lambda e, g=g: e.call(0x4ff150, [g], 'i', this=CAR), f'gb::fn_004ff150(&m, {T}, {g})', ret=True)
for _ in range(25):
    x = r.uniform(0, 500)
    go(lambda e, x=x: e.call(0x4ff3e0, [x], 'f', this=CAR), f'gb::fn_004ff3e0(&m, {T}, {num(x)})', ret=True)
    go(lambda e, x=x: e.call(0x4ff460, [x], 'f', this=CAR), f'gb::fn_004ff460(&m, {T}, {num(x)})', ret=True)
    y = r.uniform(-0.2, 1.2)
    go(lambda e, x=x, y=y: e.call(0x4ff4b0, [x, y], 'ff', this=CAR), f'gb::fn_004ff4b0(&m, {T}, {num(x)}, {num(y)})', ret=True)
    go(lambda e, x=x: e.call(0x4ff500, [x], 'f', this=CAR), f'gb::fn_004ff500(&m, {T}, {num(x)})', ret=True)
    g = r.randint(0, 14)
    go(lambda e, x=x, g=g: e.call(0x4ff570, [x, g], 'fi', this=CAR), f'gb::fn_004ff570(&m, {T}, {num(x)}, {g})', ret=True)
    go(lambda e, x=x: e.call(0x4ff550, [x], 'f', this=CAR), f'gb::fn_004ff550(&m, {T}, {num(x)})', ret=True)
for _ in range(20):
    go(lambda e: e.call(0x4ff4a0, this=CAR), f'gb::fn_004ff4a0(&m, {T})', ret=True)
    go(lambda e: e.call(0x4ff610, this=CAR), f'gb::fn_004ff610(&m, {T})', ret=True)
for _ in range(60):
    go(lambda e: e.call(0x4ff660, this=CAR), f'gb::fn_004ff660(&m, {T})', ret=True)
for mode in (0, 1, 2, 1, 0):
    go(lambda e, mode=mode: e.call(0x4ff770, [mode], 'i', this=CAR), f'gb::fn_004ff770(&mut m, {T}, {mode})')
for _ in range(15):
    go(lambda e: e.call(0x4ff790, this=CAR), f'gb::set_neutral(&mut m, {T})', filters=True)
# correct_tach: the input is read through a pointer
PTR = CAR + 0x3800
for _ in range(80):
    x = r.uniform(-30, 400)
    go(lambda e, x=x: e.call(0x4ff210, [PTR], 'i', this=CAR), f'gb::fn_004ff210(&mut m, {T}, {num(x)})',
       extra_writes=[(PTR, pack('f', x))], filters=True)
h.save()

"""Golden tests for the driver input handler (0x500850)."""
import os
import sys

sys.path.insert(0, os.path.dirname(__file__))
from harness import Harness, pack  # noqa: E402
from carstate import CAR, WATCH, RUST_FILTERS, make_filters, update_writes  # noqa: E402

h = Harness('golden_input', ['use ht2_sim::{input, filter};'])
r = h.rnd
for _ in range(400):
    now = r.uniform(5, 40)
    w, vid, n = update_writes(h.e, r, now)
    add = lambda off, kind, v: w.append((CAR + off, pack(kind, v)))
    add(0x2702, 'b', r.choice([0, 0, 1])); add(0x2703, 'b', r.choice([0, 0, 1]))
    add(0x2706, 'h', r.choice([0, 1])); add(0x2701, 'b', r.choice([0, 0, 1])); add(0x1b20, 'u', r.choice([0, 1]))
    add(0x2740, 'h', r.choice([0, 1, 2])); add(0x274c, 'i', r.choice([0, 1]))
    add(0x2728, 'f', r.uniform(0, 1.2)); add(0x2724, 'f', r.uniform(0, 3)); add(0x271c, 'f', r.uniform(-1.3, 1.3))
    add(0x29e8, 'i', r.choice([0, 1])); add(0x29e0, 'f', r.uniform(-1.2, 1.2))
    add(0x26b0, 'f', r.uniform(0, 5)); add(0x263c, 'f', r.uniform(0, 5)); add(0x1f60, 'f', r.uniform(0, 5))
    add(0xdc0, 'f', r.uniform(0.5, 40)); add(0xdc8, 'f', r.uniform(-30, 40))
    add(0x2730, 'd', now - r.uniform(0, 3)); add(0x2738, 'd', now - r.uniform(0, 3))
    add(0x271a, 'h', r.choice([0, 1])); add(0x2718, 'h', r.choice([0, 1, 2, 3]))
    h.case(w, lambda e: e.call(0x500850, this=CAR), f'input::fn_00500850(&mut m, {CAR:#x})', WATCH, time=now,
           prepare=make_filters, setup_rust=RUST_FILTERS)
h.save()

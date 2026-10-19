"""Golden tests for the engine/gearbox update (0x4ff800)."""
import os
import sys

sys.path.insert(0, os.path.dirname(__file__))
from harness import Harness  # noqa: E402
from carstate import CAR, WATCH, RUST_FILTERS, make_filters, update_writes  # noqa: E402

h = Harness('golden_gearbox_update', ['use ht2_sim::{gearbox_update as gu, filter};'])
r = h.rnd
for _ in range(300):
    now = r.uniform(5, 40)
    w, vid, n = update_writes(h.e, r, now)
    h.case(w, lambda e: e.call(0x4ff800, this=CAR), f'gu::fn_004ff800(&mut m, {CAR:#x})', WATCH, time=now,
           prepare=make_filters, setup_rust=RUST_FILTERS)
h.save()

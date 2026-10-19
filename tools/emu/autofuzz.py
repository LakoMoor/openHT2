"""Automatic random vehicle state for a function, derived from its decompiled text.

The decompiled C (kept outside the repo, in DECOMP_DIR) tells which byte offsets of `param_1`
(the vehicle object) a function touches and with which type. Floats get random values, ints and
shorts get small values, bytes get 0/1. Known counters and flags can be pinned in OVERRIDES.
"""
import glob
import os
import random
import re
import struct

from harness import pack, OBJ_BASE

DECOMP_DIR = os.environ.get('HT2_DECOMP', r'E:\tools\decomp\all')
CAR = OBJ_BASE + 0x1000
CAR_LEN = 0x4000

_funcs = None


def load_funcs():
    global _funcs
    if _funcs is not None:
        return _funcs
    _funcs = {}
    for f in sorted(glob.glob(os.path.join(DECOMP_DIR, '*.c'))):
        t = open(f, encoding='utf-8', errors='replace').read()
        for p in re.split(r'(?m)^// ==== ', t)[1:]:
            m = re.match(r'(\S+) @ (\w+) \((\d+) bytes\)', p)
            if m:
                _funcs[int(m.group(2), 16)] = p
    return _funcs


KINDS = {'float': 'f', 'int': 'i', 'uint': 'i', 'short': 'h', 'ushort': 'h', 'char': 'b', 'byte': 'b',
         'undefined1': 'b', 'undefined2': 'h', 'undefined4': 'f', 'undefined8': 'd', 'double': 'd', 'bool': 'b'}
ACCESS = re.compile(r'\*\((\w+) \*\)\((?:param_1|\(int\)param_1|this) \+ (0x[0-9a-f]+|\d+)\)')


def fields_of(addr, depth=2, seen=None):
    """{offset: kind} accessed by the function (and the functions it calls, `depth` levels)."""
    seen = set() if seen is None else seen
    funcs = load_funcs()
    out = {}
    if addr in seen or addr not in funcs:
        return out
    seen.add(addr)
    text = funcs[addr]
    for t, off in ACCESS.findall(text):
        if t in KINDS:
            out.setdefault(int(off, 0), KINDS[t])
    if depth:
        for c in re.findall(r'FUN_([0-9a-f]{8})', text):
            for o, k in fields_of(int(c, 16), depth - 1, seen).items():
                out.setdefault(o, k)
    return out


# Fixed or restricted fields: offset -> callable(rnd) -> (kind, value)
OVERRIDES = {
    0xdd4: lambda r: ('i', r.randint(1, 4)),      # number of wheels
    0x8cc: lambda r: ('i', r.choice([0, 5, 8, 18, 19, 21, 25, 31, 33, 38])),   # tech record
}


def state(addr, r, extra=None, overrides=None, fill=()):
    """Random writes for the object. `extra`: additional {offset: kind}."""
    fields = fields_of(addr)
    if extra:
        fields.update(extra)
    ov = dict(OVERRIDES)
    if overrides:
        ov.update(overrides)
    w = []
    for lo, hi in fill:
        # Ranges that are copied or read wholesale get random floats so that they carry real data.
        for off in range(lo, hi, 4):
            w.append((CAR + off, pack('f', r.uniform(-2, 2))))
    for off, kind in sorted(fields.items()):
        if any(lo <= off < hi for lo, hi in fill) and off not in ov and kind == 'f':
            continue
        if off in ov:
            k, v = ov[off](r)
            w.append((CAR + off, pack(k, v)))
        elif kind == 'i':
            w.append((CAR + off, pack('i', r.choice([0, 1, 1, 2, 3, -1]))))
        elif kind == 'h':
            w.append((CAR + off, pack('h', r.choice([0, 0, 1, 2, -1]))))
        elif kind == 'b':
            w.append((CAR + off, pack('b', r.choice([0, 1]))))
        elif kind == 'd':
            w.append((CAR + off, pack('d', r.uniform(-2, 2))))
        elif kind == 'f':
            w.append((CAR + off, pack('f', r.uniform(-2, 2))))
    return w, fields

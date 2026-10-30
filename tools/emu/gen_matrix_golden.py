"""Golden tests for the vector/matrix helpers (matrix.rs)."""
import math
import os
import sys

sys.path.insert(0, os.path.dirname(__file__))
from harness import Harness, pack  # noqa: E402
from autofuzz import CAR  # noqa: E402

h = Harness('golden_matrix', ['use ht2_sim::matrix;'])
r = h.rnd
WATCH = [(CAR + 0x3800, 0x400)]
A, B, C, D = CAR + 0x3800, CAR + 0x3840, CAR + 0x3880, CAR + 0x38c0
OUT = CAR + 0x3900
I = [0, 1, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0]
IDENT = [(0x7025f0 + 4 * i, pack('f', float(v))) for i, v in enumerate([1, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0])]


def vec(a, lo=-50, hi=50):
    return [(a + 4 * i, pack('f', r.uniform(lo, hi))) for i in range(3)]


def mat(a):
    return [(a + 4 * i, pack('f', r.uniform(-1.2, 1.2))) for i in range(12)]


def rot(a):
    t = r.uniform(-3, 3)
    c, s = math.cos(t), math.sin(t)
    return [(a + 4 * i, pack('f', v)) for i, v in enumerate([c, s, 0, -s, c, 0, 0, 0, 1, r.uniform(-9, 9), r.uniform(-9, 9), r.uniform(-9, 9)])]


T = lambda x: f'{x:#x}'
for _ in range(40):
    w = vec(A) + vec(B)
    s = r.uniform(-4, 4)
    sf = pack('f', s)
    import struct
    s32 = struct.unpack('<f', sf)[0]
    h.case(w, lambda e: e.call(0x4bddb0, [C, A, B], 'iii'), f'matrix::vec_add(&mut m, {T(C)}, {T(A)}, {T(B)})', WATCH)
    h.case(w, lambda e: e.call(0x4bde00, [C, A, B], 'iii'), f'matrix::vec_sub(&mut m, {T(C)}, {T(A)}, {T(B)})', WATCH)
    h.case(w, lambda e: e.call(0x4bde50, [C, A, B], 'iii'), f'matrix::vec_cross(&mut m, {T(C)}, {T(A)}, {T(B)})', WATCH)
    h.case(w, lambda e: e.call(0x4cb270, [C, A], 'ii'), f'matrix::vec_neg(&mut m, {T(C)}, {T(A)})', WATCH)
    h.case(w, lambda e, s=s: e.call(0x4bdd60, [C, A, s], 'iif'), f'matrix::vec_scale(&mut m, {T(C)}, {T(A)}, {s32!r}f32)', WATCH)
    h.case(w, lambda e, s=s: e.call(0x4e10d0, [C, A, s], 'iif'), f'matrix::vec_div_scalar(&mut m, {T(C)}, {T(A)}, {s32!r}f32)', WATCH)
    h.case(w, lambda e: e.call(0x610f00, [A, B], 'ii'), f'matrix::vec_dot(&m, {T(A)}, {T(B)}) as f32', WATCH, ret=True)
    h.case(w, lambda e, s=s: e.call(0x5ab620, [C, s], 'if', this=A), f'matrix::vec_scale_in_place(&mut m, {T(A)}, {T(C)}, {s32!r}f32)', WATCH)
    h.case(w, lambda e, s=s: e.call(0x5ab660, [C, s], 'if', this=A), f'matrix::vec_div_in_place(&mut m, {T(A)}, {T(C)}, {s32!r}f32)', WATCH)
    h.case(w, lambda e: e.call(0x5ab710, [C, B], 'ii', this=A), f'matrix::vec_add_in_place(&mut m, {T(A)}, {T(C)}, {T(B)})', WATCH)
    h.case(w, lambda e: e.call(0x5ab750, [C, B], 'ii', this=A), f'matrix::vec_sub_in_place(&mut m, {T(A)}, {T(C)}, {T(B)})', WATCH)
    h.case(w, lambda e: e.call(0x5ab400, [C, A], 'ii'), f'matrix::vec_unit(&mut m, {T(C)}, {T(A)})', WATCH)
    h.case(vec(A, -1e-6, 1e-6), lambda e: e.call(0x5ab400, [C, A], 'ii'), f'matrix::vec_unit(&mut m, {T(C)}, {T(A)})', WATCH)
    nb = [(B + 4 * i, w[i][1]) for i in range(3)]
    h.case(w[:3] + nb, lambda e: e.call(0x5ab6a0, [A, B], 'ii'), f'matrix::vec_near(&m, {T(A)}, {T(B)})', WATCH, ret=True, ret_int=True)
    # a rotation matrix with translation at D
    wm = mat(D) if r.random() < 0.5 else rot(D)
    h.case(vec(A) + wm, lambda e: e.call(0x4cb2b0, [C, A, D], 'iii'), f'matrix::rotate(&mut m, {T(C)}, {T(A)}, {T(D)})', WATCH)
    h.case(vec(A) + wm, lambda e: e.call(0x43f5b0, [C, A, D], 'iii'), f'matrix::transform(&mut m, {T(C)}, {T(A)}, {T(D)})', WATCH)
    h.case(vec(A) + wm, lambda e: e.call(0x5ab950, [D], 'i', this=A), f'matrix::transform_in_place(&mut m, {T(A)}, {T(D)})', WATCH)
    h.case(vec(A) + wm, lambda e: e.call(0x5ab9b0, [D], 'i', this=A), f'matrix::rotate_in_place(&mut m, {T(A)}, {T(D)})', WATCH)
    wa = mat(A) if r.random() < 0.5 else rot(A)
    h.case(wa + wm, lambda e: e.call(0x5aba40, [C, A, D], 'iii'), f'matrix::mat_mul(&mut m, {T(C)}, {T(A)}, {T(D)})', WATCH)
    h.case(wa + wm, lambda e: e.call(0x5abbd0, [D], 'i', this=A), f'matrix::mat_apply(&mut m, {T(A)}, {T(D)})', WATCH)
    x, y, z = [r.uniform(-30, 30) for _ in range(3)]
    ang = r.uniform(-7, 7)
    sx, sy, sz, sa = [struct.unpack('<f', pack('f', v))[0] for v in (x, y, z, ang)]
    h.case(IDENT, lambda e: e.call(0x5abc40, [C, x, y, z], 'ifff'), f'matrix::mat_translation(&mut m, {T(C)}, {sx!r}f32, {sy!r}f32, {sz!r}f32)', WATCH)
    h.case(IDENT, lambda e: e.call(0x5abe90, [C, x, y, z], 'ifff'), f'matrix::mat_scale(&mut m, {T(C)}, {sx!r}f32, {sy!r}f32, {sz!r}f32)', WATCH)
    h.case([], lambda e: e.call(0x5ac0c0, [C, ang], 'if'), f'matrix::mat_rot_z(&mut m, {T(C)}, {sa!r}f32)', WATCH)
    h.case([], lambda e: e.call(0x5abfd0, [C, ang], 'if'), f'matrix::mat_rot_y(&mut m, {T(C)}, {sa!r}f32)', WATCH)
    h.case([], lambda e: e.call(0x5abee0, [C, ang], 'if'), f'matrix::mat_rot_x(&mut m, {T(C)}, {sa!r}f32)', WATCH)
    if r.random() < 0.15:
        x = 0.0
    if r.random() < 0.1:
        x = y = 0.0
    sx, sy = struct.unpack('<f', pack('f', x))[0], struct.unpack('<f', pack('f', y))[0]
    h.case([], lambda e: e.call(0x5ab500, [x, y, z, C, D], 'fffii'),
           f'{{ let (a, b) = matrix::dir_to_angles(&mut m, {sx!r}f32, {sy!r}f32, {sz!r}f32); m.set_f32({T(C)}, a); m.set_f32({T(D)}, b); }}', WATCH)
for _ in range(60):
    p = [r.uniform(-20, 20) for _ in range(3)]
    d = [r.uniform(-1, 1) for _ in range(3)]
    if r.random() < 0.1:
        d = [0.0, 0.0, 0.0]
    ro = r.uniform(-4, 4)
    q = [struct.unpack('<f', pack('f', v))[0] for v in p + d + [ro]]
    h.case(IDENT, lambda e: e.call(0x5abc90, [C] + p + d + [ro], 'i' + 'f' * 7),
           f'matrix::mat_look(&mut m, {T(C)}, [{q[0]!r}f32, {q[1]!r}f32, {q[2]!r}f32], [{q[3]!r}f32, {q[4]!r}f32, {q[5]!r}f32], {q[6]!r}f32)', WATCH)
h.save()

# 5da950, 4e6362, 5de950 (added later)
h = Harness('golden_matrix2', ['use ht2_sim::matrix;', 'use ht2_sim::chassis;'])
r = h.rnd
import math as _m
for _ in range(80):
    vals = [r.uniform(-30, 30) for _ in range(3)] + [r.uniform(-4, 4) for _ in range(3)]
    h.case(IDENT, lambda e, vals=vals: e.call(0x5da950, [C] + vals, 'i' + 'd' * 6),
           f'matrix::mat_from_euler(&mut m, {T(C)}, {vals[0]!r}f64, {vals[1]!r}f64, {vals[2]!r}f64, {vals[3]!r}f64, {vals[4]!r}f64, {vals[5]!r}f64)', WATCH)
for _ in range(80):
    w = vec(A) + vec(B) + vec(D)
    h.case(IDENT + w, lambda e: e.call(0x4e6362, [C, A, B, D], 'iiii'), f'matrix::point_to_plane(&mut m, {T(C)}, {T(A)}, {T(B)}, {T(D)})', WATCH)
OUTS = [CAR + 0x3a00 + 8 * k for k in range(6)]
for k in range(200):
    if k % 3 == 0:
        t, u, v = r.uniform(-3, 3), r.uniform(-1.4, 1.4), r.uniform(-3, 3)
        a = [_m.cos(t), _m.sin(t), 0, -_m.sin(t), _m.cos(t), 0, 0, 0, 1]
        mat = [x + r.uniform(-0.05, 0.05) for x in a] + [r.uniform(-50, 50) for _ in range(3)]
    else:
        mat = [r.uniform(-1, 1) for _ in range(9)] + [r.uniform(-50, 50) for _ in range(3)]
    # orthonormal-ish: build from random rotations
    ax, ay, az = r.uniform(-3, 3), r.uniform(-1.5, 1.5), r.uniform(-3, 3)
    cz, sz, cx, sx, cy, sy = _m.cos(az), _m.sin(az), _m.cos(ax), _m.sin(ax), _m.cos(ay), _m.sin(ay)
    if k % 3 != 0:
        Rz = [[cz, sz, 0], [-sz, cz, 0], [0, 0, 1]]
        Rx = [[1, 0, 0], [0, cx, sx], [0, -sx, cx]]
        Ry = [[cy, 0, -sy], [0, 1, 0], [sy, 0, cy]]
        mm = lambda a, b: [[sum(a[i][k2] * b[k2][j] for k2 in range(3)) for j in range(3)] for i in range(3)]
        Mx = mm(mm(Ry, Rx), Rz)
        mat = [Mx[i][j] for i in range(3) for j in range(3)] + mat[9:]
    fm = [struct.unpack('<f', pack('f', v))[0] for v in mat]
    h.case([], lambda e, mat=mat: e.call(0x5de950, mat + OUTS, 'f' * 12 + 'i' * 6),
           f'chassis::fn_005de950(&mut m, [{", ".join(repr(v) + "f32" for v in fm)}], {OUTS[0]:#x}, {OUTS[1]:#x}, {OUTS[2]:#x}, {OUTS[3]:#x}, {OUTS[4]:#x}, {OUTS[5]:#x})',
           [(CAR + 0x3a00, 0x40)])
h.save()

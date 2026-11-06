"""0x5818d0 (cdecl (a, b, m2, out_pos, out_ang): position difference a-b and the angles of a*m2) run unmodified with every callee
real (0x5aba40, 0x4e2fb4, 0x5de950 and its helpers). Row: a, b, m2 (12 floats), out_pos, out_ang."""
import math, pathlib, random, struct
from emu import Emu, OBJ_BASE

r = random.Random(0x5818d0)
e = Emu()
e.install_crt_hooks()
F = lambda *v: struct.pack('<%df' % len(v), *v)
A, B, M2, OP, OA = [OBJ_BASE + 0x10000 + 0x1000 * i for i in range(5)]
e.write(0x7025f0, F(1, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0)); e.write(0x702620, F(0, 1, 0)); e.write(0x702630, F(0, 0, 1)); e.write(0x702640, F(1, 0, 0)); e.write(0x702650, F(0, 0, 0))
def rot():
    a, b, c = [r.uniform(-math.pi, math.pi) for _ in range(3)]
    if r.random() < 0.15:
        a, b, c = [r.choice([0.0, math.pi / 2, -math.pi / 2, math.pi]) for _ in range(3)]
    ca, sa, cb, sb, cc, sc = math.cos(a), math.sin(a), math.cos(b), math.sin(b), math.cos(c), math.sin(c)
    rx = [[1, 0, 0], [0, ca, sa], [0, -sa, ca]]; ry = [[cb, 0, -sb], [0, 1, 0], [sb, 0, cb]]; rz = [[cc, sc, 0], [-sc, cc, 0], [0, 0, 1]]
    mul = lambda p, q: [[sum(p[i][k] * q[k][j] for k in range(3)) for j in range(3)] for i in range(3)]
    mm = mul(mul(rx, ry), rz)
    return [x for row in mm for x in row] + [r.uniform(-20, 20) for _ in range(3)]
rows = []
for case in range(300):
    for a in (A, B, M2, OP, OA): e.write(a, bytes(0x100))
    a, b, m2 = rot(), rot(), rot()
    e.write(A, F(*a)); e.write(B, F(*b)); e.write(M2, F(*m2))
    e.call(0x5818d0, [A, B, M2, OP, OA], 'iiiii')
    rows.append(chr(9).join([F(*a).hex(), F(*b).hex(), F(*m2).hex(), e.read(OP, 12).hex(), e.read(OA, 12).hex()]))
pathlib.Path(__file__).with_name('golden_pose_delta.tsv').write_text(chr(10).join(rows) + chr(10))
print(len(rows), rows[0][-50:])

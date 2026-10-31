"""0x5843c0 (body ray test) run unmodified including 0x4e6578 and 0x4e5698; the room object's virtual [vtable+0x34](4, rec)
is a recording stub that fills the hit (matrix, point, normal) with random data and returns a random flag. Row: body words
(sparse), target +0x24 vector, ratio, rec input words, hit (hex of 0x70 bytes), flag, result, final ratio/distance/rec
(0x80 bytes), virtual call args."""
import pathlib, random, struct
from emu import Emu, OBJ_BASE, STUB

r = random.Random(0x5843c0)
e = Emu()
BODY, TGT, RATIO, DIST, REC, ROOM, VT = [OBJ_BASE + 0x10000 + 0x2000 * i for i in range(7)]
F = lambda *v: struct.pack('<%df' % len(v), *v)
e.write(0x7025f0, F(1, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0))
HIT = {}
calls = []
V34 = STUB + 0x500
e.set_u32(VT + 0x34, V34)
def v34(emu):
    rec = emu.arg(1)
    calls.append('%x|%x|%x|%s' % (emu.ecx, emu.arg(0), rec, e.read(rec + 0x4c, 24).hex()))
    e.write(rec + 4, HIT['mat'])
    e.write(rec + 0x34, HIT['pn'])
    return HIT['flag']
e.hook_thiscall(V34, v34, 2)
rows = []
val = lambda: r.choice([0.0, 1.0, -1.0, 0.5, r.uniform(-5, 5), r.uniform(-50, 50)])
for case in range(300):
    calls.clear()
    for a in (BODY, TGT, RATIO, DIST, REC, ROOM):
        e.write(a, bytes(0x1000))
    e.set_u32(ROOM, VT)
    alt = r.choice([0, 0, 1])
    body = {0x5a0: struct.pack('<I', alt), 0x320: struct.pack('<I', ROOM), 0x7f0: struct.pack('<I', ROOM)}
    for off in (0x1f4, 0x700):
        body[off + 0x24] = F(val(), val(), val())
    for o, b in body.items():
        e.write(BODY + o, b)
    tgt = F(val(), val(), val())
    if r.random() < 0.1:
        tgt = e.read(BODY + (0x700 if alt else 0x1f4) + 0x24, 12)
    e.write(TGT + 0x24, tgt)
    ratio = F(r.choice([1.0, 2.0, 0.5, r.uniform(0, 3)]))
    e.write(RATIO, ratio)
    e.write(DIST, F(-77.0))
    e.write(REC, bytes(0x80))
    HIT['mat'] = F(1, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0) if r.random() < 0.5 else F(*[r.uniform(-1.5, 1.5) for _ in range(12)])
    nrm = [r.uniform(-1, 1) for _ in range(3)]
    p = [val() for _ in range(3)]
    if r.random() < 0.5:   # hit on the segment: start + t * (end - start)
        s = struct.unpack('<3f', e.read(BODY + (0x700 if alt else 0x1f4) + 0x24, 12))
        t = struct.unpack('<3f', tgt)
        k = r.choice([0.0, 0.3, 0.9, 1.0, 1.5])
        p = [s[i] + k * (t[i] - s[i]) for i in range(3)]
    HIT['pn'] = F(*p, *nrm)
    HIT['flag'] = r.choice([0, 1, 1, 1, 2, 3])
    e.call(0x5843c0, [BODY, TGT, RATIO, DIST, REC], 'iiiii')
    res = e.eax
    rows.append(chr(9).join([str(alt), (body_hex := ','.join('%x:%s' % (o, b.hex()) for o, b in body.items())), tgt.hex(), ratio.hex(),
                             HIT['mat'].hex(), HIT['pn'].hex(), str(HIT['flag']), '%x' % res, e.read(RATIO, 4).hex(), e.read(DIST, 4).hex(),
                             e.read(REC, 0x80).hex(), ';'.join(calls)]))
pathlib.Path(__file__).with_name('golden_body_ray.tsv').write_text(chr(10).join(rows) + chr(10))
from collections import Counter
print(len(rows), Counter(x.split(chr(9))[7] for x in rows))

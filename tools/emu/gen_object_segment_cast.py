"""0x4e6d41 (segment cast against the room objects, cdecl (a, room, radius, p0, p1, rec)) run unmodified; the room query
0x4e623e(room, mode, rec) is a recording stub that writes random words into the hit area of rec and returns random flags.
Row: a, room, radius, p0, p1, rec input words, stub flags, result, final rec (0x80 bytes), stub call."""
import pathlib, random, struct
from emu import Emu, OBJ_BASE

r = random.Random(0x4e6d41)
e = Emu()
REC, P0, P1 = OBJ_BASE + 0x10000, OBJ_BASE + 0x11000, OBJ_BASE + 0x11100
F = lambda *v: struct.pack('<%df' % len(v), *v)
e.write(0x7025f0, F(1, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0))
st = {}
calls = []
def q(emu):
    room, mode, rec = emu.arg(0), emu.arg(1), emu.arg(2)
    calls.append('%x|%x|%x|%s' % (room, mode, rec, e.read(rec, 0x7c).hex()))
    e.write(rec + 0x34, st['hit'])
    return st['flags']
e.hook_ret(0x4e623e, q)
rows = []
for case in range(200):
    calls.clear()
    e.write(REC, bytes(r.randrange(256) for _ in range(0x80)))
    a, room = r.randrange(1 << 32), r.randrange(1 << 32)
    rad = F(r.choice([0.0, 0.5, r.uniform(0, 3)]))
    p0, p1 = F(*[r.uniform(-9, 9) for _ in range(3)]), F(*[r.uniform(-9, 9) for _ in range(3)])
    e.write(P0, p0); e.write(P1, p1)
    st['hit'] = bytes(r.randrange(256) for _ in range(0x30)); st['flags'] = r.choice([0, 1, 2, 3, 0x80000001, 0x10])
    e.call(0x4e6d41, [a, room, struct.unpack('<I', rad)[0], P0, P1, REC], 'iiiiii')
    rows.append(chr(9).join(['%x' % a, '%x' % room, rad.hex(), p0.hex(), p1.hex(), st['hit'].hex(), '%x' % st['flags'], '%x' % e.eax, e.read(REC, 0x80).hex(), ';'.join(calls)]))
pathlib.Path(__file__).with_name('golden_object_segment_cast.tsv').write_text(chr(10).join(rows) + chr(10))
print(len(rows)); print(rows[0][:200])

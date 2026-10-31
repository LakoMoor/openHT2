"""0x4e5a45 (cdecl (obj, p0, p1, rec, radius): 0x4e560a fills the record and calls [obj->vtable+0x34](6, rec); on a hit bit
0x4e5698 brings the hit back) run unmodified; the virtual is a recording stub. Row: obj kind (0 null / 1 obj), p0, p1,
radius, stub matrix, stub point+normal, flags, result, final rec (0x80 bytes), calls."""
import pathlib, random, struct
from emu import Emu, OBJ_BASE, STUB

r = random.Random(0x4e5a45)
e = Emu()
OBJ, VT, REC, P0, P1 = [OBJ_BASE + 0x10000 + 0x2000 * i for i in range(5)]
F = lambda *v: struct.pack('<%df' % len(v), *v)
e.write(0x7025f0, F(1, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0))
st, calls = {}, []
V34 = STUB + 0x500
e.set_u32(VT + 0x34, V34)
def v34(emu):
    rec = emu.arg(1)
    calls.append('%x|%x|%x|%s' % (emu.ecx, emu.arg(0), rec, e.read(rec, 0x7c).hex()))
    e.write(rec + 4, st['mat']); e.write(rec + 0x34, st['pn'])
    return st['flags']
e.hook_thiscall(V34, v34, 2)
rows = []
for case in range(250):
    calls.clear()
    e.write(OBJ, bytes(0x100)); e.set_u32(OBJ, VT)
    e.write(REC, bytes(r.randrange(256) for _ in range(0x80)))
    has = r.choice([0, 1, 1, 1])
    rad = F(r.choice([0.0, 0.5, r.uniform(0, 3)]))
    p0, p1 = F(*[r.uniform(-9, 9) for _ in range(3)]), F(*[r.uniform(-9, 9) for _ in range(3)])
    e.write(P0, p0); e.write(P1, p1)
    st['mat'] = F(1, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0) if r.random() < 0.5 else F(*[r.uniform(-1.5, 1.5) for _ in range(12)])
    st['pn'] = F(*[r.uniform(-9, 9) for _ in range(6)])
    st['flags'] = r.choice([0, 1, 2, 3, 0x80000001])
    e.call(0x4e5a45, [OBJ if has else 0, P0, P1, REC, struct.unpack('<I', rad)[0]], 'iiiii')
    rows.append(chr(9).join([str(has), p0.hex(), p1.hex(), rad.hex(), st['mat'].hex(), st['pn'].hex(), '%x' % st['flags'], '%x' % e.eax, e.read(REC, 0x80).hex(), ';'.join(calls)]))
pathlib.Path(__file__).with_name('golden_object_cast.tsv').write_text(chr(10).join(rows) + chr(10))
print(len(rows), sum(1 for x in rows if x.split(chr(9))[9]), 'with calls')

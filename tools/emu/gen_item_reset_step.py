"""0x583c30 (MovingItem virtual +0xcc, thiscall(flags), ret 4) run unmodified; the virtuals +0xa4 (V3, D) and +0x74 are recording
stubs (the vectors are read when called). Row: flags, this words, diff of this, events."""
import pathlib, random, struct
from emu import Emu, OBJ_BASE, STUB

r = random.Random(0x583c30)
e = Emu()
F = lambda *v: struct.pack('<%df' % len(v), *v)
THIS, VT = OBJ_BASE + 0x10000, OBJ_BASE + 0x12000
A4, V74 = STUB + 0x640, STUB + 0x680
ev = []
e.hook_thiscall(A4, lambda emu: (ev.append('a4|%s|%s' % (e.read(emu.arg(0), 12).hex(), e.read(emu.arg(1), 12).hex())), 0)[1], 2)
e.hook_thiscall(V74, lambda emu: (ev.append('74'), 0)[1], 0)
e.set_u32(VT + 0xa4, A4); e.set_u32(VT + 0x74, V74)
val = lambda k: r.choice([0.0, 1.0, -1.0, r.uniform(-k, k)])
rows = []
for case in range(300):
    ev.clear()
    e.write(THIS, bytes(0x1000)); e.set_u32(THIS, VT)
    w = {0x598: struct.pack('<i', r.choice([0, 1, 5, 9])), 0x690: struct.pack('<i', r.choice([-1, 0, 4, 5, 9])),
         0x1a0: F(val(9)), 0x1a4: F(val(9)), 0x1a8: F(val(9)), 0x7e4: F(val(9)), 0x7e8: F(val(9)), 0x7ec: F(val(9)),
         0x18c: F(r.choice([0.0, 0.5, 1.0, val(3)])), 0x688: struct.pack('<I', r.choice([0, 0x2000000, 0x2000001, 0x40])),
         0x5ec: F(val(5)), 0x5f0: F(val(5)), 0x5f4: F(val(5))}
    for o, b in w.items(): e.write(THIS + o, b)
    flags = r.choice([0, 0, 1, 4, 5, 0x104])
    before = e.read(THIS, 0x1000)
    e.call(0x583c30, [flags], 'i', this=THIS)
    after = e.read(THIS, 0x1000)
    diff = ','.join('%x:%s' % (i, after[i:i + 4].hex()) for i in range(0, 0x1000, 4) if after[i:i + 4] != before[i:i + 4])
    rows.append(chr(9).join([str(flags), ','.join('%x:%s' % (o, b.hex()) for o, b in w.items()), diff, ';'.join(ev)]))
pathlib.Path(__file__).with_name('golden_item_reset_step.tsv').write_text(chr(10).join(rows) + chr(10))
print(len(rows), sum(1 for x in rows if x.split(chr(9))[3]), 'with calls')

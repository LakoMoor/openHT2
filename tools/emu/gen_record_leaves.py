"""0x58d1f0 (thiscall, ret 4: next 28-byte record of the list at +0xf94), 0x58d250 (thiscall, ret 8: record by index) and 0x58b4f0
(thiscall: zero a 28-byte record) run unmodified. Row: kind, words, arg, result, diff of this (0x1800), out hex."""
import pathlib, random, struct
from emu import Emu, OBJ_BASE

r = random.Random(0x58d1f0)
e = Emu()
U = lambda *v: struct.pack('<%dI' % len(v), *v)
F = lambda *v: struct.pack('<%df' % len(v), *v)
THIS, OUT = OBJ_BASE + 0x10000, OBJ_BASE + 0x14000
e.write(0x702650, F(0, 0, 0))
rows = []
for case in range(300):
    e.write(THIS, bytes(0x1800)); e.write(OUT, bytes(0x40))
    kind = r.choice([0, 1, 2])
    n = r.choice([0, 1, 3, 8]); i = r.choice([0, 1, 2, 7, 9])
    w = {0x140c: U(n), 0x1410: U(i)}
    for k in range(8 * 7): w[0xf94 + 4 * k] = U(r.randrange(2 ** 32))
    for o, b in w.items(): e.write(THIS + o, b)
    for k in range(7): e.write(OUT + 4 * k, U(r.randrange(2 ** 32)))
    idx = r.choice([-1, 0, 1, 2, 3, 7, 9])
    out0 = e.read(OUT, 0x1c)
    before = e.read(THIS, 0x1800)
    if kind == 0:
        e.call(0x58d1f0, [OUT], 'i', this=THIS)
    elif kind == 1:
        e.call(0x58d250, [OUT, idx], 'ii', this=THIS)
    else:
        e.write(OUT, bytes(r.randrange(256) for _ in range(0x1c)))
        out0 = e.read(OUT, 0x1c)
        e.call(0x58b4f0, [], '', this=OUT)
    res = e.eax & 0xffffffff
    after = e.read(THIS, 0x1800)
    diff = ','.join('%x:%s' % (j, after[j:j + 4].hex()) for j in range(0, 0x1800, 4) if after[j:j + 4] != before[j:j + 4])
    rows.append(chr(9).join([str(kind), ','.join('%x:%s' % (o, b.hex()) for o, b in w.items()), str(idx), out0.hex(), str(res), diff, e.read(OUT, 0x1c).hex()]))
pathlib.Path(__file__).with_name('golden_record_leaves.tsv').write_text(chr(10).join(rows) + chr(10))
print(len(rows))

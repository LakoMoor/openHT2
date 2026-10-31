"""0x580ce0 (MovingItem virtual +0x88: integrate +0x1a0 by dt * the force at +0x314, products at +0xc4c/+0xc58) run
unmodified; the range report 0x4e2821 is a recording stub. dt (0x4e2749) is set via 0x6974c2 / clock +0xb90.
Row: input words (sparse), step ms word, dt double (hex), changed dwords (area 0x1000), report."""
import pathlib, random, struct
from emu import Emu, OBJ_BASE

r = random.Random(0x580ce0)
e = Emu()
OBJ, CLK = OBJ_BASE + 0x10000, OBJ_BASE + 0x30000
F = lambda *v: struct.pack('<%df' % len(v), *v)
rep = []
e.hook_ret(0x4e2821, lambda emu: (rep.append('%s|%08x' % (e.read(emu.arg(0), 12).hex(), emu.arg(2))), 0)[1])
e.set_u32(0x6974c4, CLK)
val = lambda: r.choice([0.0, 1.0, -1.0, 0.5, 2.0, r.uniform(-8, 8), r.uniform(-1, 1), r.uniform(-300, 300)])
IN = [0x1a0, 0x1a4, 0x1a8, 0x1ac, 0x1b0, 0x1b4, 0x314, 0x318, 0x31c]
rows = []
for case in range(300):
    rep.clear()
    e.write(OBJ, bytes(0x1000))
    inp = {o: F(val()) for o in IN}
    for o, b in inp.items():
        e.write(OBJ + o, b)
    step = r.choice([0, 0, 0, 16, 33, -5, 1000])
    dt = r.choice([0.0, 0.016, 0.0333, 0.25, 0.3, r.uniform(0, 0.4), 1 / 60])
    e.write(0x6974c2, struct.pack('<h', step))
    e.write(CLK + 0xb90, struct.pack('<d', dt))
    before = e.read(OBJ, 0x1000)
    e.call(0x580ce0, [], '', this=OBJ)
    after = e.read(OBJ, 0x1000)
    diff = ','.join('%x:%s' % (i, after[i:i + 4].hex()) for i in range(0, 0x1000, 4) if after[i:i + 4] != before[i:i + 4])
    rows.append(chr(9).join([','.join('%x:%s' % (o, b.hex()) for o, b in inp.items()), str(step), struct.pack('<d', dt).hex(), diff, rep[0] if rep else '']))
pathlib.Path(__file__).with_name('golden_moving_item_step.tsv').write_text(chr(10).join(rows) + chr(10))
print(len(rows), 'cases')

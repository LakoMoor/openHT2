"""0x581230 (MovingItem place: clamp of the z at arg2+0x2c, virtuals +0x90/+0x94/+0x88/+0x3c) run unmodified; the four
virtuals are recording stubs. Row: arg2 image (0x40 bytes hex), events `;`-separated (`slot|this|args`), final arg2 dword."""
import pathlib, random, struct
from emu import Emu, OBJ_BASE, STUB

r = random.Random(0x581230)
e = Emu()
B, VT, A1, A2 = OBJ_BASE + 0x10000, OBJ_BASE + 0x11000, OBJ_BASE + 0x12000, OBJ_BASE + 0x13000
ev = []
for i, (slot, n) in enumerate(((0x90, 2), (0x94, 2), (0x88, 2), (0x3c, 0))):
    addr = STUB + 0x300 + 0x10 * i
    e.set_u32(VT + slot, addr)
    e.hook_thiscall(addr, (lambda s, k: lambda emu: (ev.append('%x|%x|%s' % (s, emu.ecx, ','.join('%x' % emu.arg(j) for j in range(k)))), 0)[1])(slot, n), n)
e.set_u32(B, VT)
rows = []
for case in range(200):
    ev.clear()
    f = lambda: r.choice([-5000.0, -2000.0, -1999.9, -2000.1, 0.0, float('nan'), r.uniform(-3000, 100), r.uniform(-4, 4)])
    img = struct.pack('<16f', *[r.uniform(-9, 9) for _ in range(16)])
    img = img[:0x2c] + struct.pack('<f', f()) + img[0x30:]
    e.write(A2, img)
    e.call(0x581230, [A1, A2], 'ii', this=B)
    rows.append(chr(9).join([img.hex(), ';'.join(ev), e.read(A2 + 0x2c, 4).hex()]))
pathlib.Path(__file__).with_name('golden_moving_item_place.tsv').write_text(chr(10).join(rows) + chr(10))
print(len(rows), 'cases; clamped', sum(1 for x in rows if x.split(chr(9))[1].startswith('90')))

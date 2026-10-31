"""0x580f80 (MovingItem virtual: set the pose, thiscall (matrix, flag)) run unmodified; the basis check 0x58e7f0 (log only),
and the virtuals [[this+0x20]]+0x24, [this]+0x28 are recording stubs; 0x5f2190 runs unmodified. Row: byte +0x46c, matrix hex,
flag, changed dwords of object (0x1000) and node (0x100 at AUX+0x44 target), events."""
import pathlib, random, struct
from emu import Emu, OBJ_BASE, STUB

r = random.Random(0x580f80)
e = Emu()
OBJ, AUX, VT, VT2, NODE, MAT = [OBJ_BASE + 0x10000 + 0x2000 * i for i in range(6)]
ev = []
e.set_u32(OBJ + 0x20, AUX)
e.set_u32(AUX, VT2)
e.set_u32(AUX + 0x44, NODE)
e.set_u32(OBJ, VT)
A24, A28 = STUB + 0x400, STUB + 0x410
e.set_u32(VT2 + 0x24, A24)
e.set_u32(VT + 0x28, A28)
e.hook_thiscall(A24, lambda emu: (ev.append('24|%x' % emu.ecx), 0)[1], 0)
e.hook_thiscall(A28, lambda emu: (ev.append('28|%x|%x|%x' % (emu.ecx, emu.arg(0), emu.arg(1))), 0)[1], 2)
e.hook_thiscall(0x58e7f0, lambda emu: (ev.append('chk|%x|%x|%x' % (emu.ecx, emu.arg(0), emu.arg(2))), 0)[1], 3)
rows = []
for case in range(200):
    ev.clear()
    e.write(OBJ, bytes(0x1000)); e.set_u32(OBJ, VT); e.set_u32(OBJ + 0x20, AUX)
    e.write(NODE, bytes(0x100))
    flag = r.choice([0, 1, 1])
    e.write(OBJ + 0x46c, bytes([flag]))
    mat = struct.pack('<12f', *[r.uniform(-2, 2) for _ in range(12)])
    e.write(MAT, mat)
    a2 = r.randrange(0, 2 ** 32)
    before, nb = e.read(OBJ, 0x1000), e.read(NODE, 0x100)
    e.call(0x580f80, [MAT, a2], 'ii', this=OBJ)
    after, na = e.read(OBJ, 0x1000), e.read(NODE, 0x100)
    d = lambda a, b, n: ','.join('%x:%s' % (i, b[i:i + 4].hex()) for i in range(0, n, 4) if a[i:i + 4] != b[i:i + 4])
    rows.append(chr(9).join([str(flag), mat.hex(), '%x' % a2, d(before, after, 0x1000), d(nb, na, 0x100), ';'.join(ev)]))
pathlib.Path(__file__).with_name('golden_moving_item_pose.tsv').write_text(chr(10).join(rows) + chr(10))
print(len(rows)); print(rows[0][:400])

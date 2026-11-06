"""Mesh query helpers: 0x56f650 (cdecl (idx, o1..o6): reads the collected node idx from the array 0x6f2e58, writes six outputs),
0x56f3f0 ((room, a, b, c, d, count*): stores the box in 0x6f3248.., clears 0x6f34c0, calls [room->vtable+0x20](0, 0x56f450), count out) and
0x5818b0 (thiscall (flags): +0x37c |= flags, +0x388 = 0x4e27a5()). The room traversal is a recording stub that sets the counter.
Row kind first: 'acc' idx kind...; 'beg' ...; 'flg' ..."""
import pathlib, random, struct
from emu import Emu, OBJ_BASE, STUB

r = random.Random(0x56f650)
e = Emu()
OBJ = OBJ_BASE + 0x10000
OUT = OBJ_BASE + 0x20000
ROOM, VT, CLK = OBJ_BASE + 0x30000, OBJ_BASE + 0x31000, OBJ_BASE + 0x32000
F = lambda *v: struct.pack('<%df' % len(v), *v)
U = lambda *v: struct.pack('<%dI' % len(v), *v)
rows = []
# --- accessor 0x56f650
for case in range(120):
    idx = r.randrange(0, 30)
    node = OBJ + 0x400 * (case % 8)
    e.write(node, bytes(r.randrange(256) for _ in range(0x60)))
    t10 = r.choice([9, 9, 9, 3]); t40 = r.choice([7, 7, 7, 2])
    e.set_u32(node + 0x10, t10); e.set_u32(node + 0x40, t40)
    e.set_u32(0x6f2e58 + 4 * idx, node)
    e.set_u32(0x6f326c + 4 * idx, r.randrange(2 ** 32))
    e.write(OUT, bytes(0x60))
    outs = [OUT + 0x10 * k for k in range(6)]
    e.call(0x56f650, [idx] + outs, 'i' * 7)
    rows.append(chr(9).join(['acc', str(idx), '%x' % t10, '%x' % t40, e.read(node, 0x60).hex(), '%x' % e.read(0x6f326c + 4 * idx, 4)[0:4].hex().__len__() if False else e.read(0x6f326c + 4 * idx, 4).hex(), '%x' % e.eax, e.read(OUT, 0x60).hex()]))
# --- begin 0x56f3f0
V20 = STUB + 0x600
e.set_u32(VT + 0x20, V20)
e.set_u32(ROOM, VT)
seen = []
e.hook_thiscall(V20, lambda emu: (seen.append((emu.ecx, emu.arg(0), emu.arg(1), e.read(0x6f3248, 16).hex(), e.read(0x6f34c0, 4).hex())), e.set_u32(0x6f34c0, 7), 0)[2], 2)
for case in range(60):
    seen.clear()
    e.set_u32(0x6f34c0, r.randrange(100))
    box = F(*[r.uniform(-9, 9) for _ in range(4)])
    e.write(OUT, bytes(8))
    e.call(0x56f3f0, [ROOM] + list(struct.unpack('<4I', box)) + [OUT], 'i' * 6)
    rows.append(chr(9).join(['beg', box.hex(), '%x' % e.eax, e.read(OUT, 4).hex(), e.read(0x6f3248, 16).hex(), e.read(0x6f34c0, 4).hex(), ';'.join('%x,%x,%x,%s,%s' % s for s in seen)]))
# --- flags 0x5818b0 (uses 0x4e27a5: step word 0x6974c2 / 0x6974bc / clock +0xb98)
e.set_u32(0x6974c4, CLK)
for case in range(60):
    e.write(0x6974c2, struct.pack('<h', r.choice([0, 0, 5])))
    e.write(0x6974bc, F(r.uniform(0, 1)))
    e.write(CLK + 0xb98, struct.pack('<d', r.uniform(0, 1)))
    e.write(OBJ, bytes(0x400))
    e.set_u32(OBJ + 0x37c, r.randrange(2 ** 32))
    fl = r.randrange(2 ** 32)
    pre = e.read(OBJ + 0x37c, 4)
    e.call(0x5818b0, [fl], 'i', this=OBJ)
    rows.append(chr(9).join(['flg', '%x' % fl, pre.hex(), e.read(OBJ + 0x37c, 4).hex(), e.read(OBJ + 0x388, 4).hex(), e.read(0x6974c2, 2).hex(), e.read(0x6974bc, 4).hex(), e.read(CLK + 0xb98, 8).hex()]))
pathlib.Path(__file__).with_name('golden_mesh_query.tsv').write_text(chr(10).join(rows) + chr(10))
print(len(rows))


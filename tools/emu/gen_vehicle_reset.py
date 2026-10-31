"""0x580dd0 (vehicle object reset, ecx = object) run unmodified; the virtuals [vtable+0x4c]/[+0xcc], 0x5d32f0, 0x5f25a0
(writes a matrix) and the log 0x4ea27e are recording stubs. Row: constant vector 0x702650 (hex), object dwords
`off:hex`, aux image at OBJ+0x20000 (0x400 bytes), events `;`-separated (`v|slot|this`, `c|addr|this|matrix`,
`l|text`), final dwords, world word [0x6ef1fc]+0xbb0 (hex), debug flag."""
import pathlib, random, struct
from emu import Emu, OBJ_BASE, STUB

r = random.Random(0x580dd0)
e = Emu()
e.install_crt_hooks()
B, AUX, VT, WREC = OBJ_BASE + 0x10000, OBJ_BASE + 0x20000, OBJ_BASE + 0x21000, OBJ_BASE + 0x30000
ev = []
V4C, VCC = STUB + 0x200, STUB + 0x210
e.set_u32(VT + 0x4c, V4C)
e.set_u32(VT + 0xcc, VCC)
e.hook_thiscall(V4C, lambda emu: (ev.append('v|4c|%x' % emu.ecx), 0)[1], 0)
e.hook_thiscall(VCC, lambda emu: (ev.append('v|cc|%x' % emu.ecx), 0)[1], 0)
e.hook_thiscall(0x5d32f0, lambda emu: (ev.append('c|5d32f0|%x|' % emu.ecx), 0)[1], 1)


def mat(emu):
    q = emu.arg(0)
    data = struct.pack('<13f', *[r.uniform(-4, 4) for _ in range(13)])
    e.write(q, data)
    ev.append('c|5f25a0|%x|%s' % (emu.ecx, data.hex()))
    return 0


e.hook_thiscall(0x5f25a0, mat, 1)
e.hook_thiscall(0x4ea27e, lambda emu: (ev.append('l|' + bytes(e.read(emu.arg(0), 200)).split(b'\0')[0].decode('latin1')), 0)[1], 1)
e.write(0x7025f0, struct.pack('<12f', 1, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0))
FIN = (0x4a8, 0x37c, 0x4b0, 0x32c, 0x1dc, 0x1e0, 0x1e4, 0x1e8, 0x1ec, 0x1f0)
rows = []
for case in range(300):
    ev.clear()
    e.write(B, bytes(0x600))
    e.write(AUX, bytes(0x400))
    e.write(WREC, bytes(0xc00))
    vec = struct.pack('<3f', *[r.choice([0.0, 1.5, -2.25, r.uniform(-3, 3)]) for _ in range(3)])
    e.write(0x702650, vec)
    e.set_u32(0x6ef1fc, WREC)
    e.set_u32(WREC + 0xbb0, r.randrange(0, 2 ** 32))
    flag = r.choice([0, 1, 1])
    e.set_u32(0x696c6c, flag)
    vals = {0x4a8: r.choice([0, 1]), 0x37c: r.choice([0, 1]), 0x174: r.choice([0, 0, 1]), 0x4b0: r.choice([0, 1]),
            0x32c: r.randrange(0, 2 ** 32), 0x20: AUX, 0x140: AUX + 0x80}
    for o in (0x218, 0x21c, 0x220):
        vals[o] = struct.unpack('<I', struct.pack('<f', r.uniform(-9, 9)))[0]
    for o in (0x1dc, 0x1e0, 0x1e4, 0x1e8, 0x1ec, 0x1f0):
        vals[o] = r.randrange(0, 2 ** 32)
    for o, v in vals.items():
        e.set_u32(B + o, v)
    e.set_u32(B, VT)
    e.set_u32(AUX + 0x44, AUX + 0x300)         # [[b+0x20]+0x44]
    e.set_u32(AUX + 0x80 + 0xc, AUX + 0x200)   # [[b+0x140]+0xc] = name pointer
    e.write(AUX + 0x200, r.choice([b'truck', b'KAMAZ_1', b'x']) + b'\0')
    wrec = e.read(WREC + 0xbb0, 4)
    aux = e.read(AUX, 0x400)
    e.call(0x580dd0, [], '', this=B)
    fin = ','.join('%x:%x' % (o, struct.unpack('<I', e.read(B + o, 4))[0]) for o in FIN)
    inp = ','.join('%x:%x' % (o, v) for o, v in vals.items())
    rows.append(chr(9).join([vec.hex(), inp, aux.hex(), ';'.join(ev), fin, wrec.hex(), '%x' % flag]))
pathlib.Path(__file__).with_name('golden_vehicle_reset.tsv').write_text(chr(10).join(rows) + chr(10))
print(len(rows), 'cases; with log', sum(1 for x in rows if '|l|' in x or ';l|' in x or x.split(chr(9))[3].startswith('l|')))

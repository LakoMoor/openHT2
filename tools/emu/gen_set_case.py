"""0x4e480c (SetCaseSwitch(obj, index), cdecl) run unmodified; the virtual +0x3c (case count), the assert 0x4e460b and sprintf are recording stubs.
Row: kind, index, count, [obj+0x38] before, [obj+0x38] after, events."""
import pathlib, random
from emu import Emu, OBJ_BASE, STUB

r = random.Random(0x4e480c)
e = Emu()
OBJ, VT = OBJ_BASE + 0x10000, OBJ_BASE + 0x12000
V3C = STUB + 0x600
ev, st = [], {}
e.hook_thiscall(V3C, lambda emu: (ev.append('count'), st['count'])[1], 0)
e.hook_ret(0x4e460b, lambda emu: (ev.append('assert'), 0)[1])
e.hook_ret(0x61cca9, lambda emu: 0)
e.set_u32(VT + 0x3c, V3C)
rows = []
for case in range(300):
    ev.clear()
    e.write(OBJ, bytes(0x100)); e.set_u32(OBJ, VT)
    kind = r.choice([0x12, 0x12, 0x12, 0x11, 6])
    st['count'] = r.choice([0, 1, 3, 5])
    idx = r.choice([-1, 0, 1, 2, 4, 5, 9])
    before = r.randrange(0, 10)
    e.set_u32(OBJ + 0x10, kind); e.set_u32(OBJ + 0x38, before)
    e.call(0x4e480c, [OBJ, idx & 0xffffffff], 'ii')
    rows.append(chr(9).join([str(kind), str(idx), str(st['count']), str(before), str(int.from_bytes(e.read(OBJ + 0x38, 4), "little")), ';'.join(ev)]))
pathlib.Path(__file__).with_name('golden_set_case.tsv').write_text(chr(10).join(rows) + chr(10))
print(len(rows), rows[0], rows[5])

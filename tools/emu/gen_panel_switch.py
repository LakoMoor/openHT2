"""Validate actual 0x5e1a10/0x4e48a2 writes and 0x5e1a40 reads.

Only the virtual case-count query is supplied by the fixture.
"""
import pathlib, random
from emu import Emu, OBJ_BASE

STUB_BASE = 0x1f000000

e = Emu()
node, vtable = OBJ_BASE, OBJ_BASE + 0x1000
count = 0
e.set_u32(node, vtable)
e.set_u32(vtable + 0x3c, STUB_BASE + 0x700)
e.hook_thiscall(STUB_BASE + 0x700, lambda q: count, 0)
r = random.Random(0x5e1a10)
rows = []
for case in range(400):
    kind = r.choice([0, 0x12, 0x12, 0x12, 0x15, 0xffffffff])
    count = r.choice([-1, 0, 1, 2, 10, 19, 0x7fffffff])
    key = r.choice([-0x80000000, -1, 0, 1, 9, count - 1, count, 0x7fffffff])
    old = r.randrange(0x100000000)
    ptr = 0 if case % 13 == 0 else node
    results = []
    for address in [0x5e1a10, 0x4e48a2]:
        e.set_u32(node + 0x10, kind)
        e.set_u32(node + 0x38, old)
        e.call(address, [ptr, key], 'ii')
        result = e.read(node + 0x38, 4)
        results.append(int.from_bytes(result, 'little'))
        e.call(0x5e1a40, [ptr], 'i')
        results.append(e.eax)
    rows.append('\t'.join(map(str, [int(ptr != 0), kind, count, key, old] + results)))
pathlib.Path(__file__).with_name('golden_panel_switch.tsv').write_text('\n'.join(rows) + '\n')
print('400 switch states, both setters and getter captured')

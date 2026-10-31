"""List lookups run unmodified: 0x57c9e0 (object by id, list head [0x6f653c], items compared as item-0x178),
0x53f780 (list head [0x6ef1f8], item compared as p-0x10; null argument goes through 0x53f750(0))."""
import pathlib, random, struct
from emu import Emu, OBJ_BASE

r = random.Random(0x57c9e0)
e = Emu()
rows = []
BASE = OBJ_BASE + 0x40000
for case in range(300):
    which = r.choice(['57c9e0', '53f780'])
    head_g = 0x6f653c if which == '57c9e0' else 0x6ef1f8
    n = r.choice([0, 1, 2, 3, 5])
    elems = [BASE + 0x1000 * (i + 1) + r.choice([0, 0x10, 0x200]) for i in range(n)]
    for i, a in enumerate(elems):
        e.write(a, bytes(0x20))
        e.set_u32(a + 8, elems[i + 1] + 4 if i + 1 < n else 0)
    e.set_u32(head_g, elems[0] + 4 if n else 0)
    off = 0x178 if which == '57c9e0' else -0x10
    pick = r.random()
    if n and pick < 0.6:
        arg = elems[r.randrange(n)] - off if which == '57c9e0' else elems[r.randrange(n)] + 0x10
    elif pick < 0.7:
        arg = 0
    else:
        arg = r.randrange(1, 2 ** 32)
    if which == '53f780' and n and r.random() < 0.15:
        e.set_u32(head_g, 4)   # head link whose element would be null
    res = e.call(int(which, 16) if False else (0x57c9e0 if which == '57c9e0' else 0x53f780), [arg], 'i')
    rows.append(chr(9).join([which, str(n), ','.join('%x' % a for a in elems), '%x' % struct.unpack('<I', e.read(head_g, 4))[0], '%x' % arg, '%x' % e.eax]))
pathlib.Path(__file__).with_name('golden_world_lookup.tsv').write_text(chr(10).join(rows) + chr(10))
print(len(rows), 'cases;', sum(1 for x in rows if not x.endswith(chr(9) + '0')), 'found')

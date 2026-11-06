"""0x56f450 (collector callback of the road-mesh query, cdecl (node)) run unmodified (including 0x5f25c0, strncmp and 0x5ab4d0);
the traversal virtual [child->vtable+0x20](0, cb) and the count virtual [node->vtable+0x3c] are recording stubs.
Row: node words, shape owner words, parent-chain node words, name, bounds float, centre, box (16), counters before, result,
list/extra/count after, events."""
import pathlib, random, struct
from emu import Emu, OBJ_BASE, STUB

r = random.Random(0x56f450)
e = Emu()
F = lambda *v: struct.pack('<%df' % len(v), *v)
NODE, OWN, CHAIN, CHAIN2, NAME, BND, CEN, CHILD, VT, VTC = [OBJ_BASE + 0x10000 + 0x400 * i for i in range(10)]
e.write(0x7025f0, F(1, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0))
ev, cnt = [], {'n': 0}
V20, V3C = STUB + 0x700, STUB + 0x710
e.set_u32(VT + 0x20, V20); e.set_u32(VTC + 0x3c, V3C)
e.hook_thiscall(V20, lambda emu: (ev.append('t|%x|%x|%x' % (emu.ecx, emu.arg(0), e.read(0x6f34c4, 4)[0] | 0)), 0)[1], 2)
e.hook_thiscall(V3C, lambda emu: (ev.append('c|%x' % emu.ecx), cnt['n'])[1], 0)
rows = []
for case in range(400):
    ev.clear(); cnt['n'] = r.randrange(0, 6)
    for a in (NODE, OWN, CHAIN, CHAIN2, CHILD):
        e.write(a, bytes(0x100))
    ty = r.choice([9, 9, 9, 9, 9, 9, 9, 0x11, 0x12, 5])
    flags = r.choice([0, 0, 0, 0, 0, 1, 8, 9])
    nd = {0x10: ty, 0x28: flags, 0x44: r.randrange(2 ** 32)}
    name = r.choice([b'sharp', b'sharp_wall', b'sharpX', b'sharp1', b'sharp2', b'shar', b'road', b''])
    e.write(NAME, name + b'\0')
    nd[0xc] = NAME if r.random() > 0.05 else 0
    nd[0x40] = r.choice([7, 7, 7, 3]) if ty == 9 else CHILD
    nd[0x2c] = OWN
    nd[0x3c] = BND if ty != 0x12 else r.choice([0, CHAIN])
    nd[0x38] = r.randrange(0, 5)
    nd[0x48] = CEN
    nd[0] = VTC if ty == 0x12 else 0
    e.write(CHILD, struct.pack('<I', VT))
    for o, v in nd.items():
        e.set_u32(NODE + o, v)
    owner_ty = r.choice([4, 4, 4, 2])
    own = {0x10: owner_ty, 0x3c: r.choice([0, CHAIN2, CHAIN2]), 0x2c: 0, 0x38: r.choice([0, 1, 2]), 0x40: 0}
    for o, v in own.items():
        e.set_u32(OWN + o, v)
    e.write(OWN + 0x40, F(*[r.uniform(-2, 2) for _ in range(12)]))
    e.set_u32(CHAIN2 + 0x38, r.choice([0, 1, 2])); e.write(CHAIN2 + 0x40, F(*[r.uniform(-2, 2) for _ in range(12)])); e.set_u32(CHAIN2 + 0x2c, 0)
    e.set_u32(CHAIN + 0x38, 3)
    e.write(BND, F(r.uniform(0, 4))); e.write(CEN, F(*[r.uniform(-5, 5) for _ in range(3)]))
    box = F(*[r.uniform(-6, 6) for _ in range(3)], r.choice([r.uniform(0, 5), r.uniform(5, 40)]))
    e.write(0x6f3248, box)
    count0 = r.choice([0, 3, 7, 29, 30])
    cache = r.choice([0, 0, CHAIN2])
    e.set_u32(0x6f34c0, count0); e.set_u32(0x6f34c4, cache)
    e.write(0x6f2e58, bytes(0x100)); e.write(0x6f326c, bytes(0x100))
    e.call(0x56f450, [NODE], 'i')
    words = lambda a, n: e.read(a, 4 * n).hex()
    rows.append(chr(9).join([
        ','.join('%x:%08x' % (o, v) for o, v in nd.items()), ','.join('%x:%08x' % (o, v) for o, v in own.items()),
        e.read(OWN + 0x40, 48).hex(), '%x' % struct.unpack('<I', e.read(CHAIN2 + 0x38, 4))[0], e.read(CHAIN2 + 0x40, 48).hex(),
        name.hex(), e.read(BND, 4).hex(), e.read(CEN, 12).hex(), box.hex(), '%d' % count0, '%x' % cache, str(cnt['n']),
        '%x' % e.eax, words(0x6f2e58, 8), words(0x6f326c, 8), words(0x6f34c0, 1), words(0x6f34c4, 1), ';'.join(ev)]))
pathlib.Path(__file__).with_name('golden_mesh_collect.tsv').write_text(chr(10).join(rows) + chr(10))
from collections import Counter
print(len(rows), Counter(x.split(chr(9))[12] for x in rows).most_common(5), sum(1 for x in rows if int(x.split(chr(9))[15][:2] + x.split(chr(9))[15][2:], 16) is not None))


"""0x54cb90 (the ray probe of the vehicle scene, ecx = scene) run unmodified. External owners are stubs: the virtual
ray cast [owner->vtable+0x34] (writes the hit data into the query, records the query it saw) and 0x4e480c
(SetCaseSwitch, records object and index). The name table 0x71e820 and the constant vectors are real memory.
Row: scene dwords `off:hex`, name table image (sparse), cast record `kind|query hex (0x70 bytes)|written hex|ret`,
set_case calls `obj:idx,...`."""
import pathlib, random, struct
from emu import Emu, OBJ_BASE, STUB

r = random.Random(0x54cb90)
e = Emu()
SCENE, OWN, VT, TABLE, NODES, STR, HITS = (OBJ_BASE + 0x10000, OBJ_BASE + 0x20000, OBJ_BASE + 0x21000, OBJ_BASE + 0x40000,
                                           OBJ_BASE + 0x42000, OBJ_BASE + 0x60000, OBJ_BASE + 0x70000)
CAST = STUB + 0x100
rec = {}
cases = []


def fb(*v):
    return struct.pack('<%df' % len(v), *v)


# constant vectors and the identity transform (set up by the original's static initialisers)
e.write(0x702620, fb(0, 1, 0)); e.write(0x702630, fb(0, 0, 1)); e.write(0x702640, fb(1, 0, 0)); e.write(0x702650, fb(0, 0, 0))
e.write(0x7025f0, fb(1, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0))


def cast(emu):
    kind, q = emu.arg(0), emu.arg(1)
    seen = e.read(q, 0x70)
    wr = bytearray()
    if r.random() < 0.5:
        mat = [1, 0, 0, 0, 1, 0, 0, 0, 1, r.uniform(-3, 3), r.uniform(-3, 3), r.uniform(-3, 3)]
        mat = [v + (r.uniform(-0.2, 0.2) if i < 9 else 0) for i, v in enumerate(mat)]
    else:
        mat = None
    if mat:
        wr += fb(*mat)
        e.write(q + 4, fb(*mat))
    pt = fb(r.uniform(-5, 5), r.uniform(-5, 5), r.uniform(-5, 5))
    ax = r.choice([(0, 1, 0), (0, -1, 0), (1, 0, 0), (-1, 0, 0), (0, 0, 1), (0.6, 0.6, 0.5), (0, 0, 0)])
    nrm = fb(*ax)
    e.write(q + 0x34, pt + nrm)
    hit = HITS + 0x40 * r.randrange(0, 4) if r.random() < 0.85 else 0
    e.set_u32(q + 0x64, hit)
    ret = r.choice([1, 1, 1, 0, 0x101, 2])
    rec['cast'] = '%x|%s|%s|%s|%x|%x' % (kind, seen.hex(), (fb(*mat).hex() if mat else ''), (pt + nrm).hex(), hit, ret)
    return ret


e.set_u32(OWN, VT)
e.set_u32(VT + 0x34, CAST)
e.hook_thiscall(CAST, cast, 2)
setcase = []
e.hook_ret(0x4e480c, lambda emu: (setcase.append('%x:%d' % (emu.arg(0), struct.unpack('<i', struct.pack('<I', emu.arg(1)))[0])), 0)[1])

SC_OFFS = [0xce4, 0xd88, 0xd8c, 0xd90] + list(range(0x204, 0x264, 4))
IDS = [5, 17, 123, 999, 0, 42]


def bucket(s):
    return sum(s.encode()) % 500


rows = []
for case in range(500):
    rec.clear(); setcase.clear()
    e.write(SCENE, bytes(0x2c00)); e.write(TABLE, bytes(0x2000)); e.write(NODES, bytes(0x4000)); e.write(STR, bytes(0x4000))
    e.write(HITS, bytes(0x400))
    e.set_u32(0x71e828, TABLE)
    vals = {}
    for o in SC_OFFS:
        vals[o] = struct.unpack('<I', fb(r.choice([0.0, 1.0, -1.0, r.uniform(-2, 2), r.uniform(-1, 1)]))[:4])[0]
    vals[0xce4] = r.randrange(0, 2 ** 32)
    own = OWN if r.random() < 0.9 else 0
    vals[0x2aa8] = own
    vals[0x2aac] = r.choice([0, 1, 1, 2])
    for o, v in vals.items():
        e.set_u32(SCENE + o, v)
    # hits: [hit] = 0x4d or other, [hit+4] = id
    for i in range(4):
        e.set_u32(HITS + 0x40 * i, r.choice([0x4d, 0x4d, 0x4d, 0x10]))
        e.set_u32(HITS + 0x40 * i + 4, r.choice(IDS))
    # name table: K_HFLAG_nnn and K_FLAG_nnn for a random subset of ids
    tail = {}
    k = 0
    for id_ in IDS:
        for fmt in ('K_HFLAG_%03d', 'K_FLAG_%03d'):
            if r.random() < 0.8:
                nm = fmt % id_
                node, sp = NODES + 0x20 * k, STR + 0x40 * k
                e.write(sp, nm.encode() + b'\0')
                e.set_u32(node + 0xc, sp)
                e.set_u32(node + 0x10, 0x1000 + 0x10 * k + 1)
                b = bucket(nm)
                if b in tail:
                    e.set_u32(tail[b] + 8, node)
                else:
                    e.set_u32(TABLE + 4 * b, node)
                tail[b] = node
                k += 1
    image = e.read(TABLE, 0x2000) + e.read(NODES, 0x20 * k) + e.read(STR, 0x40 * k)
    hits = e.read(HITS, 0x100)
    e.call(0x54cb90, [], '', this=SCENE)
    rows.append(chr(9).join([','.join('%x:%x' % (o, v) for o, v in vals.items()), image.hex(), str(k), hits.hex(),
                             rec.get('cast', ''), ','.join(setcase)]))
pathlib.Path(__file__).with_name('golden_scene_probe.tsv').write_text(chr(10).join(rows) + chr(10))
cnt = {}
for row in rows:
    for c in row.split(chr(9))[5].split(','):
        if c:
            cnt[c.split(':')[1]] = cnt.get(c.split(':')[1], 0) + 1
print(len(rows), 'cases; casts:', sum(1 for x in rows if x.split(chr(9))[4]), 'set_case by index', cnt)

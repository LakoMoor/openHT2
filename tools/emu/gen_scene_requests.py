"""0x54d0b0 (the sound/effect request slots of the vehicle scene, ecx = scene) run unmodified; the network message
0x546360 is a recording stub (just counts the calls). 0x57c9e0, 0x5ab4d0 and 0x4e3132 are the real ones.
Row: input dwords of the scene `off:hex`, of the network object `off:hex`, list elements, calls of 0x546360,
output dwords 0x5468..0x5538 of the scene."""
import pathlib, random, struct
from emu import Emu, OBJ_BASE

r = random.Random(0x54d0b0)
e = Emu()
SCENE, NET, LIST = OBJ_BASE + 0x10000, OBJ_BASE + 0x20000, OBJ_BASE + 0x30000
calls = []
e.hook_thiscall(0x546360, lambda emu: (calls.append(1), 0)[1], 0)

F_OFFS = [0x19c, 0x1a4, 0x1a8, 0x1ac, 0x1b0, 0x1b4, 0x1b8, 0x1bc, 0x1c0, 0x1c4, 0x1d4, 0x1d8, 0x1dc, 0x1e0, 0x1e4, 0x1e8,
          0x204, 0x208, 0x20c, 0x210, 0x214, 0x218, 0x21c, 0x220, 0x224, 0x228, 0x22c, 0x230,
          0x264, 0x268, 0x26c, 0x270, 0x274, 0x278, 0x27c, 0x280, 0x284, 0x288, 0x28c, 0x290,
          0x294, 0x298, 0x29c, 0x2a0, 0x2a4, 0x2a8, 0x2ac, 0x2b0, 0x2b4,
          0x388, 0x3d4, 0x3d8, 0x3dc, 0x3e0, 0x3e4, 0x3e8, 0x598]
I_OFFS = [0x38c, 0x414, 0x594, 0x5460, 0x28bc]
OUT = list(range(0x5468, 0x5540, 4))


def fl(v):
    return struct.pack('<f', v).hex()


def rv():
    return r.choice([0.0, 1.0, -1.0, 0.5, -0.5, 2.0, 3.0, 0.1, 0.25, 4.0, 10.0, -3.0, r.uniform(-5, 5), r.uniform(-1, 1)])


rows = []
for case in range(600):
    calls.clear()
    e.write(SCENE, bytes(0x5600)); e.write(NET, bytes(0x3c00)); e.write(LIST, bytes(0x400))
    n = r.choice([0, 1, 2])
    elems = [LIST + 0x100 * (i + 1) for i in range(n)]
    for i, a in enumerate(elems):
        e.set_u32(a + 8, elems[i + 1] + 4 if i + 1 < n else 0)
    e.set_u32(0x6f653c, elems[0] + 4 if n else 0)
    kind = r.choice(['plain', 'plain', 'clean'])
    sc = {}
    for o in F_OFFS:
        sc[o] = fl(rv())
    # make the geometry tame so that the level and the direction tests are both reached
    for o in (0x1a4, 0x1a8, 0x1ac):
        sc[o] = fl(r.choice([1.0, 2.0, 0.5, -1.0, r.uniform(0.2, 3)]))
    if r.random() < 0.6:
        sc[0x388] = fl(r.choice([0.0, 1.0, 10.0, 100.0]))
    if r.random() < 0.5:
        for o in (0x1b0, 0x1b4, 0x1b8, 0x1d4, 0x1d8, 0x1dc):
            sc[o] = fl(r.uniform(-1, 1) * r.choice([0.01, 0.1, 1, 5]))
    sc[0x598] = fl(r.choice([0.0, 0.3, 1.0, 1.5, 7.0]))
    ints = {0x38c: r.choice([0, 1, 1, 1]), 0x594: r.choice([0, 1, 2, 4, 5, 6, 6, 3, 1]),
            0x5460: NET if r.random() < 0.93 else 0, 0x28bc: r.choice([0, 1, 2, 4, 4, 6, 8])}
    pick = r.random()
    ints[0x414] = elems[r.randrange(n)] - 0x178 if n and pick < 0.5 else (0 if pick < 0.6 else r.randrange(1, 2 ** 32))
    for o in OUT:
        ints[o] = r.choice([0, 0x3f800000, r.randrange(0, 2 ** 32)])
    net = {0x34ac: r.choice([1, 1, 1, 0, 2]), 0x3344: r.choice([0, 0, 0, 1])}
    for i in range(8):
        net[0x3be8 + 4 * i] = r.choice([0, 1, 5, -3 & 0xffffffff, 100, 20])
    for o, v in sc.items():
        e.write(SCENE + o, bytes.fromhex(v))
    for o, v in ints.items():
        e.set_u32(SCENE + o, v)
    for o, v in net.items():
        e.set_u32(NET + o, v)
    e.call(0x54d0b0, [], '', this=SCENE)
    out = ','.join('%x:%x' % (o, struct.unpack('<I', e.read(SCENE + o, 4))[0]) for o in OUT)
    insc = ','.join('%x:%s' % (o, v) for o, v in sc.items()) + ',' + ','.join('%x:%x' % (o, v) for o, v in ints.items())
    innet = ','.join('%x:%x' % (o, v) for o, v in net.items())
    rows.append(chr(9).join([insc, innet, ','.join('%x' % a for a in elems), '%x' % struct.unpack('<I', e.read(0x6f653c, 4))[0], str(len(calls)), out]))
pathlib.Path(__file__).with_name('golden_scene_requests.tsv').write_text(chr(10).join(rows) + chr(10))
flags = [0] * 12
for row in rows:
    for item in row.split(chr(9))[5].split(','):
        o, v = item.split(':')
        if int(o, 16) < 0x5494 and int(v, 16) == 1:
            flags[(int(o, 16) - 0x5468) // 4] += 1
print(len(rows), 'cases; calls>0:', sum(1 for x in rows if x.split(chr(9))[4] != '0'), 'flag hits by slot', flags)

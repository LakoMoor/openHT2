"""Impact sparks 0x593720: slot allocation 0x5940d0, rand and the closing call 0x594c90 are replaced;
the vector helper 0x5ab400 and all arithmetic run in the original."""
import pathlib, random, struct
from emu import Emu, OBJ_BASE

r = random.Random(0x593720)
e = Emu()
THIS, POS, NRM, CLOCK = OBJ_BASE + 0x1000, OBJ_BASE + 0x2000, OBJ_BASE + 0x2100, OBJ_BASE + 0x3000
SLOTS = OBJ_BASE + 0x10000
SLOT = 0x100
seed = [1]


def rand(em):
    seed[0] = (seed[0] * 214013 + 2531011) & 0xffffffff
    return (seed[0] >> 16) & 0x7fff


st = {'avail': 0, 'used': 0, 'tail': None}


def alloc(em):
    if st['used'] >= st['avail']:
        return 0
    a = SLOTS + SLOT * st['used']
    st['used'] += 1
    return a


def tail(em):
    st['tail'] = [em.arg(i) for i in range(5)]
    return 0


e.hook_ret(0x61edde, rand)
e.hook_thiscall(0x5940d0, alloc, 0)
e.hook_ret(0x594c90, tail)
F = lambda *v: struct.pack('<%df' % len(v), *v)
for addr, v in ((0x702620, (0, 1, 0)), (0x702630, (0, 0, 1)), (0x702640, (1, 0, 0)), (0x702650, (0, 0, 0))):
    e.write(addr, F(*v))
rows = []
for case in range(120):
    glob = bytes(r.randrange(256) for _ in range(0x70))
    e.write(0x6f6648, glob[:0x60])
    e.write(THIS + 0x60, struct.pack('<I', CLOCK)); e.write(THIS + 0x68, struct.pack('<I', 0 if case % 17 == 5 else 1))
    t = r.uniform(0, 100)
    e.write(CLOCK + 0xb98, struct.pack('<d', t))
    pos = F(*[r.uniform(-20, 20) for _ in range(3)])
    kind = case % 4
    if kind == 0:
        n = F(0.0, 0.0, 1.0 if r.random() < 0.5 else -1.0)
    elif kind == 1:
        n = F(*[r.uniform(-1, 1) for _ in range(3)])
    elif kind == 2:
        v = [r.uniform(-1, 1) for _ in range(3)]; ln = sum(x * x for x in v) ** 0.5
        n = F(*[x / ln for x in v])
    else:
        n = F(0.0, 0.0, 0.0)
    count = r.choice([0, 1, 2, 5, 10, 30, 100, 255])
    avail = r.choice([0, 1, 3, 10, 40, 1000])
    e.write(POS, pos); e.write(NRM, n)
    e.write(SLOTS, b'\xaa' * (SLOT * 140))
    seed[0] = r.randrange(1 << 32)
    s0 = seed[0]
    st.update(avail=avail, used=0, tail=None)
    e.call(0x593720, [POS, NRM, count], 'iii', this=THIS)
    dump = b''.join(e.read(SLOTS + SLOT * i, 0x80) for i in range(st['used']))
    rows.append(chr(9).join([glob[:0x60].hex(), struct.pack('<d', t).hex(), pos.hex(), n.hex(), str(count), str(avail), str(s0),
                             str(0 if case % 17 == 5 else 1), str(st['used']), ','.join(map(str, st['tail'])) if st['tail'] else '-', dump.hex()]))
pathlib.Path(__file__).with_name('golden_impact_sparks.tsv').write_text(chr(10).join(rows) + chr(10))
print(len(rows), 'cases; slots:', sum(int(x.split(chr(9))[8]) for x in rows), 'tails', sum(1 for x in rows if x.split(chr(9))[9] != '-'))

"""The name table at 0x71e820 run unmodified: 0x5ef2b0 with a null owner (-> 0x5d2720, bucket by 0x5d2500:
byte sum modulo 500, chains through +8, name at +0xc, value at +0x10). Columns: bucket array contents as
`bucket:node,node` pairs, node records `addr,name,value`, the queried name, the result."""
import pathlib, random, struct
from emu import Emu, OBJ_BASE

r = random.Random(0x5d2720)
e = Emu()
rows = []
BASE = OBJ_BASE + 0x40000
TABLE = BASE            # bucket array, 500 pointers
NODES = BASE + 0x2000
STR = BASE + 0x20000
QUERY = BASE + 0x30000
WORDS = ['ILLUM_LEVEL_%02d' % i for i in range(20)] + ['MOSCOW', 'room', 'ROOM', 'a', 'b', 'ba', 'ab', 'x' * 40]


def bucket(s):
    return sum(s.encode()) % 500


for case in range(300):
    names = r.sample(WORDS, r.randrange(0, 12))
    e.write(TABLE, bytes(500 * 4))
    e.set_u32(0x71e828, TABLE)
    recs = []
    heads = {}
    for i, n in enumerate(names):
        node, sp = NODES + 0x20 * i, STR + 0x40 * i
        e.write(sp, n.encode() + b'\0')
        val = r.randrange(1, 2 ** 32)
        b = bucket(n)
        # new nodes go to the end of the chain
        e.write(node, bytes(0x20))
        e.set_u32(node + 0xc, sp)
        e.set_u32(node + 0x10, val)
        if b in heads:
            e.set_u32(heads[b] + 8, node)
        else:
            e.set_u32(TABLE + 4 * b, node)
        heads[b] = node
        recs.append((node, n, val))
    # a stray entry in another bucket that holds a different name than its bucket (collision of sums)
    q = r.choice(names + ['nothing', 'zz', ''] ) if names else r.choice(['nothing', ''])
    if r.random() < 0.15:
        q = ''
    e.write(QUERY, q.encode() + b'\0')
    arg = 0 if r.random() < 0.05 else QUERY
    e.call(0x5ef2b0, [0, arg], 'ii')
    rows.append(chr(9).join([','.join('%x:%s:%x' % (a, n, v) for a, n, v in recs), q, '%x' % (arg != 0), '%x' % e.eax]))
pathlib.Path(__file__).with_name('golden_name_table.tsv').write_text(chr(10).join(rows) + chr(10))
print(len(rows), 'cases;', sum(1 for x in rows if not x.endswith(chr(9) + '0')), 'found')

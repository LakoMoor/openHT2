"""NetVehIn::put 0x541e80 (fastcall) run unmodified on random packets; the logging calls 0x4e260c and 0x61cca9 are
counted. Rows: packet before, packet after, number of error reports."""
import pathlib, random, struct
from emu import Emu, OBJ_BASE

r = random.Random(0x541e80)
e = Emu()
P = OBJ_BASE + 0x10000
S = 0x200
errs = [0]
e.hook_ret(0x4e260c, lambda em: errs.__setitem__(0, errs[0] + 1) or 0)
e.hook_ret(0x61cca9, lambda em: 0)
rows = []
for case in range(400):
    pkt = bytearray(S)
    for o in range(0x20, S, 4):
        kind = r.random()
        pkt[o:o + 4] = struct.pack('<f', r.choice([r.uniform(-4, 4), r.uniform(-300, 300), r.uniform(-1, 1), 0.0, 1.0, -1.0, 100.0, r.uniform(-120, 120)]))
    for o in list(range(0x60, 0x6c)) + list(range(0xbc, 0xc2)):
        pkt[o] = r.choice([0, 1, 1, 2, r.randrange(256)])
    for o in (0x12c, 0x130, 0x138, 0x13c, 0x140):
        struct.pack_into('<I', pkt, o, r.choice([0, 1, 2, 5]))
    cnt = r.randrange(0, 5)
    flags = r.choice([0, 1, 1, 0x10, 0x11, 0x400]) | r.choice([0, 0, 0x100])
    struct.pack_into('<I', pkt, 0x10, cnt)
    struct.pack_into('<I', pkt, 0x50, flags)
    pkt[0x54] = r.choice([0, 3, 7, 7, 9, 200])
    struct.pack_into('<I', pkt, 0x5c, r.choice([0, 1, 2, 3, 4, 4, 5, 9]))
    struct.pack_into('<I', pkt, 0x134, r.choice([0, 3, 7, 7, 9, 0xfffffffe]))
    struct.pack_into('<HH', pkt, 0, r.randrange(65536), r.randrange(65536))
    struct.pack_into('<H', pkt, 8, r.randrange(65536))
    short = 24 + 3 * cnt
    long = 34 + 3 * cnt
    struct.pack_into('<I', pkt, 4, short if r.random() < 0.8 else short + r.randrange(1, 4))
    struct.pack_into('<I', pkt, 8, long if r.random() < 0.8 else long + r.randrange(1, 4))
    e.write(P, bytes(pkt))
    errs[0] = 0
    e.call(0x541e80, [], '', this=P)
    rows.append(chr(9).join([bytes(pkt).hex(), e.read(P, S).hex(), str(errs[0])]))
pathlib.Path(__file__).with_name('golden_net_veh_in_put.tsv').write_text(chr(10).join(rows) + chr(10))
print(len(rows), 'cases;', sum(1 for x in rows if x.endswith(chr(9) + '0')), 'without errors')

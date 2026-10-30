"""0x4ba130: network announcement 0x603110 and the spark start 0x593720 are recorded, 0x5ab4d0 runs."""
import pathlib, random, struct
from emu import Emu, OBJ_BASE
r = random.Random(0x4ba130)
e = Emu()
POS, VEC, CFG, CLOCK = OBJ_BASE + 0x100, OBJ_BASE + 0x200, OBJ_BASE + 0x300, OBJ_BASE + 0x400
rec = {'net': None, 'spark': None}


def net(em):
    rec['net'] = (em.ecx, em.arg(0), em.arg(2), e.read(em.arg(1), em.arg(2)).hex())
    return 0


def spark(em):
    rec['spark'] = (em.ecx, em.arg(0), em.arg(1), em.arg(2))
    return 0


e.hook_thiscall(0x603110, net, 3)
e.hook_thiscall(0x593720, spark, 3)
F = lambda *v: struct.pack('<%df' % len(v), *v)
rows = []
for case in range(200):
    rec.update(net=None, spark=None)
    pos = F(*[r.uniform(-50, 50) for _ in range(3)])
    sc = r.choice([1.0, 0.1, 5.0, 40.0, 0.0, 1e-3])
    vec = F(*[r.uniform(-1, 1) * sc for _ in range(3)])
    quality = r.choice([0.0, 0.3, 0.5, 0.5000001, 0.9, 1.0])
    f514, f524 = r.choice([0, 1, 2]), r.choice([0, 0, 7])
    send = r.randrange(2)
    e.write(POS, pos); e.write(VEC, vec); e.write(0x696030, F(quality))
    e.write(0x6cec6c, struct.pack('<I', CFG)); e.write(CFG + 0x514, struct.pack('<I', f514)); e.write(CFG + 0x524, struct.pack('<I', f524))
    e.write(0x696224, struct.pack('<I', CLOCK)); e.write(0x6f66a8, struct.pack('<I', 0x12345670))
    e.call(0x4ba130, [POS, VEC, send], 'iii')
    n = '-' if rec['net'] is None else ','.join(map(str, rec['net'][:3])) + ',' + rec['net'][3]
    s = '-' if rec['spark'] is None else ','.join(map(str, rec['spark']))
    rows.append(chr(9).join([pos.hex(), vec.hex(), F(quality).hex(), str(f514), str(f524), str(send), n, s]))
pathlib.Path(__file__).with_name('golden_impact_effect.tsv').write_text(chr(10).join(rows) + chr(10))
print(len(rows), sum(1 for x in rows if x.split(chr(9))[6] != '-'), 'announced', sum(1 for x in rows if x.split(chr(9))[7] != '-'), 'sparked')

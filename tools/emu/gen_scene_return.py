"""0x554670 (Vehicle::returnToLast dispatcher, ecx = scene) run unmodified; its seven callees are recording stubs
(thiscall with 2 or 1 stack words). Row: scene dwords `off:hex`, calls `addr:this:args`."""
import pathlib, random
from emu import Emu, OBJ_BASE

r = random.Random(0x554670)
e = Emu()
SCENE = OBJ_BASE + 0x10000
calls = []
for addr, n in [(0x581230, 2), (0x5812a0, 2), (0x5816d0, 2), (0x561d10, 2), (0x4f0fc0, 2), (0x4f1dd0, 1), (0x4f1e20, 1)]:
    def mk(addr, n):
        return lambda emu: (calls.append('%x:%x:%s' % (addr, emu.ecx, ','.join('%x' % emu.arg(i) for i in range(n)))), 0)[1]
    e.hook_thiscall(addr, mk(addr, n), n)
rows = []
for case in range(200):
    calls.clear()
    e.write(SCENE, bytes(0x5600))
    vals = {0x334: r.randrange(0, 2 ** 32), 0x5464: r.choice([0, OBJ_BASE + 0x20000]),
            0x5460: r.choice([0, OBJ_BASE + 0x30000]), 0x2aac: r.choice([0, 1, 1, 2])}
    for o, v in vals.items():
        e.set_u32(SCENE + o, v)
    e.call(0x554670, [], '', this=SCENE)
    rows.append(chr(9).join([','.join('%x:%x' % (o, v) for o, v in vals.items()), ','.join(calls)]))
pathlib.Path(__file__).with_name('golden_scene_return.tsv').write_text(chr(10).join(rows) + chr(10))
print(len(rows), 'cases;', sum(1 for x in rows if x.count(':') > 6), 'with car calls')

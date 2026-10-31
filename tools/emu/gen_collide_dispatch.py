"""0x58dff0 (collision query dispatcher, thiscall(a,b,c,d,mode), ret 0x14 is 5 stack args) run unmodified; the callees
0x5843c0 (mode&4), 0x5855a0 (mode&2), 0x58b120 + 0x58cf30 (otherwise) and the debug trace (0x4e9ce4) are recording stubs.
Row: mode, [0x6f6620], dword [d+0x74], args hex, result, events."""
import pathlib, random
from emu import Emu, OBJ_BASE

r = random.Random(0x58dff0)
e = Emu()
e.install_crt_hooks()
THIS, D = OBJ_BASE + 0x10000, OBJ_BASE + 0x12000
ev = []
ret = [0]
def mk(name, n):
    def f(emu):
        ev.append('%s|%s' % (name, ','.join('%x' % emu.arg(i) for i in range(n))))
        return ret[0]
    return f
e.hook_ret(0x5843c0, mk('q4', 5))
e.hook_ret(0x5855a0, mk('q2', 6))
e.hook_ret(0x58b120, mk('q0', 6))
e.hook_thiscall(0x58cf30, lambda emu: (ev.append('after|%x' % emu.ecx), 0)[1], 0)
e.hook_ret(0x4e9ce4, lambda emu: (ev.append('trace'), 0)[1])
e.hook_ret(0x4ea2cb, lambda emu: 0)
e.hook_ret(0x4e9d90, lambda emu: 0)
rows = []
for case in range(300):
    ev.clear()
    e.write(THIS, bytes(0x400)); e.write(D, bytes(0x100))
    mode = r.choice([0, 1, 2, 3, 4, 5, 6, 7, 8])
    dbg = r.choice([0, 1])
    w74 = r.choice([0, 0, 5])
    e.set_u32(0x6f6620, dbg); e.set_u32(D + 0x74, w74); e.set_u32(THIS + 0x320, D + 0x80); e.set_u32(D + 0x8c, D + 0x90)
    ret[0] = r.choice([0, 1, 2, 3, r.randrange(0, 2 ** 32)])
    args = [r.randrange(0, 2 ** 32) for _ in range(3)] + [D]
    e.call(0x58dff0, args + [mode], 'iiiii', this=THIS); out = e.eax
    rows.append(chr(9).join([str(mode), str(dbg), str(w74), ','.join('%x' % a for a in args), '%x' % ret[0], ';'.join(ev), '%x' % (out & 0xffffffff)]))
pathlib.Path(__file__).with_name('golden_collide_dispatch.tsv').write_text(chr(10).join(rows) + chr(10))
print(len(rows)); print(rows[0]); print(sum('trace' in x for x in rows), 'traces')

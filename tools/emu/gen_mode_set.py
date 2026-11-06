"""0x58cf30 (set all flags +0xf28.. to 1) and 0x58cf60 (body mode switch, thiscall (mode), ret 4) run unmodified with the real
0x5ab710 and 0x58d0f0 family; stubs: 0x5ef5c0 (ctx constructor), 0x592a20 (cast), 0x4e5ba7 (room), virtuals +0x1c, +0x8c.
Row: kind, mode, this words, ctx hex, cast list, r5ba7, diff of this, events."""
import pathlib, random, struct
from emu import Emu, OBJ_BASE, STUB

r = random.Random(0x58cf60)
e = Emu()
F = lambda *v: struct.pack('<%df' % len(v), *v)
U = lambda *v: struct.pack('<%dI' % len(v), *v)
THIS, VT = OBJ_BASE + 0x10000, OBJ_BASE + 0x14000
e.write(0x7025f0, F(1, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0)); e.write(0x702650, F(0, 0, 0))
ev, st = [], {}
hx = lambda a, n: e.read(a, n).hex()
def virt(slot, nargs, fmt):
    addr = STUB + 0x400 + 0x10 * (slot // 4)
    e.hook_thiscall(addr, lambda emu: (ev.append(fmt(emu)), 0)[1], nargs)
    e.set_u32(VT + slot, addr)
virt(0x1c, 1, lambda emu: 'v1c|%x' % emu.arg(0))
virt(0x8c, 2, lambda emu: 'v8c|%x|%s' % (emu.arg(0), hx(emu.arg(1), 48)))
e.hook_thiscall(0x5ef5c0, lambda emu: (e.write(emu.ecx, st['ctx']), 0)[1], 0)
def cast(emu):
    i = st['n']; st['n'] += 1
    ev.append('cast|%x' % emu.arg(0))
    hit, d = st['casts'][min(i, len(st['casts']) - 1)]
    e.write(emu.arg(2), F(d))
    return hit
e.hook_ret(0x592a20, cast)
def room(emu):
    ev.append('5ba7|%x,%x,%s' % (emu.arg(0), emu.arg(1), hx(emu.arg(2), 12)))
    return st['room']
e.hook_ret(0x4e5ba7, room)
val = lambda k: r.choice([0.0, 1.0, -1.0, r.uniform(-k, k)])
rows = []
for case in range(400):
    ev.clear()
    e.write(THIS, bytes(0x1000)); e.set_u32(THIS, VT)
    w = {}
    kind = r.choice([0, 1, 1, 1])
    w[0xdf8] = struct.pack('<i', r.choice([0, 3, 7, 20]))
    w[0xe04] = U(r.choice([0, 1, 2])); w[0xf90] = U(r.choice([0, 0, 1])); w[0x320] = U(0x2a0b0000)
    for i in range(12): w[0x1f4 + 4 * i] = F(val(5))
    for i in range(12): w[0x218 + 4 * i] = F(val(5))
    for o, b in w.items(): e.write(THIS + o, b)
    mode = r.choice([0, 0, 1, 2])
    st['ctx'] = b''.join(F(val(2)) for _ in range(0x30))
    st['n'] = 0
    st['casts'] = [(r.choice([1, 1, 1, 0]), r.choice([0.0, 0.3, r.uniform(-1, 2)])) for _ in range(3)]
    st['room'] = r.choice([0, 0x2a0c0000])
    before = e.read(THIS, 0x1000)
    if kind == 0:
        e.call(0x58cf30, [], '', this=THIS)
    else:
        try:
            e.call(0x58cf60, [mode], "i", this=THIS)
        except Exception:
            print("eip", hex(e.mu.reg_read(__import__("unicorn.x86_const",fromlist=["x"]).UC_X86_REG_EIP)), ev, mode, st["casts"]); raise
    after = e.read(THIS, 0x1000)
    diff = ','.join('%x:%s' % (i, after[i:i + 4].hex()) for i in range(0, 0x1000, 4) if after[i:i + 4] != before[i:i + 4])
    rows.append(chr(9).join([str(kind), str(mode), ','.join('%x:%s' % (o, b.hex()) for o, b in w.items()), st['ctx'].hex(),
        ';'.join('%d,%s' % (h, F(d).hex()) for h, d in st['casts']), '%x' % st['room'], diff, ';'.join(ev)]))
pathlib.Path(__file__).with_name('golden_mode_set.tsv').write_text(chr(10).join(rows) + chr(10))
from collections import Counter
c = Counter(t.split('|')[0] for x in rows for t in x.split(chr(9))[7].split(';') if t)
print(len(rows), dict(c))

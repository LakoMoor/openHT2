"""Numeric observations of fillTownOrder's substitution branch (king.exe 0x41b3a0).

The already ported legal merchandise/destination selection is supplied by hooks. The
original deadline, fee, share predicate, illegal selector and RNG execute unchanged.
"""
import pathlib
import random
import struct
from emu import Emu, OBJ_BASE

e = Emu()
e.install_crt_hooks()
def allocate(emu):
    a = emu.heap_next
    emu.heap_next += max(16, (emu.arg(0) + 15) & ~15)
    return a
e.hook_ret(0x61d857, allocate)
e.hook_ret(0x61d1c8, lambda emu: 0)
e.hook_ret(0x63266c, lambda emu: 0)
e.hook_thiscall(0x401aa0, lambda emu: 0, 0)
e.set_u32(0x64b2b0, 0x1f000300)
e.write(0x1f000300, bytes.fromhex('8b4424048b4c2408870889c8c20800'))
out, node, town, human = [OBJ_BASE + i * 0x1000 for i in range(4)]
goods, distances = OBJ_BASE + 0x4000, OBJ_BASE + 0x5000
u = lambda a: struct.unpack('<I', e.read(a, 4))[0]
si = lambda a: struct.unpack('<i', e.read(a, 4))[0]
f = lambda a, v: e.write(a, struct.pack('<d', v))
for a, n in ((0x401870, 2), (0x4018b0, 1)):
    e.hook_thiscall(a, lambda emu: 0, n)
for a in (0x401010, 0x5d2ba0):
    e.hook_ret(a, lambda emu: 0)

def result(emu, value):
    emu.set_u32(emu.arg(0), value)
    return emu.arg(0)

e.hook_ret(0x477b60, lambda emu: result(emu, human))
e.hook_ret(0x41b2a0, lambda emu: result(emu, 0))
e.hook_ret(0x41b120, lambda emu: result(emu, 0))
e.hook_ret(0x4052a0, lambda emu: result(emu, 1))
# Return an x87 double for the game clock.
e.write(0x401000, b'\xdd\x05' + struct.pack('<I', OBJ_BASE + 0x7000) + b'\xc3')
f(OBJ_BASE + 0x7000, 1234.5)
e.set_u32(town + 100, distances)
f(distances + 8, 54321.0)
e.set_u32(0x68ccb8, goods)
r = random.Random(0x41b3a0)
rows = []
changed = 0
for case in range(480):
    share = (0.0, 0.25, 0.25000000000000006, 0.51)[case % 4]
    slot = (case // 4) % 2
    flags = [r.choice((8, 16, 4)), *[r.randrange(32) for _ in range(case % 9)]]
    table = [r.getrandbits(32) for _ in range(55)]
    if case % 3 == 0:
        table = [0] * 55
        table[1], table[2], table[3] = 299, 2, r.getrandbits(32)
    e.write(0x68b9c8, struct.pack('<55I', *table))
    e.set_u32(0x68baa4, 0)
    e.set_u32(0x68baa8, 31)
    f(human + 0x300, share)
    e.set_u32(0x68ccbc, goods + len(flags) * 0x68)
    for j, flag in enumerate(flags):
        e.set_u32(goods + j * 0x68 + 0x30, flag)
    e.call(0x41b3a0, [out, node, town, slot], 'iiii')
    order = u(out)
    fields = [si(order + j * 4) for j in range(4)]
    changed += fields[1] >= 0
    floats = struct.unpack('<2d', e.read(order + 16, 16))
    state = [u(0x68baa4), u(0x68baa8), *struct.unpack('<55I', e.read(0x68b9c8, 220))]
    rows.append(' '.join(map(str, [share, slot, len(flags), *flags, *table, *fields, *floats, *state])))
assert changed > 0
path = pathlib.Path(__file__).resolve().parents[2] / 'crates/ht2-sim/tests/data/illegal_order_golden.txt'
path.write_text('\n'.join(rows) + '\n', encoding='utf-8')
print(f'{len(rows)} cases; {changed} substitutions; {path}')

"""0x587e20 (thiscall, 7 stack args, ret 0x1c) run unmodified; callees 0x5ab4d0 / 0x4e27a5 (float results), 0x5dbec0,
0x56f210, 0x56f120 and the error log 0x4e260c are stubs. Row: arg1, f(0x5ab4d0), now, found(handle|-1), flagsret, a3,a4, args6 hex,
this-pre hex(0x100 at +0x4b0 plus +0x1c4..), diff, ret, events."""
import pathlib, random, struct
from emu import Emu, OBJ_BASE

r = random.Random(0x587e20)
e = Emu()
e.hook_ret(0x61cca9, lambda emu: 0)
THIS, DATA = OBJ_BASE + 0x10000, OBJ_BASE + 0x12000
FV, NV = OBJ_BASE + 0x13000, OBJ_BASE + 0x13010
ev = []
st = {'handle': 0, 'flags': 0}
e.write(0x5ab4d0, b'\xd9\x05' + struct.pack('<I', FV) + b'\xc3')
e.write(0x4e27a5, b'\xd9\x05' + struct.pack('<I', NV) + b'\xc3')
e.hook_ret(0x4e260c, lambda emu: (ev.append('err'), 0)[1])
def lookup(emu):
    ev.append('find|%x|%x,%x,%x|%x' % (emu.arg(0), emu.arg(1), emu.arg(2), emu.arg(3), emu.arg(4)))
    out = emu.arg(5)
    if st['handle'] != 0xffffffff:
        emu.set_u32(out, st['handle'])
    return 0
e.hook_thiscall(0x5dbec0, lookup, 6)
e.hook_ret(0x56f210, lambda emu: (ev.append('flags|%x' % emu.arg(0)), st['flags'])[1])
e.hook_ret(0x56f120, lambda emu: (ev.append('attach|%x,%x,%x' % (emu.arg(0), emu.arg(1), emu.arg(2))), 0)[1])
rows = []
for case in range(600):
    ev.clear()
    e.write(THIS, bytes(0x600)); e.write(DATA, bytes(0x200))
    pre = bytes(r.randrange(256) for _ in range(0x100))
    e.write(THIS + 0x4b0, pre[:0x40]); e.write(THIS + 0x218, pre[0x40:0x4c]); e.set_u32(THIS + 0x320, r.randrange(2 ** 32))
    e.set_u32(THIS + 0x37c, r.randrange(2 ** 32))
    e.set_f32(FV, r.choice([5.0, 5.5, 100.0, 6.0, 9.0, 4.0, float("nan")]))
    e.set_f32(NV, r.uniform(0, 1000))
    st['handle'] = r.choice([0xffffffff, r.randrange(0, 50), r.randrange(0, 50)])
    st['flags'] = r.choice([0, 0, 0, 1, 2, 4, 8, 0x10, 0x20, 0x30, r.randrange(256)])
    arg1 = r.choice([1, 2, 3, 4, 5, 6, 7, 0, 3, 3, 3, 3, 6, 5])
    a3 = r.choice([0, r.randrange(1, 2 ** 32)])
    a4 = r.randrange(2 ** 32)
    p5, p6 = DATA + 0x10, DATA + 0x40
    e.write(p5, bytes(r.randrange(256) for _ in range(12)))
    if arg1 == 6:
        s1 = bytes(r.randrange(1, 256) for _ in range(r.randrange(0, 12))); s2 = bytes(r.randrange(1, 256) for _ in range(r.randrange(0, 12)))
        buf = s1 + b'\0' + s2 + b'\0'
    elif arg1 == 3:
        buf = struct.pack('<I', r.choice([0, 1, 2, 0x10000, 0x10001, 0xffff0002, 3, 0, 1, 2]))
    else:
        buf = struct.pack('<I', r.randrange(2 ** 32))
    e.write(p6, buf)
    before = e.read(THIS, 0x600)
    a0 = r.choice([2, 2, 2, 2, 1])
    try:
        e.call(0x587e20, [a0, arg1, 0, a3, a4, p5, p6], 'iiiiiii', this=THIS); out = e.eax
    except Exception as ex:
        from unicorn.x86_const import UC_X86_REG_EIP, UC_X86_REG_ESI, UC_X86_REG_EDI
        print(case, arg1, hex(e.mu.reg_read(UC_X86_REG_EIP)), hex(e.mu.reg_read(UC_X86_REG_ESI)), hex(e.mu.reg_read(UC_X86_REG_EDI))); raise
    after = e.read(THIS, 0x600)
    diff = ','.join('%x:%s' % (i, after[i:i + 4].hex()) for i in range(0, 0x600, 4) if after[i:i + 4] != before[i:i + 4])
    rows.append(chr(9).join([str(a0), str(arg1), struct.pack('<f', e.f32(FV)).hex(), struct.pack('<f', e.f32(NV)).hex(), '%x' % st['handle'], '%x' % st['flags'],
        '%x' % a3, '%x' % a4, bytes(e.read(p5, 12)).hex(), bytes(buf).hex(), before[0x1c4:0x1c8].hex() + '|' + before[0x218:0x224].hex() + '|' + before[0x320:0x324].hex() + '|' + before[0x37c:0x380].hex() + '|' + before[0x4b0:0x4f0].hex(), diff, '%x' % (out & 0xffffffff), ';'.join(ev)]))
pathlib.Path(__file__).with_name('golden_item_bind.tsv').write_text(chr(10).join(rows) + chr(10))
print(len(rows)); print(rows[3]); print(sum('attach' in x for x in rows), 'attach')

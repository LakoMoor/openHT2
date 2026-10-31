"""The whole 0x54ff80 (per-frame update of the vehicle scene, ecx = scene) run unmodified on random scenes.
External owners are recording stubs: 0x54ef90 (impact), the net stream (0x602be0, 0x603110, 0x5c3630, 0x602e80),
the virtuals [vtable+0x4c]/[+0xcc] and 0x5d32f0 of the object reset, the seven callees of 0x554670;
0x555500 is `fld [LEVEL]; ret`. The ray probe, effects, lights and maneuver owners are not reached (their gates are
closed: no owner at +0x2aa8, +0x51a4 bit 0x100 clear, +0x51bc zero, +0x4c4 zero), they are covered by their own goldens.
Row: scene / net object / clock / config / second car object (sparse `off:hex`), globals (`addr:hex`), events,
changes (`S|N|O:off:dword`)."""
import pathlib, random, struct, sys
from emu import Emu, OBJ_BASE, STUB

r = random.Random(0x54ff80)
e = Emu()
e.install_crt_hooks()
SCENE, NET, OBJ2, VT = OBJ_BASE + 0x10000, OBJ_BASE + 0x20000, OBJ_BASE + 0x30000, OBJ_BASE + 0x31000
CLOCK, CFG, WORLD, LEVEL = OBJ_BASE + 0x100000, OBJ_BASE + 0x110000, OBJ_BASE + 0x120000, OBJ_BASE + 0x40000
OBJ3 = OBJ_BASE + 0x50000
SIZES = {'S': (SCENE, 0x5600), 'T': (OBJ3, 0x100), 'N': (NET, 0x3c00), 'O': (OBJ2, 0x1000), 'C': (CLOCK, 0x2000), 'G': (CFG, 0x600)}
ev = []
V4C, VCC = STUB + 0x200, STUB + 0x210
e.set_u32(VT + 0x4c, V4C)
e.set_u32(VT + 0xcc, VCC)
e.write(0x555500, b'\xd9\x05' + struct.pack('<I', LEVEL) + b'\xc3')
e.hook_thiscall(V4C, lambda emu: (ev.append('v|4c|%x' % emu.ecx), 0)[1], 0)
e.hook_thiscall(VCC, lambda emu: (ev.append('v|cc|%x' % emu.ecx), 0)[1], 0)
e.hook_thiscall(0x5d32f0, lambda emu: (ev.append('c|5d32f0|%x|' % emu.ecx), 0)[1], 1)
e.hook_thiscall(0x602be0, lambda emu: 0, 0)
e.hook_thiscall(0x5c3630, lambda emu: 0, 1)
e.hook_thiscall(0x602e80, lambda emu: 0, 0)
e.hook_thiscall(0x603110, lambda emu: (ev.append('n|%d|%s' % (struct.unpack('<i', struct.pack('<I', emu.arg(0)))[0],
                                                         bytes(e.read(emu.arg(1), emu.arg(2))).hex())), 0)[1], 3)
for addr, n in [(0x581230, 2), (0x5812a0, 2), (0x5816d0, 2), (0x561d10, 2), (0x4f0fc0, 2), (0x4f1dd0, 1), (0x4f1e20, 1)]:
    def mk(addr, n):
        return lambda emu: (ev.append('r|%x|%x|%s' % (addr, emu.ecx, ','.join('%x' % emu.arg(i) for i in range(n)))), 0)[1]
    e.hook_thiscall(addr, mk(addr, n), n)
e.hook_ret(0x54ef90, lambda emu: (ev.append('i|%s|%s' % (e.read(emu.arg(1), 12).hex(), e.read(emu.arg(2), 12).hex())), 0)[1])

F = lambda *v: struct.pack('<%df' % len(v), *v)
e.write(0x702620, F(0, 1, 0)); e.write(0x702630, F(0, 0, 1)); e.write(0x702640, F(1, 0, 0)); e.write(0x702650, F(0, 0, 0))
e.write(0x7025f0, F(1, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0))
GLOBALS = [0x6ef200, 0x6cec6c, 0x6ef1fc, 0x6974c4, 0x6f33ac, 0x6796b0, 0x696c6c, 0x6ef2a0, 0x6ef2a4, 0x71e828, 0x6ef254, 0x6ef258]
GW = {0x6974c2: 2, 0x67d10a: 2}


def sparse(b):
    out = []
    for i in range(0, len(b), 16):
        c = b[i:i + 16]
        if any(c):
            out.append('%x:%s' % (i, c.hex()))
    return ','.join(out)


def rf(lo, hi):
    return struct.unpack('<I', F(r.uniform(lo, hi)))[0]


def junk():
    c = r.random()
    if c < 0.35:
        return 0
    if c < 0.5:
        return r.choice([1, 2, 3, 4])
    if c < 0.6:
        return 0x3f800000
    return rf(-2, 2)


COUNT = int(sys.argv[1]) if len(sys.argv) > 1 else 200
rows, skipped, faults = [], 0, {}
seed_case = 0
while len(rows) < COUNT:
    seed_case += 1
    ev.clear()
    for k, (a, n) in SIZES.items():
        e.write(a, bytes(n))
    e.write(WORLD, bytes(0x100)); e.write(LEVEL, bytes(4))
    # scene: random words everywhere, then the structural fields
    dens = r.choice([0.03, 0.06, 0.12])
    words = [junk() if r.random() < dens else 0 for _ in range(0x5600 // 4)]
    e.write(SCENE, struct.pack('<%dI' % len(words), *words))
    S = lambda off, v: e.set_u32(SCENE + off, v)
    S(0x2aa8, 0)                      # no ray cast owner
    S(0x51a4, r.choice([0, 1, 0x80, 0x81, 2, 0x82]))   # bit 0x100 clear: no effects
    S(0x51bc, 0)                      # no lights
    S(0x4c4, r.choice([0, 1]))        # the maneuver stops at the lookup of the object +0x414 (never in the list)
    S(0x2a38, r.choice([0, 0, 0, 1]))
    S(0x2540, OBJ3)
    e.write(OBJ3, struct.pack('<%dI' % 0x28, *[r.choice([0, 1, 2, 3, 4]) for _ in range(0x28)]))
    e.set_u32(OBJ3 + 8, r.choice([1, 4, 8]))
    for i in range(9):
        if r.random() < 0.7:
            e.set_u32(SCENE + 0x523c + 4 * i, e.u32(OBJ3 + 4 + 0x14 * i) if hasattr(e, 'u32') else struct.unpack('<I', e.read(OBJ3 + 4 + 0x14 * i, 4))[0])
    S(0x38c, r.choice([0, 1, 1])); S(0x390, r.choice([0, 1])); S(0x2aac, r.choice([0, 1, 1]))
    S(0x5464, OBJ2 if r.random() < 0.6 else 0)
    S(0x5460, NET if r.random() < 0.6 else 0)
    S(0x594, r.choice([0, 1, 4, 5, 6, 2]))
    S(0x28bc, r.choice([0, 1, 2, 4]))
    S(0x28b4, 0)
    S(0x174, r.choice([0, 0, 1])); S(0x4a8, r.choice([0, 1])); S(0x37c, r.choice([0, 1]))
    S(0x4b4, r.randrange(0, 2 ** 32)); S(0x184, r.choice([0, 0, 1]))
    S(0x10, VT)
    S(0x414, r.choice([0, 0, 0x77]))
    e.write(SCENE + 0x5260, bytes(0x40))
    e.write(SCENE + 0x1a4, F(*[r.choice([1.0, 2.0, 0.5, r.uniform(0.2, 3)]) for _ in range(3)]))
    e.write(SCENE + 0x19c, F(r.choice([500.0, 5000.0, 20000.0])))
    # net object: pointer-free random content
    nw = [junk() if r.random() < 0.08 else 0 for _ in range(0x3c00 // 4)]
    e.write(NET, struct.pack('<%dI' % len(nw), *nw))
    for o in range(0x2b68, 0x3400, 4):
        e.set_u32(NET + o, 0)
    e.set_u32(NET + 0x34ac, r.choice([1, 1, 0])); e.set_u32(NET + 0x3344, r.choice([0, 0, 1]))
    e.write(NET + 0x332c, F(r.choice([0.0, 0.5, 3.0, 20.0, r.uniform(-30, 30)]), r.uniform(-3, 3)))
    ow = [junk() if r.random() < 0.15 else 0 for _ in range(0x1000 // 4)]
    e.write(OBJ2, struct.pack('<%dI' % len(ow), *ow))
    # clock, config, world
    e.write(CLOCK + 0xb90, struct.pack('<d', r.choice([0.016, 0.033, 0.1, r.uniform(0.001, 0.3)])))
    e.write(CLOCK + 0xb98, struct.pack('<d', r.uniform(0, 1000)))
    e.set_u32(CLOCK + 0xbb0, r.randrange(0, 5000))
    e.set_u32(CFG + 0x514, r.choice([0, 1, 2])); e.set_u32(CFG + 0x524, r.choice([0, 1, 2]))
    e.write(WORLD + 0x58, struct.pack('<d', r.choice([0.0, 4.0, 5.0, 12.0, 0x16 + 0.5, 0x17, 30.0, -3.0])))
    e.write(LEVEL, F(r.choice([0.0, 1.0, -2.5, r.uniform(-100, 100)])))
    for a, v in {0x6ef200: 0, 0x6cec6c: CFG, 0x6ef1fc: CLOCK, 0x6974c4: CLOCK, 0x6f33ac: WORLD,
                 0x6796b0: r.choice([0, 8, 8]), 0x696c6c: 0, 0x6ef254: 0, 0x6ef258: 0}.items():
        e.set_u32(a, v)
    e.write(0x6974c2, struct.pack('<h', 0))
    e.write(0x6ef2a0, F(r.uniform(0, 3), r.uniform(0, 3)))
    e.write(0x67d10a, struct.pack('<h', r.choice([0, 1, 5, -1])))
    e.set_u32(0x71e828, 0)
    before = {k: e.read(a, n) for k, (a, n) in SIZES.items()}
    gl = ','.join('%x:%s' % (a, e.read(a, 4).hex()) for a in GLOBALS) + ',' + ','.join('%x:%s' % (a, e.read(a, n).hex()) for a, n in GW.items()) + ',555500:' + e.read(LEVEL, 4).hex()
    gl += ',ar:' + e.read(WORLD + 0x58, 8).hex() + ',ac:' + e.read(0x6ef2a0, 8).hex()
    try:
        e.call(0x54ff80, [], '', this=SCENE, max_insn=3_000_000)
    except Exception as ex:
        skipped += 1
        faults[(str(ex)[:40], hex(e.mu.reg_read(__import__('unicorn.x86_const',fromlist=['x']).UC_X86_REG_EIP)))] = faults.get((str(ex)[:40], hex(e.mu.reg_read(__import__('unicorn.x86_const',fromlist=['x']).UC_X86_REG_EIP))), 0) + 1
        e.reset_cpu() if hasattr(e, 'reset_cpu') else None
        continue
    ch = []
    for k in ('S', 'N', 'O', 'T'):
        a, n = SIZES[k]
        after = e.read(a, n)
        for i in range(0, n, 4):
            if after[i:i + 4] != before[k][i:i + 4]:
                ch.append('%s:%x:%s' % (k, i, after[i:i + 4].hex()))
    rows.append(chr(9).join([sparse(before['S']), sparse(before['N']), sparse(before['O']), sparse(before['C']), sparse(before['G']), sparse(before['T']), gl,
                             ';'.join(ev), ','.join(ch)]))
pathlib.Path(__file__).with_name('golden_scene_update.tsv').write_text(chr(10).join(rows) + chr(10))
print(len(rows), 'cases;', skipped, 'skipped (emulator fault); with events', sum(1 for x in rows if x.split(chr(9))[7]),
      '; net', sum(1 for x in rows if '\tn|' in x or ';n|' in x), '; return', sum(1 for x in rows if 'r|' in x.split(chr(9))[7]))
print(sorted(faults.items(), key=lambda kv: -kv[1])[:8])

"""Wheel dust levels of 0x54ff80 (0x55059f..0x5506cd), entered with ebx = 8 and esi = 0 as at that point."""
import pathlib, random, struct
from emu import Emu, OBJ_BASE, STACK_TOP
from unicorn.x86_const import UC_X86_REG_ESP, UC_X86_REG_EBP, UC_X86_REG_ESI, UC_X86_REG_EBX, UC_X86_REG_EDI, UC_X86_REG_FPCW

r = random.Random(0x55059f)
e = Emu()
SCENE, OWNER, SURF = OBJ_BASE + 0x10000, OBJ_BASE + 0x20000, OBJ_BASE + 0x30000
F = lambda *v: struct.pack('<%df' % len(v), *v)
rows = []
for case in range(400):
    e.write(SCENE, bytes(0x5400)); e.write(OWNER, bytes(0x4000))
    e.write(SCENE + 0x27f0, b'\x77' * 32)
    speed = r.choice([r.uniform(-40, 40), 0.5, 1.0, 1.0000001, 14.99, 15.0, 30.0])
    slip = r.uniform(-30, 30)
    mass = r.choice([r.uniform(100, 20000), 10000.0, 3000.0])
    wheels = r.randrange(0, 9)
    owner_on = r.choice([1, 1, 1, 0]); track = r.choice([1, 1, 1, 2]); busy = r.choice([0, 0, 0, 1])
    e.write(OWNER + 0x332c, F(speed, slip)); e.write(SCENE + 0x19c, F(mass))
    e.set_u32(SCENE + 0x5460, OWNER if owner_on else 0); e.set_u32(SCENE + 0x2aac, track); e.set_u32(OWNER + 0x3344, busy)
    e.set_u32(SCENE + 0x28bc, wheels)
    spec = []
    for i in range(8):
        on = r.choice([1, 1, 0]); blocked = r.choice([0, 0, 0, 1]); sk = r.choice([6, 10, 11, 6, 3, 0]); has = r.choice([1, 1, 1, 0]); has2 = r.choice([1, 1, 1, 0])
        e.write(OWNER + 0xf8 * i + 0x2b68, struct.pack('<H', on))
        e.set_u32(OWNER + 0x3444 + 4 * i, blocked)
        e.set_u32(OWNER + 0xf8 * i + 0x2bd4, SURF + 0x40 * i if has else 0)
        e.set_u32(OWNER + 0xf8 * i + 0x2bdc, 0x1234 if has2 else 0)
        e.set_u32(SURF + 0x40 * i + 4, sk)
        spec.append((on, blocked, sk, has, has2))
    sp = STACK_TOP - 0x3000
    mu = e.mu
    mu.reg_write(UC_X86_REG_ESP, sp); mu.reg_write(UC_X86_REG_EBP, SCENE); mu.reg_write(UC_X86_REG_ESI, 0); mu.reg_write(UC_X86_REG_EBX, 8); mu.reg_write(UC_X86_REG_EDI, 1)
    mu.reg_write(UC_X86_REG_FPCW, 0x27f)
    mu.emu_start(0x55059f, 0x5506cd, count=100000)
    rows.append(chr(9).join([F(speed, slip).hex(), F(mass).hex(), str(wheels), str(owner_on), str(track), str(busy), ';'.join(','.join(map(str, s)) for s in spec), e.read(SCENE + 0x27f0, 32).hex()]))
pathlib.Path(__file__).with_name('golden_scene_wheel_levels.tsv').write_text(chr(10).join(rows) + chr(10))
print(len(rows), sum(1 for x in rows if any(c != '0' for c in x.split(chr(9))[-1])), 'with levels')

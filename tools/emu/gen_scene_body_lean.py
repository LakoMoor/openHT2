"""Body lean smoothing of 0x54ff80 (0x5501d7..0x550452) with the original pow 0x61db50."""
import pathlib, random, struct
from emu import Emu, OBJ_BASE, STACK_TOP
from unicorn.x86_const import UC_X86_REG_ESP, UC_X86_REG_EBP, UC_X86_REG_ESI, UC_X86_REG_EDI, UC_X86_REG_FPCW

r = random.Random(0x5501d7)
e = Emu()
for addr, v in ((0x702620, (0, 1, 0)), (0x702630, (0, 0, 1)), (0x702640, (1, 0, 0)), (0x702650, (0, 0, 0))):
    e.write(addr, struct.pack('<3f', *v))
SCENE, CLOCK = OBJ_BASE + 0x10000, OBJ_BASE + 0x100000
e.set_u32(0x6974c4, CLOCK)
e.write(0x6974c2, struct.pack('<h', 0))
F = lambda *v: struct.pack('<%df' % len(v), *v)
rows = []
for case in range(300):
    e.write(SCENE, bytes(0x5000))
    mat = F(*[r.uniform(-1.2, 1.2) for _ in range(12)])
    pos = F(*[r.uniform(-30, 30) for _ in range(3)])        # 0x1b0
    f1a4 = F(*[r.choice([r.uniform(-4, 4), 1.0, 0.0]) for _ in range(3)])
    f1bc = F(*[r.uniform(-5, 5) for _ in range(3)])
    prev = F(*[x + r.uniform(-1, 1) for x in struct.unpack('<3f', pos)])   # 0x1d4
    f1e0 = F(*[r.uniform(-1, 1) for _ in range(3)])
    lean = F(r.uniform(-0.1, 0.1), r.uniform(-0.1, 0.1))
    dt = r.choice([0.016, 0.033, 0.1, 0.3, 0.001, r.uniform(0.001, 0.3)])
    g = F(r.uniform(0, 3), r.uniform(0, 3))
    e.write(SCENE + 0x264, mat); e.write(SCENE + 0x1b0, pos); e.write(SCENE + 0x1a4, f1a4); e.write(SCENE + 0x1bc, f1bc)
    e.write(SCENE + 0x1d4, prev); e.write(SCENE + 0x1e0, f1e0); e.write(SCENE + 0x4f54, lean)
    e.write(SCENE + 0x2a3c, F(1.0, 2.0, 3.0)); e.set_u32(SCENE + 0x2a38, 1)
    e.write(0x6ef2a0, g)
    e.write(CLOCK + 0xb90, struct.pack('<d', dt)); e.write(CLOCK + 0xb98, struct.pack('<d', r.uniform(0, 100)))
    sp = STACK_TOP - 0x3000
    e.write(sp, bytes(0x200))
    mu = e.mu
    mu.reg_write(UC_X86_REG_ESP, sp); mu.reg_write(UC_X86_REG_EBP, SCENE); mu.reg_write(UC_X86_REG_ESI, 0); mu.reg_write(UC_X86_REG_EDI, 1)
    mu.reg_write(UC_X86_REG_FPCW, 0x27f)
    mu.emu_start(0x5501d7, 0x550452, count=200000)
    out = e.read(SCENE + 0x4f54, 8) + e.read(SCENE + 0x2a3c, 12) + e.read(SCENE + 0x2a38, 4)
    rows.append(chr(9).join([mat.hex(), pos.hex(), f1a4.hex(), f1bc.hex(), prev.hex(), f1e0.hex(), lean.hex(), struct.pack('<d', dt).hex(), g.hex(), out.hex()]))
pathlib.Path(__file__).with_name('golden_scene_body_lean.tsv').write_text(chr(10).join(rows) + chr(10))
print(len(rows))

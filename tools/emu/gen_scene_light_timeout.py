"""0x55186d..0x551895 of 0x54ff80: [scene+0x5080] = 1 once [+0x5090] + [+0x508c] is below the game clock; entered mid-function."""
import pathlib, random, struct
from emu import Emu, OBJ_BASE, STACK_TOP
from unicorn.x86_const import UC_X86_REG_ESP, UC_X86_REG_EBP, UC_X86_REG_FPCW

r = random.Random(0x55186d)
e = Emu()
SCENE, CLOCK = OBJ_BASE + 0x10000, OBJ_BASE + 0x40000
rows = []
for case in range(300):
    e.write(SCENE, bytes(0x5100))
    a, b = r.choice([r.uniform(0, 50), 0.0, 1.5]), r.choice([r.uniform(0, 50), 0.0, 1.5])
    t = r.choice([a + b, a + b + 1e-9, (a + b) - 1e-9, r.uniform(0, 100)])
    old = r.choice([0, 1, 2, 5])
    e.write(SCENE + 0x508c, struct.pack('<2f', b, a)); e.set_u32(SCENE + 0x5080, old)
    e.set_u32(0x6ef1fc, CLOCK); e.write(CLOCK + 0xb98, struct.pack('<d', t))
    mu = e.mu
    mu.reg_write(UC_X86_REG_ESP, STACK_TOP - 0x3000); mu.reg_write(UC_X86_REG_EBP, SCENE); mu.reg_write(UC_X86_REG_FPCW, 0x27f)
    mu.emu_start(0x55186d, 0x551895, count=1000)
    rows.append(chr(9).join([struct.pack('<2f', b, a).hex(), struct.pack('<d', t).hex(), str(old), str(struct.unpack('<I', e.read(SCENE + 0x5080, 4))[0])]))
pathlib.Path(__file__).with_name('golden_scene_light_timeout.tsv').write_text(chr(10).join(rows) + chr(10))
print(len(rows), sum(1 for x in rows if x.endswith(chr(9) + '1')), 'set')

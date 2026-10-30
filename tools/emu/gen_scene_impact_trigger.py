"""Impact trigger of 0x54ff80 (0x54fff3..0x5501d7), entered with the registers the function has there."""
import pathlib, random, struct
from emu import Emu, OBJ_BASE, STACK_TOP
from unicorn.x86_const import UC_X86_REG_ESP, UC_X86_REG_EBP, UC_X86_REG_ESI, UC_X86_REG_EDI, UC_X86_REG_FPCW

r = random.Random(0x54fff3)
e = Emu()
for addr, v in ((0x702620, (0, 1, 0)), (0x702630, (0, 0, 1)), (0x702640, (1, 0, 0)), (0x702650, (0, 0, 0))):
    e.write(addr, struct.pack('<3f', *v))
SCENE = OBJ_BASE + 0x10000
rec = {'impact': None}


def impact(em):
    rec['impact'] = (e.read(em.arg(1), 12).hex(), e.read(em.arg(2), 12).hex())
    return 0


e.hook_ret(0x54ef90, impact)
F = lambda *v: struct.pack('<%df' % len(v), *v)
rows = []
for case in range(300):
    rec['impact'] = None
    e.write(SCENE, bytes(0x2b00))
    mat = F(*[r.uniform(-1.5, 1.5) for _ in range(12)])
    frame = F(*[r.uniform(-30, 30) for _ in range(6)])
    p = F(*[r.uniform(-3, 3) for _ in range(3)])
    vec = F(*[r.uniform(-1, 1) * r.choice([1.0, 0.3, 0.05]) for _ in range(3)])
    flag = r.choice([1, 1, 1, 0]); glob = r.choice([0, 0, 0, 1]); on = r.choice([1, 1, 1, 0]); busy = r.choice([0, 0, 0, 5])
    e.write(SCENE + 0x264, mat); e.write(SCENE + 0x1b0, frame[:12]); e.write(SCENE + 0x1bc, frame[12:24])
    e.write(SCENE + 0x2a3c, p); e.write(SCENE + 0x3e0, vec)
    e.set_u32(SCENE + 0x2a38, flag); e.set_u32(0x6ef200, glob); e.set_u32(SCENE + 0x38c, on); e.set_u32(SCENE + 0x414, busy)
    sp = STACK_TOP - 0x3000
    e.write(sp, bytes(0x200))
    mu = e.mu
    mu.reg_write(UC_X86_REG_ESP, sp); mu.reg_write(UC_X86_REG_EBP, SCENE); mu.reg_write(UC_X86_REG_ESI, 0); mu.reg_write(UC_X86_REG_EDI, 1)
    mu.reg_write(UC_X86_REG_FPCW, 0x27f)
    mu.emu_start(0x54fff3, 0x5501d7, count=100000)
    out = ','.join(rec['impact']) if rec['impact'] else '-'
    rows.append(chr(9).join([mat.hex(), frame.hex(), p.hex(), vec.hex(), str(flag), str(glob), str(on), str(busy), out.replace(',', ';')]))
pathlib.Path(__file__).with_name('golden_scene_impact_trigger.tsv').write_text(chr(10).join(rows) + chr(10))
print(len(rows), sum(1 for x in rows if not x.endswith('-')), 'impacts')

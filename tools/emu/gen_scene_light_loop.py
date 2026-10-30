"""Object-creation loop 0x550f4c..0x551094 of 0x54ff80 (three lights, each with a transform node), entered mid-function:
esp = frame with [F+0x2c] = light array, [F+0x44] = matrices, [F+0x8c] = count. malloc and the scene's virtual
registration call ([vtable+0x14], stubbed at a nop) are recorded."""
import pathlib, random, struct
from emu import Emu, OBJ_BASE, STACK_TOP
from unicorn.x86_const import UC_X86_REG_ESP, UC_X86_REG_EBP, UC_X86_REG_FPCW

r = random.Random(0x550f4c)
e = Emu()
HEAP = 0x2a800000
nxt = [HEAP]
calls = []


def malloc(em):
    a = nxt[0]
    nxt[0] += (em.arg(0) + 15) & ~15
    return a


STUB = 0x5abc3b
e.hook_ret(0x632643, malloc)
e.hook_thiscall(STUB, lambda em: calls.append((em.ecx, em.arg(0), em.arg(1))) or 0, 2)
e.write(0x7025f0, struct.pack('<12f', 1, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0))
P, REG, VT = OBJ_BASE + 0x20000, OBJ_BASE + 0x21000, OBJ_BASE + 0x22000
FRAME, ARR, MATS, SCENE = STACK_TOP - 0x5000, OBJ_BASE + 0x23000, OBJ_BASE + 0x24000, OBJ_BASE + 0x30000
e.set_u32(0x6ef254, P); e.set_u32(0x69622c, REG); e.set_u32(REG, VT); e.set_u32(VT + 0x14, STUB)
rows = []
for case in range(200):
    nxt[0] = HEAP; calls.clear()
    e.write(HEAP, bytes(0x800)); e.write(ARR, bytes(16)); e.write(FRAME, bytes(0x400))
    count = r.choice([1, 2, 3, 3, 3])
    pv = r.randrange(2 ** 32)
    e.set_u32(P + 0x24, pv)
    mats = b''.join(struct.pack('<12f', *[r.uniform(-5, 5) for _ in range(12)]) for _ in range(3))
    e.write(MATS, mats)
    e.set_u32(FRAME + 0x2c, ARR); e.set_u32(FRAME + 0x44, MATS); e.set_u32(FRAME + 0x8c, count)
    mu = e.mu
    mu.reg_write(UC_X86_REG_ESP, FRAME); mu.reg_write(UC_X86_REG_EBP, SCENE); mu.reg_write(UC_X86_REG_FPCW, 0x27f)
    mu.emu_start(0x550f4c, 0x551094, count=500000)
    slots = struct.pack('<3I', *[struct.unpack('<I', e.read(FRAME + o, 4))[0] for o in (0x2c, 0x44, 0x8c)])
    rows.append(chr(9).join([str(count), '%08x' % pv, mats.hex(), e.read(ARR, 16).hex(), e.read(HEAP, nxt[0] - HEAP).hex(), str(nxt[0] - HEAP),
                              slots.hex(), ','.join('%x/%x/%x' % c for c in calls),
                              bytes(e.read(0x67334c, 24)).split(bytes(1))[0].hex(), bytes(e.read(0x68d1c0, 24)).split(bytes(1))[0].hex()]))
pathlib.Path(__file__).with_name('golden_scene_light_loop.tsv').write_text(chr(10).join(rows) + chr(10))
print(len(rows), 'cases;', rows[0].split(chr(9))[7])

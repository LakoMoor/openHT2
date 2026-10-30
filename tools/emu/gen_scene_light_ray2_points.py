"""0x551094..0x551474 of 0x54ff80 (second set of rays, up to the first vec_set call), entered mid-function with a
random frame: ebp = scene, esp = frame, x87 empty. Stops at 0x551474 or, for a heading in range, at 0x551860 (dropped)."""
import pathlib, random, struct
from emu import Emu, OBJ_BASE, STACK_TOP
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_ESP, UC_X86_REG_EBP, UC_X86_REG_FPCW, UC_X86_REG_EIP

r = random.Random(0x551094)
e = Emu()
SCENE, WORLD = OBJ_BASE + 0x10000, OBJ_BASE + 0x40000
FRAME = STACK_TOP - 0x4000
FS, BELOW = 0x180, 0x40
TBL = 0x6d2548
rows = []
e.write(0x7025f0, struct.pack('<12f', 1, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0))
for a, v in ((0x702620, (0, 1, 0)), (0x702630, (0, 0, 1)), (0x702640, (1, 0, 0)), (0x702650, (0, 0, 0))):
    e.write(a, struct.pack('<3f', *v))
stops = {0x551474: 0, 0x551860: 1}
for case in range(2000):
    if len(rows) >= 300:
        break
    e.write(SCENE, bytes(0x2c00)); e.write(WORLD, bytes(0x700)); e.write(FRAME - BELOW, bytes(FS + BELOW))
    idx = r.randrange(0, 3)
    e.set_u32(SCENE + 0x28b4, idx)
    e.write(TBL + 0x5d0 * idx, struct.pack('<12f', *[r.uniform(-4, 4) for _ in range(12)]))
    e.set_u32(0x6d2078, WORLD)
    e.write(WORLD + 0x5b4, struct.pack('<3f', *[r.uniform(-60, 60) for _ in range(3)]))
    frame = b''.join(struct.pack('<f', r.uniform(-3, 3)) for _ in range(FS // 4))
    e.write(FRAME, frame)
    mu = e.mu
    mu.reg_write(UC_X86_REG_ESP, FRAME); mu.reg_write(UC_X86_REG_EBP, SCENE); mu.reg_write(UC_X86_REG_FPCW, 0x27f)
    hit = {}

    def stop(uc, addr, size, user):
        if addr in stops:
            hit['a'] = addr
            uc.emu_stop()
    h = mu.hook_add(UC_HOOK_CODE, stop, begin=0x551474, end=0x551860)
    mu.emu_start(0x551094, 0x551866, count=200000)
    mu.hook_del(h)
    take = stops.get(hit.get('a', mu.reg_read(UC_X86_REG_EIP)))
    if take != 0:
        continue
    rows.append(chr(9).join([str(idx), e.read(TBL + 0x5d0 * idx, 48).hex(), e.read(WORLD + 0x5b4, 12).hex(), frame.hex(),
                              hex(FRAME), e.read(FRAME - BELOW, FS + BELOW).hex()]))
pathlib.Path(__file__).with_name('golden_scene_light_ray2_points.tsv').write_text(chr(10).join(rows) + chr(10))
print(len(rows), 'cases')

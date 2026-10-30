"""0x550735..0x550a42 of 0x54ff80 (light-ray set-up) entered mid-function: ebp = scene, esp = frame, x87 empty;
stops after the matrix products and the loop set-up (0x550f4c); cases that leave at 0x551090 are dropped."""
import math, pathlib, random, struct
from emu import Emu, OBJ_BASE, STACK_TOP
from unicorn.x86_const import UC_X86_REG_ESP, UC_X86_REG_EBP, UC_X86_REG_FPCW, UC_X86_REG_FP0, UC_X86_REG_FPSW, UC_X86_REG_EIP

r = random.Random(0x550735)
e = Emu()
SCENE, WORLD = OBJ_BASE + 0x10000, OBJ_BASE + 0x40000
CLOCK = OBJ_BASE + 0x50000
FRAME = STACK_TOP - 0x4000
FS = 0x300
TBL = 0x6d2530
rows = []


def sti(mu, i):
    top = (mu.reg_read(UC_X86_REG_FPSW) >> 11) & 7
    mant, exp = mu.reg_read(UC_X86_REG_FP0 + ((top + i) & 7))
    return st((mant, exp))


def st(v):
    mant, exp = v
    sign = -1.0 if exp & 0x8000 else 1.0
    exp &= 0x7fff
    return sign * mant / 2 ** 63 * 2.0 ** (exp - 16383)


def rot(a, b, c):
    ca, sa, cb, sb, cc, sc = math.cos(a), math.sin(a), math.cos(b), math.sin(b), math.cos(c), math.sin(c)
    return [cb * cc, cb * sc, -sb, sa * sb * cc - ca * sc, sa * sb * sc + ca * cc, sa * cb, ca * sb * cc + sa * sc, ca * sb * sc - sa * cc, ca * cb]


def xf(pos):
    return rot(r.uniform(-3, 3), r.uniform(-1.5, 1.5), r.uniform(-3, 3)) + pos


stops = {0x550f4c: 0, 0x551090: 1}
e.write(0x7025f0, struct.pack('<12f', 1, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0))
for a, v in ((0x702620, (0, 1, 0)), (0x702630, (0, 0, 1)), (0x702640, (1, 0, 0)), (0x702650, (0, 0, 0))):
    e.write(a, struct.pack('<3f', *v))
for case in range(1200):
    if len(rows) >= 300:
        break
    pos = lambda: [r.uniform(-50, 50) for _ in range(3)]
    e.write(SCENE, bytes(0x5100)); e.write(WORLD, bytes(0x700)); e.write(FRAME, bytes(FS))
    e.set_u32(0x6974c4, CLOCK); e.write(0x6974c2, struct.pack('<h', r.choice([0, 0, 1])))
    e.write(0x6974bc, struct.pack('<f', r.uniform(0, 500)))
    e.write(CLOCK + 0xb98, struct.pack('<d', r.choice([r.uniform(0, 1e4), r.uniform(0, 5)])))
    e.write(SCENE + 0x501c, struct.pack('<f', r.choice([0.0, r.uniform(0, 1)])))
    e.write(SCENE + 0x204, struct.pack('<12f', *xf(pos()))); e.write(SCENE + 0x2c4, struct.pack('<12f', *xf(pos())))
    idx = r.randrange(0, 3)
    e.set_u32(SCENE + 0x28b4, idx)
    e.write(TBL - 0xc + 0x5d0 * idx, struct.pack('<18f', *[r.uniform(-4, 4) for _ in range(18)]))
    e.set_u32(0x6d2078, WORLD)
    e.write(0x6974a8, struct.pack('<3f', *[r.uniform(-1, 1) for _ in range(3)]))
    cam = [r.uniform(-60, 60) for _ in range(3)]
    e.write(WORLD + 0x5b4, struct.pack('<3f', *cam))
    frame = bytes(r.randrange(256) for _ in range(FS)) if False else b''.join(struct.pack('<f', r.uniform(-3, 3)) for _ in range(FS // 4))
    e.write(FRAME, frame)
    mu = e.mu
    sp = FRAME
    mu.reg_write(UC_X86_REG_ESP, sp); mu.reg_write(UC_X86_REG_EBP, SCENE); mu.reg_write(UC_X86_REG_FPCW, 0x27f)
    mu.mem_write(sp - 4, struct.pack('<I', 0))
    from unicorn import UcError
    hit = {}
    hooks = []
    from unicorn import UC_HOOK_CODE
    def stop(uc, addr, size, user):
        if addr in stops:
            hit['a'] = addr
            uc.emu_stop()
    h = mu.hook_add(UC_HOOK_CODE, stop, begin=0x550f4c, end=0x550f4c)
    mu.emu_start(0x550735, 0x551090, count=200000)
    mu.hook_del(h)
    eip = mu.reg_read(UC_X86_REG_EIP)
    take = stops.get(hit.get('a', eip))
    if take != 0:
        continue
    head = 0.0
    out = e.read(FRAME, FS)
    sc = e.read(SCENE + 0x5020, 12)
    rows.append(chr(9).join([struct.pack('<d', e.mu and struct.unpack('<d', e.read(CLOCK + 0xb98, 8))[0]).hex(),
                              e.read(0x6974c2, 2).hex() + e.read(0x6974bc, 4).hex(), e.read(SCENE + 0x501c, 4).hex(),
                              e.read(SCENE + 0x204, 0xf0).hex(), str(idx), e.read(TBL - 0xc + 0x5d0 * idx, 72).hex(), e.read(WORLD + 0x5b4, 12).hex(), e.read(0x6974a8, 12).hex(),
                              frame.hex(), str(take), hex(FRAME), out.hex(), sc.hex()]))
pathlib.Path(__file__).with_name('golden_scene_light_ray_matrices.tsv').write_text(chr(10).join(rows) + chr(10))
print(len(rows), "cases")

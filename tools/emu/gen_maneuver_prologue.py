"""Prologue of 0x54f040 (maneuver chooser) entered mid-function at 0x54f052: ebx = scene, esp = frame F, x87 empty.
Stops at 0x54f2fc (gates passed, before the list lookup 0x53f780), 0x54f98a / 0x54f98c (early return 0).
Columns: inputs (scene bytes 0x5070..0x50a0 and 0x180..0x1e0 / 0x230..0x264 / 0x390 / 0x414 / 0x4c4, the object, clock,
globals, frame) and outputs (stop, frame, scene words)."""
import pathlib, random, struct
from emu import Emu, OBJ_BASE, STACK_TOP
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_ESP, UC_X86_REG_EBP, UC_X86_REG_EBX, UC_X86_REG_FPCW, UC_X86_REG_EIP, UC_X86_REG_ECX

r = random.Random(0x54f040)
e = Emu()
SCENE, CLOCK, GLB = OBJ_BASE + 0x10000, OBJ_BASE + 0x30000, OBJ_BASE + 0x31000
OBJS = OBJ_BASE + 0x40000
FRAME = STACK_TOP - 0x4000
FS = 0x200
rows = []
stops = {0x54f2fc: 0, 0x54f98a: 1, 0x54f98c: 2}


def fl(lo, hi):
    return struct.pack('<f', r.uniform(lo, hi))


def vec(lo, hi):
    return b''.join(fl(lo, hi) for _ in range(3))


stats = [0, 0, 0]
for case in range(3000):
    if len(rows) >= 180:
        break
    e.write(SCENE, bytes(0x5100)); e.write(CLOCK, bytes(0xc00)); e.write(GLB, bytes(0x600))
    n = r.randrange(0, 3)
    objs = [OBJS + 0x1000 * (i + 1) for i in range(n)]
    for o in objs:
        e.write(o, bytes(0x400))
        e.write(o + 0x1a0, vec(-60, 60)); e.write(o + 0x1c4, vec(-60, 60)); e.write(o + 0x224, b''.join(fl(-2, 2) for _ in range(12)))
    for i, o in enumerate(objs):
        e.set_u32(o + 0x178 + 8, objs[i + 1] + 0x178 + 4 if i + 1 < n else 0)
    e.set_u32(0x6f653c, objs[0] + 0x178 + 4 if n else 0)
    gate = r.random()
    e.set_u32(SCENE + 0x4c4, 0 if gate < 0.05 else 1)
    e.set_u32(SCENE + 0x390, 0 if r.random() < 0.05 else 1)
    e.set_u32(0x6cec6c, GLB)
    e.set_u32(GLB + 0x514, r.choice([0, 1, 1, 2]) if r.random() < 0.9 else 0)
    e.set_u32(SCENE + 0x184, 0 if r.random() < 0.9 else 1)
    pick = r.random()
    ident = (objs[r.randrange(n)] if n and pick < 0.9 else r.randrange(0, 2 ** 32))
    e.set_u32(SCENE + 0x414, ident)
    e.set_u32(SCENE + 0x507c, r.choice([0, 0, 1]))
    e.write(SCENE + 0x5084, fl(0, 50)); e.write(SCENE + 0x5088, fl(0, 12))
    for a in (0x5074, 0x5094, 0x5098, 0x5078):
        e.set_u32(SCENE + a, r.randrange(1, 100))
    e.set_u32(0x6ef1fc, CLOCK)
    e.write(CLOCK + 0xb98, struct.pack('<d', r.uniform(0, 60)))
    e.write(SCENE + 0x1d4, vec(-60, 60))
    e.write(SCENE + 0x234, b''.join(fl(-1, 1) for _ in range(12)))
    e.write(0x702620, vec(-1, 1))
    frame = b''.join(struct.pack('<f', r.uniform(-3, 3)) for _ in range(FS // 4))
    e.write(FRAME - 0x100, bytes(0x100)); e.write(FRAME, frame)
    AREAS = [(0x180, 0x264), (0x390, 0x398), (0x410, 0x418), (0x4c0, 0x4c8), (0x5070, 0x50a0)]
    inp = (b''.join(e.read(SCENE + a, b - a) for a, b in AREAS), e.read(CLOCK + 0xb98, 8), e.read(0x702620, 12), e.read(0x6cec6c, 4), e.read(GLB + 0x514, 4),
           e.read(0x6f653c, 4), b''.join(e.read(o + 0x170, 0xf0) for o in objs))
    mu = e.mu
    mu.reg_write(UC_X86_REG_ESP, FRAME); mu.reg_write(UC_X86_REG_EBP, FRAME + 0x300); mu.reg_write(UC_X86_REG_EBX, SCENE)
    mu.reg_write(UC_X86_REG_FPCW, 0x27f)
    hit = {}

    def stop(uc, addr, size, user):
        if addr in stops:
            hit['a'] = addr
            uc.emu_stop()
    hs = [mu.hook_add(UC_HOOK_CODE, stop, begin=a, end=a) for a in stops]
    mu.emu_start(0x54f052, 0x54fa00, count=200000)
    for h in hs:
        mu.hook_del(h)
    if 'a' not in hit.keys() and 'a' not in hit:
        continue
    st = stops[hit['a']]
    stats[st] += 1
    if stats[st] > (60, 60, 60)[st]:
        continue
    out = e.read(FRAME, FS)
    rows.append(chr(9).join([str(st), frame.hex(), inp[0].hex(), inp[1].hex(), inp[2].hex(), inp[3].hex(), inp[4].hex(), inp[5].hex(), inp[6].hex(),
                              '%x' % ident, out.hex(), e.read(SCENE + 0x5070, 0x30).hex()]))
pathlib.Path(__file__).with_name('golden_maneuver_prologue.tsv').write_text(chr(10).join(rows) + chr(10))
print(len(rows), 'cases', stats)

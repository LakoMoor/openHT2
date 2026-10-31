"""Whole light-ray block 0x5506d4..0x551866 of 0x54ff80 (conditions, both ray sets, both creation loops), entered
mid-function with a random frame; malloc and the scene registration call are recorded. Stops at 0x551866."""
import math, pathlib, random, struct
from emu import Emu, OBJ_BASE, STACK_TOP
from unicorn.x86_const import UC_X86_REG_ESP, UC_X86_REG_EBP, UC_X86_REG_FPCW

r = random.Random(0x5506d4)
e = Emu()
HEAP = 0x2a800000
nxt = [HEAP]
calls = []
STUB = 0x5abc3b


def malloc(em):
    a = nxt[0]
    nxt[0] += (em.arg(0) + 15) & ~15
    return a


e.hook_ret(0x632643, malloc)
e.hook_thiscall(STUB, lambda em: calls.append((em.ecx, em.arg(0), em.arg(1))) or 0, 2)
e.write(0x7025f0, struct.pack('<12f', 1, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0))
for a, v in ((0x702620, (0, 1, 0)), (0x702630, (0, 0, 1)), (0x702640, (1, 0, 0)), (0x702650, (0, 0, 0))):
    e.write(a, struct.pack('<3f', *v))
SCENE, WORLD, CLOCK, REC, FLAG = OBJ_BASE + 0x10000, OBJ_BASE + 0x40000, OBJ_BASE + 0x50000, OBJ_BASE + 0x30000, OBJ_BASE + 0x38000
P1, P2, REG, VT = OBJ_BASE + 0x60000, OBJ_BASE + 0x61000, OBJ_BASE + 0x62000, OBJ_BASE + 0x63000
FRAME, FS, BELOW = STACK_TOP - 0x6000, 0x400, 0x40
TBL = 0x6d2518


def rot(a, b, c):
    ca, sa, cb, sb, cc, sc = math.cos(a), math.sin(a), math.cos(b), math.sin(b), math.cos(c), math.sin(c)
    return [cb * cc, cb * sc, -sb, sa * sb * cc - ca * sc, sa * sb * sc + ca * cc, sa * cb, ca * sb * cc + sa * sc, ca * sb * sc - sa * cc, ca * cb]


def xf():
    return rot(r.uniform(-3, 3), r.uniform(-1.5, 1.5), r.uniform(-3, 3)) + [r.uniform(-50, 50) for _ in range(3)]


e.set_u32(0x6d2078, WORLD); e.set_u32(0x6974c4, CLOCK); e.set_u32(0x69622c, REG); e.set_u32(REG, VT); e.set_u32(VT + 0x14, STUB)
e.write(0x6974c2, struct.pack('<h', 0))
rows = []
kept = 0
for case in range(900):
    nxt[0] = HEAP; calls.clear()
    e.write(HEAP, bytes(0x1000)); e.write(SCENE, bytes(0x5600)); e.write(FRAME - BELOW, bytes(FS + BELOW)); e.write(WORLD, bytes(0x700))
    e.write(REC, bytes(0xc0)); e.write(FLAG, bytes(0x20)); e.write(TBL, bytes(0x5d0 * 3 + 0x100))
    ok_all = r.random() < 0.85
    e.set_u32(SCENE + 0x51bc, 1 if (ok_all or r.random() < 0.5) else 0)
    idx = r.randrange(0, 3)
    e.set_u32(SCENE + 0x28b4, idx)
    e.write(TBL + 0x5d0 * idx, struct.pack('<%df' % 0x174, *[r.uniform(-4, 4) if k % 7 else 0.5 for k in range(0x174)]))
    e.set_u32(0x6ef254, P1 if (ok_all or r.random() < 0.7) else 0); e.set_u32(0x6ef258, P2 if (ok_all or r.random() < 0.7) else 0)
    e.set_u32(P1 + 0x24, r.randrange(2 ** 32)); e.set_u32(P2 + 0x24, r.randrange(2 ** 32))
    e.set_u32(SCENE + 0x5080, r.choice([1, 5, 5, 5, 5, 5, 0]))
    e.write(SCENE + 0x224, struct.pack('<f', r.choice([1.0, 1.0, 0.7, 0.2, 0.0])))
    e.set_u32(SCENE + 0x5440 + 0x14, FLAG); e.set_u32(FLAG + 0xc, r.choice([0] * 9 + [1]))
    e.set_u32(SCENE + 0x2540, REC)
    for off in (4, 0x18, 0x2c, 0x40, 0x54, 0x68, 0x7c, 0x90, 0xa4):
        b = r.choice([1, 7, 100, 10000])
        e.write(REC + off, struct.pack('<ii', r.choice([0, 0, 0, 0, 0, 0, r.randrange(-b, b + 1)]), b))
    e.write(CLOCK + 0xb98, struct.pack('<d', r.uniform(0, 5)))
    e.write(SCENE + 0x501c, struct.pack('<f', r.uniform(0, 1)))
    e.write(SCENE + 0x204, struct.pack('<12f', *xf())); e.write(SCENE + 0x2c4, struct.pack('<12f', *xf()))
    e.write(WORLD + 0x5b4, struct.pack('<3f', *[r.uniform(-60, 60) for _ in range(3)]))
    e.write(0x6974a8, struct.pack('<3f', *[r.uniform(-1, 1) for _ in range(3)]))
    frame = b''.join(struct.pack('<f', r.uniform(-3, 3)) for _ in range(FS // 4))
    e.write(FRAME, frame)
    REG_AREAS = [(0x200, 0x2f8), (0x28b0, 0x28b8), (0x2540, 0x2548), (0x5000, 0x5100), (0x51b8, 0x51c0), (0x5440, 0x5460)]
    areas = lambda: b''.join(e.read(SCENE + a, b - a) for a, b in REG_AREAS)
    before = dict(areas=areas(), tbl=e.read(TBL + 0x5d0 * idx, 0x5d0), world=e.read(WORLD + 0x5b4, 12), p=e.read(P1 + 0x24, 4) + e.read(P2 + 0x24, 4),
                  vec=e.read(0x6974a8, 12), rec=e.read(REC, 0xc0), clock=e.read(CLOCK + 0xb98, 8), flag=e.read(FLAG + 0xc, 4),
                  ptrs=struct.pack('<2I', 1 if e.read(0x6ef254, 4) != bytes(4) else 0, 1 if e.read(0x6ef258, 4) != bytes(4) else 0))
    mu = e.mu
    mu.reg_write(UC_X86_REG_ESP, FRAME); mu.reg_write(UC_X86_REG_EBP, SCENE); mu.reg_write(UC_X86_REG_FPCW, 0x27f)
    mu.emu_start(0x5506d4, 0x551866, count=3000000)
    row = chr(9).join([hex(FRAME), str(idx), frame.hex()] + [before[k].hex() for k in ('areas', 'tbl', 'world', 'p', 'vec', 'rec', 'clock', 'flag', 'ptrs')] +
                      [areas().hex(), e.read(FRAME, FS).hex(), e.read(HEAP, nxt[0] - HEAP).hex(), str(nxt[0] - HEAP), ','.join('%x/%x/%x' % c for c in calls),
                      bytes(e.read(0x67334c, 24)).split(bytes(1))[0].hex(), bytes(e.read(0x68d1c0, 24)).split(bytes(1))[0].hex()])
    (rows if (calls or len(rows) - kept < 60) else None) is not None and rows.append(row)
    if calls:
        kept += 1
pathlib.Path(__file__).with_name('golden_scene_light_rays.tsv').write_text(chr(10).join(rows) + chr(10))
print(len(rows), 'cases;', kept, 'with registrations')

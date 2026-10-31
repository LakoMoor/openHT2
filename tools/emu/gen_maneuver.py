"""The whole maneuver chooser 0x54f040 (ecx = scene) run unmodified on random scenes, with the road module calls
replaced by recording stubs: 0x533340 (zone object), 0x445c90 (opposite direction), 0x438d70 (road parameters),
0x439580 (coordinates), 0x439320 (tangent), 0x533510 (one-way test). Each stub draws its outputs from the generator
and records `kind|args|written bytes|result`; the Rust test replays the records in order.

Row: scene (sparse), vehicles (sparse), tables, calls, checkpoints `addr:frame` (frame = esp after the pushes, 0x200
bytes, first time each address in CK is reached), result, final scene words, final vehicle words."""
import pathlib, random, struct, sys
from emu import Emu, OBJ_BASE, STACK_TOP
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_ESP

SEED = int(sys.argv[1], 0) if len(sys.argv) > 1 else 0x54f040
r = random.Random(SEED)
e = Emu()
SCENE, CLOCK, GLB = OBJ_BASE + 0x10000, OBJ_BASE + 0x30000, OBJ_BASE + 0x31000
ROOMS, NODES, TABLE = OBJ_BASE + 0x32000, OBJ_BASE + 0x33000, OBJ_BASE + 0x34000
NAMES = OBJ_BASE + 0x35000
VEH = OBJ_BASE + 0x40000     # vehicle i at VEH + 0x2000 * i
VSIZE = 0x1000
CK = [int(x, 16) for x in sys.argv[2].split(',')] if len(sys.argv) > 2 else [0x54f41e, 0x54fdcd]
COUNT = int(sys.argv[3]) if len(sys.argv) > 3 else 150
OUT = sys.argv[4] if len(sys.argv) > 4 else 'golden_maneuver.tsv'
FS = 0x200


def fl(lo, hi):
    return struct.pack('<f', r.uniform(lo, hi))


def vec(lo, hi):
    return b''.join(fl(lo, hi) for _ in range(3))


def sparse(b):
    out = []
    for i in range(0, len(b), 16):
        c = b[i:i + 16]
        if any(c):
            out.append('%x:%s' % (i, c.hex()))
    return ','.join(out)


calls = []
COORD_SCALE = 6
ALIGN = False


def rnd_pid():
    return struct.pack('<iidiiii', r.randrange(0, 4), r.choice([-1, 0, 1, 2]), r.uniform(0, 1), r.randrange(0, 3), r.randrange(0, 3),
                       r.choice([0, 0, 0x10, 0x30, 0x20, 4, 8]), r.randrange(0, 2))


def pid_args(emu, first):
    return [emu.arg(first + i) for i in range(8)]


def h_zone(emu):
    a = [emu.arg(i) for i in range(5)]
    obj = r.choice([0, 0x1000 + r.randrange(1, 9)]) if r.random() < 0.8 else 0x1234
    pid = rnd_pid()
    e.set_u32(a[2], obj); e.write(a[3], pid)
    ret = r.choice([0, 1, 2])
    calls.append('zone|%s|%08x%s|%d' % (','.join('%x' % x for x in a), obj, pid.hex(), ret))
    return ret


def h_opp(emu):
    out, pid = emu.arg(0), pid_args(emu, 1)
    nw = rnd_pid()
    e.write(out, nw)
    calls.append('opp|%s|%s|%x' % (','.join('%x' % x for x in [out] + pid), nw.hex(), out))
    return out


def h_params(emu):
    pid, out = pid_args(emu, 0), emu.arg(8)
    w = bytearray(0x30)
    w[0] = r.choice([0, 0, 0, 0, 0, 0x10, 0x20, 0x30, 4, 8])                 # flags (bits 0x30 block the manoeuvre)
    width = r.choice([0.0, 1.0, 2.0, 4.0, 8.0, 12.0, 3.5]) if r.random() < 0.9 else r.uniform(0, 12)
    struct.pack_into('<d', w, 8, width)
    struct.pack_into('<I', w, 0x20, r.choice([0, 1, 2, 3]))                  # out[8]: lanes
    e.write(out, bytes(w))
    calls.append('params|%s|%s|0' % (','.join('%x' % x for x in pid + [out]), bytes(w).hex()))
    return 0


def h_coord(emu):
    out, pid = emu.arg(0), pid_args(emu, 1)
    w = vec(-80, 80)
    if out == frame_base.get('f', 0) + 0x4c:
        sp = struct.unpack('<3f', e.read(SCENE + 0x258, 12))
        w = struct.pack('<3f', *[x + r.uniform(-1, 1) * COORD_SCALE for x in sp])
    e.write(out, w)
    calls.append('coord|%s|%s|0' % (','.join('%x' % x for x in [out] + pid), w.hex()))
    return 0


def h_tan(emu):
    out, pid = emu.arg(0), pid_args(emu, 1)
    w = vec(-1, 1)
    if ALIGN and out == frame_base.get('f', 0) + 0x20:
        p = struct.unpack('<3f', e.read(frame_base['f'] + 0x58, 12))
        k = r.uniform(0.5, 3)
        w = struct.pack('<3f', *[x * k + r.uniform(-0.05, 0.05) for x in p])
    e.write(out, w)
    calls.append('tan|%s|%s|%x' % (','.join('%x' % x for x in [out] + pid), w.hex(), out))
    return out


def h_one(emu):
    p = emu.arg(0)
    pid = e.read(p, 32)
    ret = r.randrange(2)
    calls.append('one|%x|%s|%d' % (p, pid.hex(), ret))
    return ret


e.hook_ret(0x533340, h_zone)
e.hook_ret(0x445c90, h_opp)
e.hook_ret(0x438d70, h_params)
e.hook_ret(0x439580, h_coord)
e.hook_ret(0x439320, h_tan)
e.hook_ret(0x533510, h_one)

rows = []
stats = {}
COV = set()
import os
CASE_PCS = set()
e.mu.hook_add(UC_HOOK_CODE, lambda uc, a, sz, u: CASE_PCS.add(a), begin=0x54f040, end=0x54ff70)
for case in range(COUNT * 60):
    if len(rows) >= COUNT:
        break
    calls.clear()
    CASE_PCS.clear()
    ALIGN = r.random() < 0.6
    if ALIGN:
        pass
    COORD_SCALE = r.choice([0.8, 6, 6])
    e.write(SCENE, bytes(0x5100)); e.write(CLOCK, bytes(0xc00)); e.write(GLB, bytes(0x600))
    nveh = 8
    for i in range(nveh):
        v = VEH + 0x2000 * i
        e.write(v, bytes(VSIZE))
        e.write(v + 0x160, b''.join(fl(-60, 60) for _ in range(0xe0 // 4)))     # 0x1a0.., 0x1c4.., transforms
        if r.random() < 0.3:
            p = struct.unpack('<3f', e.read(v + 0x10 + 0x1c4, 12))
            e.write(v + 0x10 + 0x1a0, struct.pack('<3f', p[0] + r.uniform(1.9, 3.3), p[1], p[2]))
        e.write(v + 0x1f4 + 0x10, b''.join(fl(-2, 2) for _ in range(12)))
        e.write(v + 0x224 + 0x10 - 0x10, b''.join(fl(-2, 2) for _ in range(12)))
        e.set_u32(v + 0x188 + 0x10, 2 if r.random() < 0.7 else r.randrange(0, 4))
        e.set_u32(v + 0xdf0 + 0x10, 2 if r.random() < 0.7 else r.randrange(0, 4))
        e.set_u32(v + 0x334, NAMES)
        e.write(v + 0xcc4 + 0x10, fl(0, 400))
        e.set_u32(v + 0x5a0 + 0x10, r.randrange(2))
        e.write(v + 0x700 + 0x10, b''.join(fl(-2, 2) for _ in range(12)))
        n = r.randrange(0, 4)
        e.set_u32(v + 0xdf8 + 0x10, n)
        e.set_u32(v + 0xe00 + 0x10, r.randrange(0, 3))
        for k in range(n):
            e.write(v + 0xef8 + 0x10 + 12 * k, vec(-5, 5))
            e.set_u32(v + 0xf28 + 0x10 + 4 * k, r.randrange(2))
    # vehicle i: base V = VEH + 0x2000 * i (list B element), object X = V + 0x10 (list A element at X + 0x178)
    for i in range(nveh):
        V = VEH + 0x2000 * i
        X = V + 0x10
        e.set_u32(X + 0x178 + 8, VEH + 0x2000 * (i + 1) + 0x10 + 0x178 + 4 if i + 1 < nveh else 0)
        e.set_u32(V + 8, VEH + 0x2000 * (i + 1) + 4 if i + 1 < nveh else 0)
    e.set_u32(0x6f653c, VEH + 0x10 + 0x178 + 4)
    e.set_u32(0x6ef1f8, 0 if r.random() < 0.04 else VEH + 4)
    X0 = VEH + 0x10
    e.set_u32(SCENE + 0x414, X0 if r.random() < 0.95 else VEH + 0x2000 * r.randrange(1, 8) + 0x10)
    e.set_u32(SCENE + 0x4c4, 1); e.set_u32(SCENE + 0x390, 1)
    e.set_u32(0x6cec6c, GLB); e.set_u32(GLB + 0x514, r.choice([0, 1]))
    e.set_u32(SCENE + 0x184, 0)
    e.set_u32(SCENE + 0x507c, 0)
    e.write(SCENE + 0x5084, fl(0, 50)); e.write(SCENE + 0x5088, fl(0, 12))
    for a in (0x5074, 0x5094, 0x5098, 0x5078):
        e.set_u32(SCENE + a, r.randrange(1, 100))
    e.set_u32(0x6ef1fc, CLOCK)
    e.write(CLOCK + 0xb98, struct.pack('<d', r.uniform(0, 60)))
    e.write(SCENE + 0x1b0, vec(-30, 30))
    e.write(SCENE + 0x1d4, vec(-0.4, 0.4) if r.random() < 0.4 else vec(-30, 30))
    e.write(SCENE + 0x228, vec(-30, 30))
    e.write(SCENE + 0x234, b''.join(fl(-1, 1) for _ in range(12)))
    e.write(SCENE + 0x258, vec(-60, 60))
    e.write(SCENE + 0x294, b''.join(fl(-1, 1) for _ in range(12)))
    e.set_u32(SCENE + 0x334, NAMES)
    for k in range(6):
        e.set_u32(SCENE + 0xdc4 + 4 * k, 0 if r.random() < 0.3 else VEH + 0x2000 * r.randrange(0, 8) + 0x10)
        e.write(SCENE + 0xddc + 4 * k, fl(0, 300))
    n = r.randrange(0, 5)
    e.set_u32(SCENE + 0x10 + 0xdf8, n)
    e.set_u32(SCENE + 0x10 + 0xe00, r.randrange(0, 2))
    for k in range(n):
        e.write(SCENE + 0x10 + 0xef8 + 12 * k, vec(-6, 6))
        e.set_u32(SCENE + 0x10 + 0xf28 + 4 * k, r.randrange(2) if r.random() < 0.8 else 1)
    e.write(0x702620, vec(-1, 1)); e.write(0x702630, vec(-1, 1)); e.write(0x702650, vec(-1, 1))
    if ALIGN:
        e.write(0x702620, struct.pack('<3f', 1, 0, 0))
        e.write(SCENE + 0x234, struct.pack('<12f', *[(1.0 if i in (0, 4, 8) else 0.0) + r.uniform(-0.15, 0.15) for i in range(9)], 0, 0, 0))
    # the room table
    e.set_u32(0x68cf38, TABLE); e.set_u32(0x68cf3c, TABLE + 16)
    for k in range(4):
        e.set_u32(TABLE + 4 * k, ROOMS + 0x100 * k)
        e.write(ROOMS + 0x100 * k, bytes(0x100))
        e.set_u32(ROOMS + 0x100 * k + 0x70, NODES + 0x20 * (2 * k)); e.set_u32(ROOMS + 0x100 * k + 0x74, NODES + 0x20 * (2 * k + 1))
        e.write(NODES + 0x20 * (2 * k) + 0x10, struct.pack('<h', r.randrange(0, 2)))
        e.write(NODES + 0x20 * (2 * k + 1) + 0x10, struct.pack('<h', r.randrange(0, 2)))
    e.write(NAMES, bytes(0x20))
    snap = (sparse(e.read(SCENE, 0x5100)), sparse(e.read(VEH, 0x2000 * nveh - 0x1000)) if False else
            '|'.join('%d@%s' % (i, sparse(e.read(VEH + 0x2000 * i, VSIZE))) for i in range(nveh)),
            sparse(e.read(ROOMS, 0x400)) + '/' + sparse(e.read(NODES, 0x100)),
            e.read(CLOCK + 0xb98, 8).hex() + '/' + e.read(0x702620, 12).hex() + '/' + e.read(0x702630, 12).hex() + '/' + e.read(0x702650, 12).hex() + '/' + e.read(0x6ef1f8, 4).hex())
    mu = e.mu
    cps = {}
    frame_base = {}

    def hook(uc, addr, size, user):
        if addr == 0x54f052:
            frame_base['f'] = uc.reg_read(UC_X86_REG_ESP)
        elif addr in CK and addr not in cps:
            cps[addr] = bytes(uc.mem_read(frame_base['f'], FS)).hex()
    hs = [mu.hook_add(UC_HOOK_CODE, hook, begin=a, end=a) for a in [0x54f052] + CK]
    try:
        e.call(0x54f040, [], '', this=SCENE, max_insn=2_000_000)
    except Exception as ex:
        from unicorn.x86_const import UC_X86_REG_EIP
        print('fault', ex, hex(mu.reg_read(UC_X86_REG_EIP)), calls[-2:])
        raise
    finally:
        for h in hs:
            mu.hook_del(h)
    res = e.eax
    new = CASE_PCS - COV
    if not new and r.random() > 0.03:
        continue
    COV.update(CASE_PCS)
    key = (res - X0) if res else None
    stats[key] = stats.get(key, 0) + 1
    rows.append(chr(9).join(list(snap) + [';'.join(calls), '%x' % frame_base.get('f', 0), ';'.join('%x:%s' % (a, b) for a, b in cps.items()), '%x' % res,
                                          e.read(SCENE + 0x5070, 0x30).hex()]))
(pathlib.Path(OUT) if '/' in OUT else pathlib.Path(__file__).with_name(OUT)).write_text(chr(10).join(rows) + chr(10))
print(len(rows), 'cases', stats)
if COV:
    open(os.path.join(os.environ['TEMP'], 'cov.txt'), 'w').write(' '.join('%x' % a for a in sorted(COV)))

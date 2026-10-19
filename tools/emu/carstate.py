"""Random but valid vehicle state for the differential tests of the engine and gearbox."""
import struct

from harness import pack, OBJ_BASE, HEAP_BASE

CAR = OBJ_BASE + 0x1000          # the _Car_V object
CAR_LEN = 0x4000
TECH_TABLE = 0x697868
IDS = [0, 5, 8, 18, 19, 21, 25, 31, 33, 38]
HEAP_WATCH = (HEAP_BASE, 0x800)


def tech_writes(emu, vid):
    """The parts of `vehicle.tech` record `vid` the engine code reads, as writes."""
    base = TECH_TABLE + vid * 0x970
    return [(base + 96, emu.read(base + 96, 184))]


def ts_writes(r, base, level=None):
    w = []
    for i in range(3):
        a = r.uniform(0, 15)
        b = a + r.choice([-1, 1]) * r.uniform(0.5, 25)
        w += [(base + i * 4, pack('f', a)), (base + 0xc + i * 4, pack('f', b)),
              (base + 0x18 + i * 4, pack('f', r.choice([0.0, r.uniform(0.01, 5)])))]
    w += [(base + 0x24, pack('f', r.uniform(-0.1, 1.2))), (base + 0x28, pack('i', r.randint(0, 2))),
          (base + 0x2c, pack('i', 3)), (base + 0x30, pack('i', r.randint(0, 2)))]
    return w


def car_writes(emu, r, vid=None, gear=None):
    """Random engine/wheel state. Returns (writes, vehicle id, wheel count)."""
    vid = r.choice(IDS) if vid is None else vid
    n = r.randint(1, 6)
    ng = struct.unpack('<i', emu.read(TECH_TABLE + vid * 0x970 + 144, 4))[0]
    w = tech_writes(emu, vid)
    f = lambda off, lo, hi: w.append((CAR + off, pack('f', r.uniform(lo, hi))))
    i32 = lambda off, v: w.append((CAR + off, pack('i', v)))
    w.append((CAR + 0x8cc, pack('i', vid)))
    i32(0xdd4, n)
    for i in range(n):
        w.append((CAR + 0x19a4 + 2 * i, pack('h', r.choice([0, 0, 1, -1]))))
        i32(0x19d4 + 4 * i, r.choice([0, 1, 1]))
        i32(0x1bb4 + 4 * i, r.choice([0, 1, 1, 1]))
        f(0x2670 + 4 * i, 0, 1.3)
        f(0xd20 + 4 * i, 0, 3)
    f(0x1b70, 0.3, 2.0)
    f(0xdc8, -5, 45)
    for g in range(15):
        f(0x1c44 + 4 * g, 0.4, 3)
    f(0x1c88, 80, 320)
    f(0x1c84, 0, 450)
    f(0x29d8, 0, 1)
    f(0x29dc, 0, 1)
    f(0x1c8c, 0, 3)
    i32(0x1c1c, r.randint(0, max(1, ng)) if gear is None else gear)
    i32(0x1c18, max(1, ng))
    i32(0x1c14, r.randint(0, 1))
    i32(0x1c20, r.randint(-1, max(1, ng)))
    i32(0x1c24, r.choice([0, 0, 1, 2, 21, 22, 100]))
    i32(0x1c28, r.choice([0, 1, 2, 3, 500, 1500]))
    f(0x1c2c, 0, 20)
    f(0x1c30, 0.1, 1.5)
    f(0x1c34, 0.1, 1.5)
    f(0x1c38, 0.1, 1.5)
    f(0x1c3c, 0, 20)
    f(0x1c40, 0.5, 5)
    f(0x1c90, 0, 400)
    f(0x1c94, 0, 1500)
    for off in (0x3ebc, 0x3ef0, 0x3f8c, 0x3fc0, 0x3ff4):
        w += ts_writes(r, CAR + off)
    return w, vid, n


def make_filters(emu):
    """Construct the two engine filters with the original constructor."""
    emu.call(0x4eab58, [0x4eaa41, 0.3, 0, 0.0], 'ifif', this=CAR + 0x1b50)
    emu.call(0x4eab58, [0x4eaa77, 0.5, 0, 0.0], 'ifif', this=CAR + 0x1b30)


RUST_FILTERS = (f'filter::new(&mut m, {CAR + 0x1b50:#x}, filter::WEIGHT_LINEAR, 0.3f32, 0, 0.0f32); '
                f'filter::new(&mut m, {CAR + 0x1b30:#x}, filter::WEIGHT_QUADRATIC, 0.5f32, 0, 0.0f32);')
WATCH = [(CAR, CAR_LEN), HEAP_WATCH]


def update_writes(emu, r, now):
    """State for the per-frame engine/gearbox update (0x4ff800)."""
    w, vid, n = car_writes(emu, r)
    ng = max(1, struct.unpack('<i', emu.read(TECH_TABLE + vid * 0x970 + 144, 4))[0])
    add = lambda off, kind, v: w.append((CAR + off, pack(kind, v)))
    f = lambda off, lo, hi: add(off, 'f', r.uniform(lo, hi))
    back = lambda off: add(off, 'f', now - r.choice([0.0, 0.01, 0.3, 0.7, 1.5, 3.0, 10.0]))
    add(0x2774, 'i', r.choice([0, 1, 1, 1, 1]))
    add(0x1b24, 'i', r.choice([0, 1, 2, 2, 2, 3]))
    add(0x2716, 'h', r.choice([0, 0, 0, 1, 2]))
    add(0x2714, 'h', r.choice([0, 0, 1, 2, 3]))
    add(0x2704, 'h', r.choice([0, 1, 1]))
    add(0x2718, 'h', r.choice([0, 1, 2]))
    add(0x2740, 'h', r.choice([0, 0, 1, 2]))
    add(0x2746, 'h', r.choice([0, 0, 0, 0, 1, -1, 0x14 + r.randint(0, ng), 0x15, 0x16, 100]))
    for off in (0x19a4, 0x19a6):
        add(off, 'h', r.choice([0, 0, 0, 1]))
    f(0x2710, -0.2, 1.2)
    f(0x270c, -0.2, 1.2)
    f(0xdc4, -6, 40)
    f(0xdc8, -6, 40)
    f(0x3bd4, 0, 2)
    f(0x1b74, -1, 1)
    f(0x1b78, -1, 1)
    for off in range(0x1844, 0x1860, 4):
        f(off, -2, 2)
    for off in range(0x10, 0x40, 4):
        f(off, -1.5, 1.5)
    for off in (0x17d8, 0x17dc, 0x1c2c, 0x1c3c, 0x1b28, 0x1b2c):
        back(off)
    add(0x2754, 'i', r.choice([0, 0, 1]))
    add(0x2760, 'i', r.choice([0, 1, 1]))
    add(0x277c, 'i', r.choice([0, 0, 1, -1]))
    add(0x2701, 'b', r.choice([0, 0, 1]))
    add(0x2700, 'b', r.choice([0, 1, 2]))
    add(0x1c24, 'i', r.choice([0, 0, 0, 1, 21, 22, 100]))
    add(0x1c20, 'i', r.choice([-1, 0, 1, 2, ng]))
    w.append((0x696c4c, pack('i', r.choice([0, 0, 1]))))
    w.append((0x702630 + 8, pack('f', 1.0)))   # up vector (0,0,1): the static initialiser's result
    return w, vid, n

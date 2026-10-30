"""Pedal axes of the player's input update (0x53b590..0x53b67e) and the axis read 0x5300b0, run in the
original from the middle of 0x53b4c0 with registers set as the function has them there."""
import pathlib, random, struct
from emu import Emu, OBJ_BASE, STACK_TOP
from unicorn.x86_const import UC_X86_REG_ESP, UC_X86_REG_EBP, UC_X86_REG_ESI, UC_X86_REG_FPCW

r = random.Random(0x53b590)
CAR, CTL, VALS = OBJ_BASE + 0x10000, OBJ_BASE + 0x20000, OBJ_BASE + 0x30000
CLOCK_OBJ = OBJ_BASE + 0x100000
DEVICE = 0x6d1db8
WEIGHTS = [0x4eaa0e, 0x4eaa41, 0x4eaa77, 0x4eab02]
rows = []
for case in range(40):
    e = Emu()
    e.install_crt_hooks()
    e.set_u32(0x6974c4, CLOCK_OBJ)
    mode = r.randrange(2)
    e.set_u32(0x6d2068, mode)
    weight, window = r.choice(WEIGHTS), r.choice([0.2, 0.3, 0.5, 1.0])
    mix = r.choice([0, 0, 0x4eaa41, 0x4eab02])
    default = r.choice([0.0, 0.5])
    e.call(0x4eab58, [weight, window, mix, default], 'ifif', this=CTL + 0x118)
    # axes 1 and 2: which of the two sources exist (0 none, 1 first, 2 second, 3 both)
    cfg = [r.randrange(4), r.randrange(4)]
    for k, idx in enumerate((1, 2)):
        a = VALS + 16 * idx
        e.set_u32(DEVICE + idx * 4 + 0x280, a if cfg[k] & 1 else 0)
        e.set_u32(DEVICE + idx * 4 + 0x28c, a + 4 if cfg[k] & 2 else 0)
    t = r.uniform(0, 10)
    frames = []
    for step in range(r.randint(10, 30)):
        t += r.choice([0.0, 0.016, 0.033, 0.1])
        e.write(CLOCK_OBJ + 0xb98, struct.pack('<d', t))
        vals = [r.choice([r.uniform(-1, 1), -1.0, 1.0, 0.0, -0.5]) for _ in range(4)]
        for idx in (1, 2):
            e.write(VALS + 16 * idx, struct.pack('<2f', vals[(idx - 1) * 2], vals[(idx - 1) * 2 + 1]))
        engine = r.choice([1, 1, 1, 0])
        e.write(CAR + 0x2700, bytes([engine]))
        e.set_u32(CAR + 0x270c, 0xdeadbeef); e.set_u32(CAR + 0x2710, 0xdeadbeef)
        e.write(CAR + 0x2716, struct.pack('<h', 0))
        mu = e.mu
        sp = STACK_TOP - 0x2000
        mu.reg_write(UC_X86_REG_ESP, sp); mu.reg_write(UC_X86_REG_ESI, CAR); mu.reg_write(UC_X86_REG_EBP, CTL)
        mu.reg_write(UC_X86_REG_FPCW, 0x27f)
        mu.mem_write(sp + 0x28, bytes(0x20))
        mu.emu_start(0x53b590, 0x53b67e, count=200000)
        frames.append((t, vals, engine, struct.unpack('<II', e.read(CAR + 0x270c, 8)), struct.unpack('<h', e.read(CAR + 0x2716, 2))[0]))
    fh = lambda x: struct.pack('<f', x).hex()
    head = [str(mode), str(weight), fh(window), str(mix), fh(default), str(cfg[0]), str(cfg[1])]
    body = ';'.join('%r,%s,%d,%d,%d,%d' % (t, '/'.join(fh(v) for v in vals), eng, g, b, s) for t, vals, eng, (g, b), s in frames)
    rows.append(chr(9).join([','.join(head), body]))
pathlib.Path(__file__).with_name('golden_pedal_axes.tsv').write_text(chr(10).join(rows) + chr(10))
print(len(rows), 'cases')

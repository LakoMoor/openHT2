"""0x555500 (gauge level of the scene, ecx = scene) run unmodified; the only stub is the virtual [vtable+0xb8] whose
result the weighing never uses (it returns a float through `fld [X]; ret` in emulator memory). The result is
read from the FPU as a double: 0x555500 is followed by `fstp qword [R]` placed at a stub address.
Row: car present, weights (0x90 bytes), object bytes (0xb4), car bytes +0x3c00..+0x4040, result bits."""
import pathlib, random, struct
from emu import Emu, OBJ_BASE, STUB
from unicorn.x86_const import UC_X86_REG_ESP, UC_X86_REG_ECX, UC_X86_REG_EIP, UC_X86_REG_FPCW

r = random.Random(0x555500)
e = Emu()
SCENE, OBJ, CAR, VT, X, R = (OBJ_BASE + 0x10000, OBJ_BASE + 0x20000, OBJ_BASE + 0x30000, OBJ_BASE + 0x21000,
                             OBJ_BASE + 0x22000, OBJ_BASE + 0x23000)
VIRT = STUB + 0x300
e.write(VIRT, b'\xd9\x05' + struct.pack('<I', X) + b'\xc3')   # fld dword [X]; ret
e.set_u32(VT + 0xb8, VIRT)
# a tiny trampoline: call 0x555500; fstp qword [R]; ret-to-stub
TR = STUB + 0x400
e.write(TR, b'\xe8' + struct.pack('<i', 0x555500 - (TR + 5)) + b'\xdd\x1d' + struct.pack('<I', R) + b'\xc3')
F = lambda *v: struct.pack('<%df' % len(v), *v)
rows = []
for case in range(300):
    e.write(SCENE, bytes(0x5600)); e.write(OBJ, bytes(0x100)); e.write(CAR, bytes(0x4100)); e.write(R, bytes(8))
    has = r.choice([1, 1, 1, 0])
    e.set_u32(SCENE + 0x5460, CAR if has else 0)
    e.set_u32(SCENE + 0x2540, OBJ)
    e.set_u32(SCENE, VT)
    sc = lambda: r.choice([0.0, 1.0, -1.0, 0.5, r.uniform(-4, 4), r.uniform(-100, 100)])
    wts = F(*[sc() for _ in range(36)])
    e.write(SCENE + 0x28c0, wts)
    ob = F(*[sc() for _ in range(45)])
    e.write(OBJ, ob)
    cb = F(*[sc() for _ in range(0x440 // 4)])
    e.write(CAR + 0x3c00, cb)
    e.write(X, F(r.uniform(-3, 3)))
    # run the trampoline as a call so that the return lands in the stub area
    e.call(TR, [], '', this=SCENE)
    res = e.read(R, 8) if has else None
    if not has:
        res = struct.pack('<d', 0.0)
    rows.append(chr(9).join([str(has), wts.hex(), ob.hex(), cb.hex(), '%016x' % struct.unpack('<Q', res)[0]]))
pathlib.Path(__file__).with_name('golden_scene_level.tsv').write_text(chr(10).join(rows) + chr(10))
print(len(rows), 'cases')

import os, struct, sys
sys.path.insert(0, os.path.dirname(__file__))
from emu import Emu, OBJ_BASE
from unicorn import UC_HOOK_MEM_WRITE, UcError
from unicorn.x86_const import UC_X86_REG_EIP
e = Emu(); e.install_crt_hooks(); e.load_tech()
CAR = OBJ_BASE + 0x1000; CLOCK = OBJ_BASE + 0x100000; WORLD = OBJ_BASE + 0x200000; VT = WORLD + 0x100; STUBFN = WORLD + 0x400
e.write(0x6974c4, struct.pack('<I', CLOCK)); e.write(CLOCK + 0xb98, struct.pack('<d', 10.0)); e.write(CLOCK + 0xb90, struct.pack('<d', 0.016))
e.write(0x7025f0, struct.pack('<12f', 1,0,0, 0,1,0, 0,0,1, 0,0,0)); e.write(0x702630, struct.pack('<3f', 0,0,1)); e.write(0x702620, struct.pack('<3f', 0,1,0)); e.write(0x702640, struct.pack('<3f', 1,0,0))
e.write(STUBFN, bytes([0x31, 0xc0, 0xc2, 0x08, 0x00])); e.write(WORLD, struct.pack('<I', VT)); e.write(VT, struct.pack('<64I', *([STUBFN] * 64)))
targets = [int(x, 16) for x in sys.argv[2:]]
seen = {}
def w(uc, access, addr, size, value, user):
    off = addr - CAR
    if any(t <= off < t + 4 for t in targets):
        eip = uc.reg_read(UC_X86_REG_EIP)
        seen.setdefault((off, eip), []).append(value)
e.mu.hook_add(UC_HOOK_MEM_WRITE, w, begin=CAR, end=CAR + 0x5600)
vid = int(sys.argv[1])
try:
    e.call(0x4ebd00, [WORLD] + [1,0,0, 0,1,0, 0,0,1, 100.0,200.0,300.0] + [vid], 'i' + 'f'*12 + 'i', this=CAR, max_insn=5_000_000)
except UcError as ex:
    print('stopped:', ex)
for (off, eip), vals in sorted(seen.items()):
    print(hex(off), 'written at', hex(eip), 'values', [hex(v) for v in vals[:4]])
b = e.read(CAR + 0x1e00, 0x40)
print([round(struct.unpack_from('<f', b, i)[0], 4) for i in range(0, 0x40, 4)])

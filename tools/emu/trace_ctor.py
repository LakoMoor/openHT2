import struct, sys
sys.path.insert(0, '.')
from unicorn import *
from unicorn.x86_const import *
from harness import Harness, OBJ_BASE
from carstate import CAR
h = Harness('tmp_ctor', [])
e = h.e
WORLD = OBJ_BASE + 0x200000
VT, STUBFN = WORLD + 0x100, WORLD + 0x400
consts = [(0x7025f0, struct.pack('<12f', 1, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0)), (0x702630, struct.pack('<3f', 0, 0, 1)),
          (0x702620, struct.pack('<3f', 0, 1, 0)), (0x702640, struct.pack('<3f', 1, 0, 0)),
          (STUBFN, bytes([0x31, 0xc0, 0xc2, 0x08, 0x00])), (WORLD, struct.pack('<I', VT)), (VT, struct.pack('<64I', *([STUBFN] * 64)))]
h.reset(10.0)
for a, d in consts:
    e.write(a, d)
e.hook_ret(0x4e6059, lambda e: 0)
e.hook_ret(0x4e623e, lambda e: 0)
e.hook_ret(0x6037e0, lambda e: 0)
FUNCS = [int(x, 16) for x in sys.argv[2].split(',')]
depth = [0]
def rel(v):
    return ('car+%#x' % (v - CAR)) if CAR <= v < CAR + 0x8000 else hex(v)
def ent(uc, address, size, user):
    sp = uc.reg_read(UC_X86_REG_ESP)
    caller = struct.unpack('<I', uc.mem_read(sp, 4))[0]
    args = struct.unpack('<6I', uc.mem_read(sp + 4, 24))
    print('%x <- %x ecx=%s args=%s' % (address, caller, rel(uc.reg_read(UC_X86_REG_ECX)), [rel(a) for a in args[:int(sys.argv[3])]]))
for fa in FUNCS:
    e.mu.hook_add(UC_HOOK_CODE, ent, begin=fa, end=fa)
vid = int(sys.argv[1])
args = [WORLD] + [1, 0, 0, 0, 1, 0, 0, 0, 1, 100.0, 200.0, 300.0] + [vid]
e.call(0x4ebd00, args, 'i' + 'f' * 12 + 'i', this=CAR, max_insn=20_000_000)

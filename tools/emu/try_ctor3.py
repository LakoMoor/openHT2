import struct, sys
sys.path.insert(0, '.')
from unicorn import *
from unicorn.x86_const import *
from harness import Harness, pack, OBJ_BASE
from carstate import CAR, TECH_TABLE
h = Harness('tmp_ctor', [])
e = h.e
WORLD = OBJ_BASE + 0x200000
VT = WORLD + 0x100
STUBFN = WORLD + 0x400
consts = [(0x7025f0, struct.pack('<12f', 1, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0)),
          (0x702630, struct.pack('<3f', 0, 0, 1)), (0x702620, struct.pack('<3f', 0, 1, 0)),
          (0x702640, struct.pack('<3f', 1, 0, 0)),
          (STUBFN, bytes([0x31, 0xc0, 0xc2, 0x08, 0x00])), (WORLD, struct.pack('<I', VT)),
          (VT, struct.pack('<64I', *([STUBFN] * 64)))]
h.reset(10.0)
for a, d in consts:
    e.write(a, d)
last = []
def tr(uc, address, size, user):
    last.append(address)
    if len(last) > 30: last.pop(0)
e.mu.hook_add(UC_HOOK_CODE, tr)
def cb(uc, access, address, size, value, user):
    print('UNMAPPED', hex(address), 'eip', hex(uc.reg_read(UC_X86_REG_EIP)))
    return False
e.mu.hook_add(UC_HOOK_MEM_READ_UNMAPPED | UC_HOOK_MEM_WRITE_UNMAPPED | UC_HOOK_MEM_FETCH_UNMAPPED, cb)
vid = 5
e.hook_ret(0x4e6059, lambda e: 0)
e.hook_ret(0x4e623e, lambda e: 0)
e.hook_ret(0x6037e0, lambda e: 0)
args = [WORLD] + [1, 0, 0, 0, 1, 0, 0, 0, 1, 100.0, 200.0, 300.0] + [vid]
try:
    e.call(0x4ebd00, args, 'i' + 'f' * 12 + 'i', this=CAR, max_insn=20_000_000)
    print('completed')
except UcError as ex:
    print('ERR', ex, [hex(x) for x in last[-8:]])

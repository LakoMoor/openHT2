import os, struct, sys
sys.path.insert(0, os.path.dirname(__file__))
from emu import Emu, OBJ_BASE
from unicorn import UC_HOOK_MEM_READ_UNMAPPED, UC_HOOK_MEM_WRITE_UNMAPPED, UC_HOOK_MEM_FETCH_UNMAPPED, UcError

e = Emu(); e.install_crt_hooks(); e.load_tech()
CAR = OBJ_BASE + 0x1000
CLOCK = OBJ_BASE + 0x100000
e.write(0x6974c4, struct.pack('<I', CLOCK)); e.write(CLOCK + 0xb98, struct.pack('<d', 10.0)); e.write(CLOCK + 0xb90, struct.pack('<d', 0.016))
# static constants
e.write(0x7025f0, struct.pack('<12f', 1,0,0, 0,1,0, 0,0,1, 0,0,0))
e.write(0x702630, struct.pack('<3f', 0,0,1)); e.write(0x702620, struct.pack('<3f', 0,1,0)); e.write(0x702640, struct.pack('<3f', 1,0,0))
log=[]
def bad(uc, access, addr, size, value, user):
    log.append((access, hex(addr), size)); return False
e.mu.hook_add(UC_HOOK_MEM_READ_UNMAPPED | UC_HOOK_MEM_WRITE_UNMAPPED | UC_HOOK_MEM_FETCH_UNMAPPED, bad)
import collections
from unicorn import UC_HOOK_CODE
ring = collections.deque(maxlen=400)
e.mu.hook_add(UC_HOOK_CODE, lambda uc, a, sz, u: ring.append(a))
vid = int(sys.argv[1]) if len(sys.argv) > 1 else 31
args = [0] + [1,0,0, 0,1,0, 0,0,1, 100.0,200.0,300.0] + [vid]
try:
    e.call(0x4ebd00, args, 'i' + 'f'*12 + 'i', this=CAR, max_insn=5_000_000)
    print('ctor finished')
except UcError as ex:
    print('error', ex, log[-3:])
    from unicorn.x86_const import UC_X86_REG_EIP
    print('eip', hex(e.mu.reg_read(UC_X86_REG_EIP)))
    print('last', [hex(a) for a in list(ring)])
b = e.read(CAR, 0x5600)
f = lambda o: struct.unpack_from('<f', b, o)[0]
i = lambda o: struct.unpack_from('<i', b, o)[0]
print('id', i(0x8cc), 'peak', f(0x1c88), 'brakecoef', f(0x1c8c), 'maxgear', i(0x1c18), 'gear', i(0x1c1c), 'manual', i(0x1c14))
print('gear factors', [round(f(0x1c44+4*g),3) for g in range(8)], 'shift', f(0x1c30), f(0x1c34), f(0x1c38), 'c40', f(0x1c40))
print('wheels', i(0xdd4), 'pos', f(0x34), f(0x38), f(0x3c))

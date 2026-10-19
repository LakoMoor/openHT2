import sys; sys.path.insert(0, r'E:\openHT2\openHT2\tools\emu')
from unicorn import *
from unicorn.x86_const import *
import emu
orig = emu.Emu.call
def call(self, *a, **k):
    def cb(uc, access, address, size, value, user):
        print('UNMAPPED', hex(address), 'eip', hex(uc.reg_read(UC_X86_REG_EIP)))
        return False
    if not getattr(self, '_dbg', False):
        self.mu.hook_add(UC_HOOK_MEM_READ_UNMAPPED, cb); self._dbg = True
    return orig(self, *a, **k)
emu.Emu.call = call
mod = sys.argv[1]
try:
    __import__(mod)
except Exception as ex:
    print('ERR', ex)

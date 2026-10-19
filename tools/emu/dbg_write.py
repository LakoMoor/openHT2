import sys, struct
sys.path.insert(0, r'E:\openHT2\openHT2\tools\emu')
from unicorn import *
from unicorn.x86_const import *
import harness
gen, case, lo, hi = sys.argv[1], int(sys.argv[2]), int(sys.argv[3], 16), int(sys.argv[4], 16)
orig = harness.Harness.case
def case_fn(self, *a, **k):
    if len(self.cases) == case:
        mu = self.e.mu
        def cb(uc, access, address, size, value, user):
            print('write', hex(address), hex(value & 0xffffffff), 'eip', hex(uc.reg_read(UC_X86_REG_EIP)))
        h1 = mu.hook_add(UC_HOOK_MEM_WRITE, cb, begin=lo, end=hi)
        orig(self, *a, **k)
        raise SystemExit
    return orig(self, *a, **k)
harness.Harness.case = case_fn
__import__(gen)

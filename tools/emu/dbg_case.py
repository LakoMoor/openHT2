"""Run a generator but trace calls of one function in one case: python dbg_case.py gen_x CASE ADDR NARGS"""
import sys, struct
sys.path.insert(0, r'E:\openHT2\openHT2\tools\emu')
from unicorn import *
from unicorn.x86_const import *
import harness
gen, case, addr, nargs = sys.argv[1], int(sys.argv[2]), int(sys.argv[3], 16), int(sys.argv[4])
orig = harness.Harness.case
def case_fn(self, *a, **k):
    if len(self.cases) == case:
        mu = self.e.mu
        def ent(uc, address, size, user):
            sp = uc.reg_read(UC_X86_REG_ESP)
            ret = struct.unpack('<I', uc.mem_read(sp, 4))[0]
            args = struct.unpack('<%dI' % nargs, uc.mem_read(sp + 4, 4 * nargs))
            print('call', hex(address), 'ecx', hex(uc.reg_read(UC_X86_REG_ECX)), [hex(x) for x in args])
            def ex(uc2, address2, size2, user2):
                print('  ret', hex(uc2.reg_read(UC_X86_REG_EAX)))
                uc2.hook_del(hh[0])
            hh = [uc.hook_add(UC_HOOK_CODE, ex, begin=ret, end=ret)]
        h1 = mu.hook_add(UC_HOOK_CODE, ent, begin=addr, end=addr)
        orig(self, *a, **k)
        mu.hook_del(h1)
        raise SystemExit
    return orig(self, *a, **k)
harness.Harness.case = case_fn
__import__(gen)

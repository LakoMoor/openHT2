"""Run case CASE of a generator in the emulator up to address STOP and dump the vehicle memory."""
import sys
sys.path.insert(0, r'E:\openHT2\openHT2\tools\emu')
from unicorn import *
import harness
gen, case, stop, out = sys.argv[1], int(sys.argv[2]), int(sys.argv[3], 16), sys.argv[4]
orig = harness.Harness.case
def case_fn(self, *a, **k):
    if len(self.cases) == case:
        mu = self.e.mu
        from carstate import CAR
        def cb(uc, address, size, user):
            open(out, 'wb').write(bytes(uc.mem_read(CAR, 0x6000)))
            uc.emu_stop()
        mu.hook_add(UC_HOOK_CODE, cb, begin=stop, end=stop)
        try:
            orig(self, *a, **k)
        except Exception as ex:
            print('stopped', ex)
        raise SystemExit
    return orig(self, *a, **k)
harness.Harness.case = case_fn
__import__(gen)

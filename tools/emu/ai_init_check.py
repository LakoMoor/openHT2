"""Start the whole AI world of a map in the emulator (0x435730) and print the calls it makes to the
game's callbacks (0x4d76b0 player events, 0x4d8b60)."""
import sys

import dump_ai_world
from dump_ai_world import load

STEPS = [0x4031e0, 0x406600, 0x403110, 0x402be0, 0x406340, 0x4842a0, 0x406370, 0x40d1b0, 0x40c3b0, 0x40cef0,
         0x42e9f0, 0x42f4c0, 0x430230, 0x430480]
_orig_call = None


def watch(e):
    from unicorn import UC_HOOK_CODE
    for a in STEPS:
        e.mu.hook_add(UC_HOOK_CODE, lambda uc, addr, size, u: print('# step', hex(addr), flush=True), begin=a, end=a)

events = []
dump_ai_world.WATCH = watch
e = load(sys.argv[1], full=True, events=events)
for which, args in events[:200]:
    print(which, ' '.join(hex(a) for a in args))
print(len(events), 'callback calls')

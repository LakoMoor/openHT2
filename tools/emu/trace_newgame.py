"""Start a new game (0x434780) in the emulated AI world and log every draw of the road module's
generators (0x68b9c8, 0x68a870: a draw moves the index at +0xdc), with where it was made, for
checking the order of the ported start of a game.

    python trace_newgame.py <game folder>\\ENV > draws.txt

Needs the start-up snapshot that ai_traffic.py writes (%TEMP%\\ht2_ai_<map>_start.snap).
"""
import os
import struct
import sys

from unicorn import UC_HOOK_MEM_WRITE, UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_EIP, UC_X86_REG_ESP

from dump_ai_world import load

folder = sys.argv[1]
snap = os.path.join(os.environ.get('TEMP', '.'), 'ht2_ai_' + os.path.basename(os.path.normpath(folder)) + '_start.snap')
e = load(folder, full=True, events=[], snapshot=snap)
# the game clock the AI reads (as in ai_traffic.py), time 0, and an empty game scene
GAME, CLOCK, SCENE = e.heap_next, e.heap_next + 0x1000, e.heap_next + 0x2000
e.heap_next += 0x4000
e.write(GAME, bytes(0x4000))
e.set_u32(0x6cec6c, GAME)
e.set_u32(GAME + 0x54c, CLOCK)
e.set_u32(0x696ca0, SCENE)
e.hook_thiscall(0x5d9290, lambda emu: 0, 1)
for addr in (0x4d76b0, 0x4d8b60):    # the game's callbacks: not run
    e.hook_ret(addr, lambda emu: 0)
GENS = {0x68b9c8: 'ai', 0x68a870: 'second'}


def callers(uc, n=4):
    """The first return addresses on the stack that follow a call."""
    sp = uc.reg_read(UC_X86_REG_ESP)
    out = []
    for k in range(0, min(0x4000, 0x30000000 - sp), 4):
        w = struct.unpack('<I', bytes(uc.mem_read(sp + k, 4)))[0]
        if 0x401005 <= w < 0x640000:
            before = bytes(uc.mem_read(w - 6, 6))
            if before[1] == 0xe8 or before[4] == 0xff and before[5] & 0x38 == 0x10 or before[0] == 0xff and before[1] & 0x38 == 0x10:
                out.append(hex(w))
                if len(out) == n:
                    break
    return out


def on_write(uc, access, addr, size, value, user):
    base = addr - 0xdc
    print(GENS[base], hex(uc.reg_read(UC_X86_REG_EIP)), (value + 0) % 55, ' '.join(callers(uc)), flush=True)


for g in GENS:
    e.mu.hook_add(UC_HOOK_MEM_WRITE, on_write, begin=g + 0xdc, end=g + 0xdc + 3)
for a in (0x403940, 0x409310, 0x4140f0, 0x4305f0, 0x4143c0, 0x414670, 0x433f90, 0x4340f0, 0x434270, 0x434070,
          0x4095c0, 0x4279a0, 0x42fa90, 0x423c00, 0x41d340, 0x41cff0, 0x42cd70, 0x42cb20, 0x477b60):
    e.mu.hook_add(UC_HOOK_CODE, lambda uc, addr, size, u: print('step', hex(addr), flush=True), begin=a, end=a)
e.call(0x434780, [0, 0, 0], 'iii', max_insn=10**12)
print('done')

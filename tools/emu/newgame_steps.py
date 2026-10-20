"""Run the steps of the new-game start (0x434780) one by one in the emulated AI world, for
looking at the state between them. The town distance table (0x414670) takes most of an hour and
draws no random numbers; it is skipped unless asked for.

    from newgame_steps import run
    e = run(folder, until=0x434270)      # runs the steps up to and including 0x434270
"""
import os

from dump_ai_world import load

STEPS = [0x403940, 0x409310, 0x4140f0, 0x4305f0, 0x4143c0, 0x414670, 0x433f90, 0x4340f0, 0x434270, 0x434070,
         0x4095c0, 0x42fa90]   # (0x5e48f0 and 0x4279a0 in between are the game scene's: no AI state)


def run(folder, until, distances=False):
    snap = os.path.join(os.environ.get('TEMP', '.'), 'ht2_ai_' + os.path.basename(os.path.normpath(folder)) + '_start.snap')
    e = load(folder, full=True, events=[], snapshot=snap)
    game, clock, scene = e.heap_next, e.heap_next + 0x1000, e.heap_next + 0x2000
    e.heap_next += 0x4000
    e.write(game, bytes(0x4000))
    e.set_u32(0x6cec6c, game)
    e.set_u32(game + 0x54c, clock)
    e.set_u32(0x696ca0, scene)
    e.hook_thiscall(0x5d9290, lambda emu: 0, 1)
    for addr in (0x4d76b0, 0x4d8b60):
        e.hook_ret(addr, lambda emu: 0)
    for step in STEPS:
        if step == 0x414670 and not distances:
            continue
        e.call(step, max_insn=10**12)
        if step == 0x403940:
            # the second generator: seeded with the AI time (0 at the start) XOR 161803398 (0x434780)
            seed_second(e, 0)
        if step == until:
            break
    return e


def seed_second(e, time):
    import struct
    table = [0] * 55
    s = (time ^ 0x9a4ec86) & 0xffffffff
    k, idx_seed = 1, s
    e_ = 0
    while e_ < 0x46e:
        nxt = (idx_seed - k) & 0xffffffff
        i = e_ % 55
        e_ += 21
        table[i] = k
        k = nxt
        idx_seed = table[i]
    for _ in range(4):
        for c in range(55):
            table[c] = (table[c] - table[(c + 31) % 55]) & 0xffffffff
    e.write(0x68a870, struct.pack('<55I', *table))
    e.write(0x68a948, struct.pack('<III', s, 0, 0x1f))

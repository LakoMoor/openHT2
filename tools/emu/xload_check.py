"""AI ticks (0x42d6e0) from the state after a new game with a model of the game's scene objects,
for comparing the ported `xLoadEntities` (0x424550) and `updateGhosts` (0x420ed0).

The game gives every player a scene object id when it is made (event 1 of its handler 0x4d76b0:
100000 + a counter); here the players of the new game get 100000 + their place in the list and
later ones the next numbers. The scene callbacks are replaced: 0x4d8a40 loaded, 0x4d8a60 in an
active module (always), 0x4d8a90 memory left (fewer than <budget> objects loaded), 0x4d8b20 load,
0x4d8ae0 unload. XLOAD_NO_OBJECTS=1 leaves out the hidden objects (list 0x68cf6c, flag 0x20) that
the port does not have yet; XLOAD_VEC=1 prints the vectors each player is weighed by.

Printed: `tick N`, `load K` / `unload K` (K: the player's place in the list), `ghost K place`
for players made in a tick (as tick_dump.py prints places), `delete K`.

    python xload_check.py <game folder>/ENV <ticks> <dt> <budget> > xload.txt
"""
import os
import struct
import sys

from unicorn import UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_ESP

from dump_ai_world import load
from newgame_steps import setup

folder, ticks, dt, budget = sys.argv[1], int(sys.argv[2]), float(sys.argv[3]), int(sys.argv[4])
snap = os.path.join(os.environ.get('TEMP', '.'), 'ht2_ai_' + os.path.basename(os.path.normpath(folder)) + '.snap')
e = load(folder, full=True, events=[], snapshot=snap)
# setup() stubs the game's event handler; it is replaced below (one hook per address)
hook_ret = e.hook_ret
e.hook_ret = lambda addr, fn: None if addr == 0x4d76b0 else hook_ret(addr, fn)
setup(e)
e.hook_ret = hook_ret
u = lambda a: struct.unpack('<I', e.read(a, 4))[0]
i32 = lambda a: struct.unpack('<i', e.read(a, 4))[0]
clock = u(u(0x6cec6c) + 0x54c)
sent = u(0x68b990)


def nodes():
    out = []
    n = u(sent)
    while n != sent:
        out.append(n)
        n = u(n)
    return out


start = nodes()
for k, n in enumerate(start):
    e.set_u32(n + 8 + 0x4c, 100000 + k)
counter = [len(start)]
print('players', len(start), flush=True)
# the AI generator (0x68b9c8, indices 0x68baa4 / 0x68baa8), for starting the port from it
print('rng', ' '.join(str(x) for x in struct.unpack('<55I', e.read(0x68b9c8, 220))), u(0x68baa4), u(0x68baa8), flush=True)

if os.environ.get('XLOAD_NO_OBJECTS'):
    o = u(0x68cf6c)
    n = u(o)
    while n != o:
        obj = u(n + 8)
        e.write(obj + 8, bytes([e.read(obj + 8, 1)[0] & ~0x20 & 0xff]))
        n = u(n)

loaded = set()


def who(i):
    for k, n in enumerate(nodes()):
        if i32(n + 8 + 0x4c) == i:
            return k
    return None


def on_load(emu):
    i = emu.arg(0)
    if who(i) is not None and i not in loaded:
        loaded.add(i)
        print('load', who(i), flush=True)
    return 0


def on_unload(emu):
    i = emu.arg(0)
    if i in loaded:
        loaded.discard(i)
        print('unload', who(i), flush=True)
    return 0


def handler(emu):
    # event 1 (a player made): its scene object id
    node, event = emu.arg(0), emu.arg(1)
    if event == 1 and i32(node + 0x54) < 0:
        e.set_u32(node + 0x54, 100000 + counter[0])
        counter[0] = (counter[0] + 1) % 500
    from newgame_steps import KNOWN_EVENTS
    return 0 if event in KNOWN_EVENTS else 0xffffffff


e.hook_ret(0x4d76b0, handler)
e.hook_ret(0x4d8a40, lambda emu: int(emu.arg(0) in loaded))
e.hook_ret(0x4d8a60, lambda emu: int(emu.arg(0) >= 100000))
e.hook_ret(0x4d8a90, lambda emu: int(len(loaded) < budget))
e.hook_ret(0x4d8b20, on_load)
e.hook_ret(0x4d8ae0, on_unload)


def closeness(uc, a, size, user):
    sp = uc.reg_read(UC_X86_REG_ESP)
    v = struct.unpack('<6d', e.read(u(sp + 4), 48))
    node = u(sp + 8 + 0x10)
    ns = nodes()
    print('want', ns.index(node) if node in ns else -1, ' '.join(repr(x) for x in v), flush=True)


# nodes deleted this tick (a new player may get a deleted one's memory)
deleted = []


def delete(uc, a, size, user):
    sp = uc.reg_read(UC_X86_REG_ESP)
    node = u(sp + 4)
    ns = nodes()
    print('delete', ns.index(node) if node in ns else hex(node), flush=True)
    deleted.append(node)


e.mu.hook_add(UC_HOOK_CODE, delete, begin=0x4466a0, end=0x4466a0)


def ghosts(uc, a, size, user):
    # the AI generator when the task starts (other tasks the port lacks draw from it too)
    print('ghosts', ' '.join(str(x) for x in struct.unpack('<55I', e.read(0x68b9c8, 220))), u(0x68baa4), u(0x68baa8), flush=True)


e.mu.hook_add(UC_HOOK_CODE, ghosts, begin=0x420ed0, end=0x420ed0)
if os.environ.get('XLOAD_MOVE_RNG'):
    # the generator when the moves start (after the tasks and 0x446af0, 0x423a90)
    e.mu.hook_add(UC_HOOK_CODE, lambda uc, a, size, user: print('moves', ' '.join(str(x) for x in struct.unpack('<55I', e.read(0x68b9c8, 220))), u(0x68baa4), u(0x68baa8), flush=True), begin=0x42d7a4, end=0x42d7a4)
if os.environ.get('XLOAD_VEC'):
    e.mu.hook_add(UC_HOOK_CODE, closeness, begin=0x4244c0, end=0x4244c0)

time = 0.0
for tick in range(1, ticks + 1):
    time += dt
    e.write(clock + 0xb98, struct.pack('<d', time))
    e.write(clock + 0xb90, struct.pack('<d', dt))
    print('tick', tick, flush=True)
    before = set(nodes())
    deleted.clear()
    e.call(0x42d6e0, max_insn=10**12)
    track = [int(x) for x in os.environ.get('XLOAD_TRACK', '').split(',') if x]
    before -= set(deleted)
    for k, n in enumerate(nodes()):
        if k in track:
            p = n + 8
            t = struct.unpack('<d', e.read(p + 0x28, 8))[0]
            print('place', k, i32(p + 0x20), i32(p + 0x24), repr(t), i32(p + 0x30), i32(p + 0x34), i32(p + 0x38), 'route', (u(p + 0x14) - u(p + 0x10)) // 12, 'flags', hex(u(p + 0x58)), hex(u(p + 0x5c)), flush=True)
    for k, n in enumerate(nodes()):
        if n not in before:
            p = n + 8
            t = struct.unpack('<d', e.read(p + 0x28, 8))[0]
            print('ghost', k, i32(p + 0x20), i32(p + 0x24), repr(t), i32(p + 0x30), i32(p + 0x34), i32(p + 0x38), flush=True)

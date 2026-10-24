"""The town orders `formOrders` (0x41b720, the first task of the game clock) makes, from the state
after a new game (the snapshot ai_traffic.py makes): after each of the first ticks, the orders in
the two slots of every store node of every town (0x20 bytes: merchandise, the legal and the
illegal one when it was swapped, the town it goes to, the deadline, the fee) and the AI
generator.

    python formorders_check.py <game folder>/ENV <ticks> > formorders.txt
"""
import os
import struct
import sys

from dump_ai_world import load
from newgame_steps import setup

folder, ticks = sys.argv[1], int(sys.argv[2])
snap = os.path.join(os.environ.get('TEMP', '.'), 'ht2_ai_' + os.path.basename(os.path.normpath(folder)) + '.snap')
# FO_SAVE=<tick>:<file> keeps the state after that tick; FO_START=<tick>:<file> goes on from it
save_at = os.environ.get('FO_SAVE', '').split(':', 1) if os.environ.get('FO_SAVE') else None
start_at = os.environ.get('FO_START', '').split(':', 1) if os.environ.get('FO_START') else None
first = 1
if start_at:
    first, snap = int(start_at[0]) + 1, start_at[1]
e = load(folder, full=True, events=[], snapshot=snap)
setup(e)
u = lambda a: struct.unpack('<I', e.read(a, 4))[0]
i32 = lambda a: struct.unpack('<i', e.read(a, 4))[0]
f64 = lambda a: struct.unpack('<d', e.read(a, 8))[0]
clock = u(u(0x6cec6c) + 0x54c)


def rng():
    return ' '.join(str(x) for x in struct.unpack('<55I', e.read(0x68b9c8, 220))) + ' %d %d' % (u(0x68baa4), u(0x68baa8))


def dump(tick):
    for t in range((u(0x68cec4) - u(0x68cec0)) // 4):
        town = u(u(0x68cec0) + 4 * t)
        for k, a in enumerate(range(u(town + 0x1c), u(town + 0x20), 0x10)):
            for s in range(2):
                o = u(a + 8 + 4 * s)
                if o:
                    print('slot', tick, t, k, s, i32(o), i32(o + 4), i32(o + 8), i32(o + 12), repr(f64(o + 0x10)), repr(f64(o + 0x18)), flush=True)


from unicorn import UC_HOOK_CODE
# when a player arrives (actOnArrival, 0x419cb0)
from unicorn.x86_const import UC_X86_REG_ESP


def arrival(uc, a, size, user):
    node = u(uc.reg_read(UC_X86_REG_ESP) + 4)
    k, n = 0, u(sent)
    while n != sent and n != node:
        n, k = u(n), k + 1
    print('arrival', k, 'type', i32(node + 0x5c), 'state', hex(u(node + 0x64)), 'flags', hex(u(node + 0x60)), flush=True)


sent = u(0x68b990)
# FO_TRACE=<hex>,<hex>,...: print the first five arguments of each call of these functions
for f in filter(None, os.environ.get('FO_TRACE', '').split(',')):
    def trace(uc, a, size, user):
        sp = uc.reg_read(UC_X86_REG_ESP)
        print('call %x' % a, *[hex(u(sp + 4 + 4 * j)) for j in range(5)], flush=True)
    e.mu.hook_add(UC_HOOK_CODE, trace, begin=int(f, 16), end=int(f, 16))
e.mu.hook_add(UC_HOOK_CODE, arrival, begin=0x419cb0, end=0x419cb0)
human = u(u(0x68b990))
print('human +0x300', repr(f64(human + 0x300)), 'd100', repr(f64(0x68d100)), flush=True)
print('rng', rng(), flush=True)
time = 0.0
for _ in range(1, first):
    time += 0.1
# FO_RNGWATCH=<tick>: during that tick, print where the AI generator is drawn from (writes of
# its index 0x68baa4) with the return addresses on the stack
watch_tick = int(os.environ.get('FO_RNGWATCH', '0'))
cur_tick = [0]
if watch_tick:
    from unicorn import UC_HOOK_MEM_WRITE

    def rng_write(uc, access, addr, size, value, user):
        if cur_tick[0] == watch_tick:
            sp = uc.reg_read(UC_X86_REG_ESP)
            rets = [hex(u(sp + 4 * j)) for j in range(0, 64) if 0x401000 <= u(sp + 4 * j) < 0x640000][:6]
            print('draw at', hex(uc.reg_read(UC_X86_REG_EIP)), rets, flush=True)
    from unicorn.x86_const import UC_X86_REG_EIP
    e.mu.hook_add(UC_HOOK_MEM_WRITE, rng_write, begin=0x68baa4, end=0x68baa7)
# FO_HUMAN=<file>: the human player drives: before each tick the game reports where his vehicle
# is (0x43ee70, a matrix with the point of that line of the file)
path = [tuple(map(float, l.split())) for l in open(os.environ['FO_HUMAN'])] if os.environ.get('FO_HUMAN') else None
if path:
    hnode = u(u(0x68b990))
    MATRIX = e.heap_next
    e.heap_next += 0x40
# FO_ORDERS=<file>: lines "<tick> <town index> <node index> <amount>": before that tick the game
# hands the human player an order (message 0x11: loadOrder 0x443d50, amount > 0 loads, < 0
# unloads); after each tick his money (+0x88), +0x218, +0x2f8 and loads (+0x204/+0x208 list,
# 0x90 bytes each: merchandise, town, amount +0x38, deadline +0x48, time +0x58, price +0x60)
orders = {}
if os.environ.get('FO_ORDERS'):
    for l in open(os.environ['FO_ORDERS']):
        w = l.split()
        if len(w) == 4:
            orders.setdefault(int(w[0]), []).append((int(w[1]), int(w[2]), float(w[3])))
# the game's side of an order (the interface 0x56f6d0, messages 0x56ae60) is not run
if orders:
    for a in (0x56f6d0, 0x56ae60):
        e.hook_ret(a, lambda emu: 0)
ARGS = e.heap_next
e.heap_next += 0x20


def human_orders(tick):
    hn = u(u(0x68b990))
    for t, r, amount in orders.get(tick, []):
        number = i32(u(u(0x68cec0) + 4 * t) + 0x18)
        e.write(ARGS, struct.pack('<di', amount, -1))
        e.call(0x443d50, [hn, number, r, ARGS, ARGS + 8], 'iiiii', max_insn=10**10)
        print('horder', tick, t, r, amount, 'ret', e.eax - (1 << 32) if e.eax >> 31 else e.eax, 'amount', repr(f64(ARGS)), 'index', i32(ARGS + 8), flush=True)


def comp_state(tick):
    # FO_COMP=1: after each tick, the human player and the competitors (list 0x68a980): index,
    # place (road, t bits, segment), +0x1f8, +0x200, +0x350
    idx, n, k = {}, u(sent), 0
    while n != sent:
        idx[n] = k
        n, k = u(n), k + 1
    hn = u(sent)
    print('compglobals', tick, repr(f64(0x66c6a0)), repr(f64(0x68d0f0)), flush=True)
    for c in [hn] + [u(a) for a in range(u(0x68a980), u(0x68a984), 4)]:
        print('comp', tick, idx.get(c, -1), i32(c + 0x28), struct.unpack('<Q', e.read(c + 0x30, 8))[0], i32(c + 0x3c), repr(f64(c + 0x1f8)), e.read(c + 0x200, 1)[0], e.read(c + 0x350, 1)[0], flush=True)


def human_state(tick):
    hn = u(u(0x68b990))
    lst = u(hn + 0x208) or u(hn + 0x204)
    loads = []
    if lst and i32(lst + 4) != -1:
        for a in range(u(lst + 0x1c), u(lst + 0x20), 0x90):
            loads.append('%d %d %r %r %r %r' % (i32(a), i32(a + 0xc), f64(a + 0x38), f64(a + 0x48), f64(a + 0x58), f64(a + 0x60)))
    print('hstate', tick, repr(f64(hn + 0x88)), i32(hn + 0x218), repr(f64(hn + 0x2f8)), len(loads), *loads, flush=True)


# FO_COVER=<file>: the start addresses of the basic blocks of the AI module (0x401000..0x490000)
# that run, written to the file every 500 ticks and at the end
cover_file = os.environ.get('FO_COVER')
cover = set()
if cover_file:
    from unicorn import UC_HOOK_BLOCK
    e.mu.hook_add(UC_HOOK_BLOCK, lambda uc, a, size, user: cover.add(a), begin=0x401000, end=0x490000)


def write_cover():
    with open(cover_file, 'w') as f:
        f.write('\n'.join('%x' % a for a in sorted(cover)) + '\n')


# FO_OFFENCE=<file>: lines "<tick> <value>": before that tick (before the human player is placed)
# the game counts an offence of his (0x56f120: +0xd0 of his node grows, flag 0x20 of +0xd8)
offences = {}
if os.environ.get('FO_OFFENCE'):
    for l in open(os.environ['FO_OFFENCE']):
        w = l.split()
        if len(w) == 2:
            offences.setdefault(int(w[0]), []).append(float(w[1]))
for tick in range(first, ticks + 1):
    cur_tick[0] = tick
    for v in offences.get(tick, []):
        hn_ = u(u(0x68b990))
        e.write(hn_ + 0xd0, struct.pack('<d', f64(hn_ + 0xd0) + v))
        e.write(hn_ + 0xd8, struct.pack('<I', u(hn_ + 0xd8) | 0x20))
    if cover_file and tick % 500 == 0:
        write_cover()
    if path:
        x, y, z = path[tick - 1]
        e.write(MATRIX, struct.pack('<12f', 1, 0, 0, 0, 1, 0, 0, 0, 1, x, y, z))
        e.call(0x43ee70, [hnode, MATRIX], 'ii', max_insn=10**10)
        print('hplace', i32(hnode + 0x28), i32(hnode + 0x2c), struct.unpack('<Q', e.read(hnode + 0x30, 8))[0], i32(hnode + 0x38), i32(hnode + 0x3c), i32(hnode + 0x40), flush=True)
    time += 0.1
    e.write(clock + 0xb98, struct.pack('<d', time))
    e.write(clock + 0xb90, struct.pack('<d', 0.1))
    if orders:
        human_orders(tick)
    e.call(0x42d6e0, max_insn=10**12)
    print('tick', tick, flush=True)
    if orders:
        human_state(tick)
    if os.environ.get('FO_COMP'):
        comp_state(tick)
    if os.environ.get('FO_PW'):
        # FO_PW=k,k,...: after each tick these players' place (road, t bits, segment), route
        # length and end, and waypoints (kind and node)
        want = [int(x) for x in os.environ['FO_PW'].split(',')]
        n, k = u(sent), 0
        while n != sent:
            if k in want:
                r0, r1 = u(n + 0x18), u(n + 0x1c)
                wps = ' '.join('%d:%d' % (i32(a + 0x20), i32(a + 4)) for a in range(u(n + 0x48), u(n + 0x4c), 0x28))
                print('pw', tick, k, i32(n + 0x28), struct.unpack('<Q', e.read(n + 0x30, 8))[0], i32(n + 0x3c), (r1 - r0) // 12, i32(r1 - 0xc) if r1 != r0 else -1, wps, flush=True)
            n, k = u(n), k + 1
    dump(tick)
    print('rng', rng(), flush=True)
    if save_at and int(save_at[0]) == tick:
        from dump_ai_world import save
        save(e, save_at[1])
if cover_file:
    write_cover()

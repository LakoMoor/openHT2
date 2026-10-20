"""Start a new game in the emulated AI world (0x435730, then 0x434780) and run its tick (0x42d6e0)
with a clock, printing what the road module asks the game to do through its player callback
(0x4d76b0: event 1 creates a vehicle).

    python ai_traffic.py <game folder>\\ENV <seconds>
"""
import struct
import sys

from dump_ai_world import load

import os

folder, seconds = sys.argv[1], float(sys.argv[2])
SNAP = os.path.join(os.environ.get('TEMP', '.'), 'ht2_ai_' + os.path.basename(os.path.normpath(folder)) + '.snap')
events = []
fresh = not os.path.exists(SNAP)
e = load(folder, full=True, events=events, snapshot=None if fresh else SNAP)
if not fresh:
    for which, addr in (('player', 0x4d76b0), ('other', 0x4d8b60)):
        e.hook_ret(addr, lambda emu, which=which: (events.append((which, [emu.arg(k) for k in range(4)])), 0)[1])
print('# after start-up:', len(events), 'callbacks')

# the game clock the AI reads (0x483a20): [[0x6cec6c] + 0x54c] + 0xb98 (time), + 0xb90 (step)
GAME, CLOCK = e.heap_next, e.heap_next + 0x1000
e.heap_next += 0x3000
e.write(GAME, bytes(0x1000))
e.write(CLOCK, bytes(0x1000))
e.set_u32(0x6cec6c, GAME)
e.set_u32(GAME + 0x54c, CLOCK)


def set_time(t, dt):
    e.write(CLOCK + 0xb98, struct.pack('<d', t))
    e.write(CLOCK + 0xb90, struct.pack('<d', dt))


set_time(0.0, 0.0)
# the 3D scene is not there: its module look-up (scene keys of the districts) finds nothing
e.hook_thiscall(0x5d9290, lambda emu: 0, 1)
from collections import deque
from unicorn import UC_HOOK_BLOCK
trail = deque(maxlen=16)
e.mu.hook_add(UC_HOOK_BLOCK, lambda uc, a, s, u: trail.append(a) if a < 0x61c000 else None)


def call(addr, args=(), kinds=''):
    try:
        e.call(addr, args, kinds, max_insn=10**12)
    except Exception as ex:
        print('# failed in', hex(addr), ex, [hex(a) for a in trail])
        from unicorn.x86_const import UC_X86_REG_EIP, UC_X86_REG_EAX, UC_X86_REG_ECX, UC_X86_REG_EDX, UC_X86_REG_EBX, UC_X86_REG_ESI, UC_X86_REG_EDI, UC_X86_REG_EBP
        rr = {n: hex(e.mu.reg_read(r)) for n, r in (('eip', UC_X86_REG_EIP), ('eax', UC_X86_REG_EAX), ('ecx', UC_X86_REG_ECX), ('edx', UC_X86_REG_EDX),
                                                    ('ebx', UC_X86_REG_EBX), ('esi', UC_X86_REG_ESI), ('edi', UC_X86_REG_EDI), ('ebp', UC_X86_REG_EBP))}
        print('# registers', rr)
        print('# backtrace', backtrace())
        sent = struct.unpack('<I', e.read(0x68b990, 4))[0]
        n = struct.unpack('<I', e.read(sent, 4))[0]
        while n != sent:
            pos = e.read(n, 0x80).hex()
            print('# player', hex(n), 'position (road, node, t lo/hi, piece, segment, flags, -)', pos)
            n = struct.unpack('<I', e.read(n, 4))[0]
        raise SystemExit(1)


def backtrace():
    """Return addresses on the stack that follow a call instruction (most recent first)."""
    from unicorn.x86_const import UC_X86_REG_ESP
    sp = e.mu.reg_read(UC_X86_REG_ESP)
    out = []
    for k in range(0, 0x20000, 4):
        try:
            w = struct.unpack('<I', e.read(sp + k, 4))[0]
        except Exception:
            break
        if not 0x401005 <= w < 0x640000:
            continue
        before = e.read(w - 6, 6)
        if before[1] == 0xe8 or before[4] == 0xff and before[5] & 0x38 == 0x10 or before[0] == 0xff and before[1] & 0x38 == 0x10:
            out.append(hex(w))
        if len(out) >= 25:
            break
    return out


from unicorn import UC_HOOK_CODE
for a in (0x403940, 0x409310, 0x4140f0, 0x4305f0, 0x4143c0, 0x414670, 0x433f90, 0x4340f0, 0x434270, 0x434070, 0x4095c0,
          0x4279a0, 0x42fa90, 0x423c00, 0x41d340, 0x41cff0, 0x42cd70, 0x42cb20, 0x477b60):
    e.mu.hook_add(UC_HOOK_CODE, lambda uc, addr, size, u: print('# start step', hex(addr), flush=True), begin=a, end=a)
if fresh:
    call(0x434780, [0, 0, 0], 'iii')
    import dump_ai_world
    dump_ai_world.save(e, SNAP)
print('# new game:', len(events), 'callbacks')
_u = lambda a: struct.unpack('<I', e.read(a, 4))[0]
_b, _e = _u(0x68cbf8), _u(0x68cbfc)
print('# vehicle categories (0x68cbf8):', [(_u(c + 4) - _u(c)) // 4 for c in range(_b, _e, 12)])
_vb, _ve = _u(0x68cfd8), _u(0x68cfdc)
for _k in range(min(6, (_ve - _vb) // 4)):
    _v = _u(_vb + 4 * _k)
    _rb, _re = _u(_v + 0x24), _u(_v + 0x28)
    print('# vehicle', _k, 'kind', _u(_v + 0x20), 'roles', [_u(a) for a in range(_rb, _re, 4)][:12], 'raw', e.read(_v, 0x40).hex())
# the players (a list, sentinel at [0x68b990]); the first is the human player, whose truck stands at
# the start position of game.cnf: the game reports it with setPlayerPosition(PlayerId, vrMatrix)
_sent = _u(0x68b990)
players = []
_n = _u(_sent)
while _n != _sent and len(players) < 1000:
    players.append(_n)
    _n = _u(_n)
print('# players after the new game:', len(players))
MATRIX = e.heap_next
e.heap_next += 0x40
e.write(MATRIX, struct.pack('<12f', 1, 0, 0, 0, 1, 0, 0, 0, 1, -3020.0, -4625.0, 20.8))


def report_player():
    if players:
        call(0x43ee70, [players[0], MATRIX], 'ii')


from unicorn import UC_HOOK_CODE as _HC
from unicorn.x86_const import UC_X86_REG_ESP as _SP


def _peek(uc, addr, size, user):
    sp = uc.reg_read(_SP)
    words = struct.unpack('<12i', bytes(uc.mem_read(sp, 48)))
    print('# at', hex(addr), 'stack', words, flush=True)
    if addr == 0x456140:
        pl = words[3] & 0xffffffff
        print('#   player', hex(pl), 'words', struct.unpack('<32i', bytes(uc.mem_read(pl, 128))), flush=True)


for _a in (0x456140, 0x478260, 0x4858b0):
    e.mu.hook_add(_HC, _peek, begin=_a, end=_a)
report_player()
t, dt, shown = 0.0, 0.1, 0
while t < seconds:
    t += dt
    set_time(t, dt)
    report_player()
    call(0x42d6e0)
    for which, args in events[shown:]:
        print(f'{t:7.2f} {which} ' + ' '.join(hex(a) for a in args))
    shown = len(events)
print('#', len(events), 'callbacks in all')

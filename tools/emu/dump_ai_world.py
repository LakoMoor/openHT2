"""Run the original road-network loader (0x406600: every *.way of a map folder, segments, graph,
roads) in the emulator and dump what it built as JSON, to check ht2_sim::road::RoadNet against.

    python dump_ai_world.py <game folder>\\ENV out.json
"""
import json
import os
import struct
import sys

import pefile

sys.path.insert(0, os.path.dirname(__file__))
from emu import Emu, STUB, EXE  # noqa: E402

folder = sys.argv[1]
out = sys.argv[2]
e = Emu()
e.hook_ret(0x4e260c, lambda emu: 0)       # error log
e.hook_ret(0x61edde, lambda emu: 12345)   # rand
e.hook_ret(0x61cca9, lambda emu: 0)       # sprintf
BIG, BIG_LEN = 0x40000000, 0x10000000
e.mu.mem_map(BIG, BIG_LEN)
e.heap_next = BIG


def alloc(emu, who=0):
    n = emu.arg(0)
    if n > 0x100000:
        print('big alloc', hex(who), hex(n), 'game', [hex(x) for x in game] if 'game' in globals() else '')
    a = emu.heap_next
    emu.heap_next += (n + 15) & ~15
    assert emu.heap_next < BIG + BIG_LEN
    return a


e.hook_ret(0x632643, lambda emu: alloc(emu, 0x632643))               # operator new / malloc
e.hook_ret(0x44b740, lambda emu: alloc(emu, 0x44b740))               # pooled allocator of the containers
e.hook_ret(0x44b4a0, lambda emu: 0)
e.hook_ret(0x61d1c8, lambda emu: 0)       # free
e.hook_ret(0x63266c, lambda emu: 0)       # operator delete
e.hook_ret(0x61d857, lambda emu: alloc(emu, 0x61d857))               # malloc (CRT heap)
e.hook_ret(0x401010, lambda emu: 0)       # error report
e.write(STUB + 0x200, bytes([0xc2, 0x04, 0x00]))
e.write(STUB + 0x210, bytes.fromhex('8b4424048b4c2408870889c8c20800'))  # InterlockedExchange

# --- files ---------------------------------------------------------------
names = sorted(f for f in os.listdir(folder) if f.lower().endswith('.way'))
files = {}
next_fp = [0x3000]


def cstr(a):
    s = b''
    while True:
        c = e.read(a, 1)
        if c == b'\0':
            return s.decode('latin-1')
        s += c
        a += 1


def fopen(emu):
    name = os.path.basename(cstr(emu.arg(0)).replace('\\', '/'))
    path = os.path.join(folder, name)
    if not os.path.exists(path):
        return 0
    fp = STUB + 0x400 + next_fp[0] % 0x400
    next_fp[0] += 0x10
    files[fp] = [open(path, 'rb').read(), 0]
    return fp


def fread(emu):
    buf, size, count, fp = emu.arg(0), emu.arg(1), emu.arg(2), emu.arg(3)
    data, pos = files[fp]
    n = min(count, (len(data) - pos) // size) if size else 0
    emu.write(buf, data[pos:pos + n * size])
    files[fp][1] = pos + n * size
    return n


def ftell(emu):
    return files[emu.arg(0)][1]


def fseek(emu):
    fp, off, whence = emu.arg(0), emu.arg(1), emu.arg(2)
    off = struct.unpack('<i', struct.pack('<I', off))[0]
    base = {0: 0, 1: files[fp][1], 2: len(files[fp][0])}[whence]
    files[fp][1] = base + off
    return 0


def _open(emu):
    name = os.path.basename(cstr(emu.arg(0)).replace(chr(92), '/'))
    path = os.path.join(folder, name)
    if not os.path.exists(path):
        return 0xffffffff
    fd = 0x100 + len(files)
    files[fd] = [open(path, 'rb').read(), 0]
    return fd


def _read(emu):
    fd, buf, n = emu.arg(0), emu.arg(1), emu.arg(2)
    data, pos = files[fd]
    chunk = data[pos:pos + n]
    emu.write(buf, chunk)
    files[fd][1] = pos + len(chunk)
    return len(chunk)


e.hook_ret(0x628e69, _open)
e.hook_ret(0x625232, _read)
e.hook_ret(0x6212a4, lambda emu: 0)       # _close
e.hook_ret(0x61cc96, fopen)
e.hook_ret(0x61cb9a, lambda emu: 0)
e.hook_ret(0x61e20f, fread)
e.hook_ret(0x61e45f, ftell)
e.hook_ret(0x61e6a3, fseek)

find = {'i': 0}


def fill(data):
    e.write(data, bytes(0x140))
    e.write(data + 0x2c, names[find['i']].encode() + b'\0')


def find_first(emu):
    find['i'] = 0
    if not names:
        return 0xffffffff
    fill(emu.arg(1))
    return 0x77


def find_next(emu):
    find['i'] += 1
    if find['i'] >= len(names):
        return 0
    fill(emu.arg(1))
    return 1


e.hook_thiscall(STUB + 0x220, find_first, 2)
e.hook_thiscall(STUB + 0x230, find_next, 2)
e.hook_thiscall(STUB + 0x240, lambda emu: 1, 1)
for imp in pefile.PE(EXE).DIRECTORY_ENTRY_IMPORT:
    for i in imp.imports:
        if not i.name:
            continue
        if b'CriticalSection' in i.name:
            e.write(i.address, struct.pack('<I', STUB + 0x200))
        target = {b'InterlockedExchange': 0x210, b'FindFirstFileA': 0x220, b'FindNextFileA': 0x230, b'FindClose': 0x240}.get(i.name)
        if target:
            e.write(i.address, struct.pack('<I', STUB + target))

# the folder prefix (a string: begin 0x68bac8, end 0x68bacc, capacity end 0x68bad0)
prefix = b'C:\\MAP\\'
pa = alloc_addr = e.heap_next
e.heap_next += 0x100
e.write(pa, prefix + b'\0')
for a, v in ((0x68bac8, pa), (0x68bacc, pa + len(prefix)), (0x68bad0, pa + 0x100)):
    e.set_u32(a, v)

# C++ static initialisers (the CRT runs the table 0x66c000..0x66c488 at start-up): global
# containers get their sentinel nodes here.
ok = bad = 0
for a in range(0x66c000, 0x66c488, 4):
    fn = struct.unpack('<I', e.read(a, 4))[0]
    if not fn:
        continue
    try:
        e.call(fn, max_insn=2_000_000)
        ok += 1
    except Exception:
        bad += 1
print('static initialisers', ok, 'ran', bad, 'failed')
from collections import deque
from unicorn import UC_HOOK_BLOCK
from unicorn.x86_const import UC_X86_REG_ESP
last = deque(maxlen=12)
game = deque(maxlen=6)
e.mu.hook_add(UC_HOOK_BLOCK, lambda uc, a, s, u: (last.append(a), game.append(a) if a < 0x61c000 else None))
try:
    e.call(0x406600, max_insn=4_000_000_000)
except Exception as ex:
    print('failed', ex, [hex(a) for a in last], 'game', [hex(a) for a in game], 'esp', hex(e.mu.reg_read(UC_X86_REG_ESP)))
    raise

u32 = lambda a: struct.unpack('<I', e.read(a, 4))[0]
f64 = lambda a: struct.unpack('<d', e.read(a, 8))[0]


def vec(begin):
    b, en = u32(begin), u32(begin + 4)
    return [u32(b + 4 * k) for k in range((en - b) // 4)]


roads = []
for r in vec(0x68cf38):
    roads.append({'start': u32(r + 0x70), 'end': u32(r + 0x74), 'segments': len(vec(r + 0x78)),
                  'length': f64(r + 0x18), 'cumulative': f64(r + 0x28)})
nodes = vec(0x68cf00)
json.dump({'files': names, 'nodes': len(nodes), 'node_ptrs': nodes, 'roads': roads}, open(out, 'w'))
print(len(nodes), 'nodes', len(roads), 'roads')

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



def load(folder, full=False, events=None, snapshot=None):
    """An emulator with the original road network of `folder` loaded (0x406600); with `full`, the
    whole AI world started (0x435730) with its game callbacks recorded into `events`. With
    `snapshot` (a file written by `save`), the memory is restored from it instead of starting."""
    global e, game, files, names, next_fp, find
    e = Emu()
    e.hook_ret(0x4e260c, lambda emu: 0)       # error log
    e.hook_ret(0x61edde, lambda emu: 12345)   # rand
    e.hook_ret(0x61cca9, lambda emu: 0)       # sprintf
    BIG, BIG_LEN = 0x40000000, 0x10000000
    FILES = 0x1e000000   # FILE handles handed out by the hooked fopen
    e.mu.mem_map(FILES, 0x100000)
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
    e.write(STUB + 0x260, bytes.fromhex('8b4c2404b801000000f00fc10140c20400'))  # InterlockedIncrement
    e.write(STUB + 0x280, bytes.fromhex('8b4c2404b8fffffffff00fc10148c20400'))  # InterlockedDecrement

    # --- files ---------------------------------------------------------------
    root = os.path.dirname(os.path.normpath(folder))

    def find_file(name):
        name = name.replace(chr(92), '/')
        base = os.path.basename(name).lower()
        rel = name.split('/MAP/', 1)[-1] if '/MAP/' in name else name.lstrip('/').split(':', 1)[-1].lstrip('/')
        for cand in (os.path.join(folder, os.path.basename(name)), os.path.join(root, rel)):
            if os.path.exists(cand):
                return cand
        for d, _, fs in os.walk(root):
            for f in fs:
                if f.lower() == base:
                    return os.path.join(d, f)
        print('# missing file', name)
        return None

    names = sorted(f for f in os.listdir(folder) if f.lower().endswith('.way'))
    files = {}
    next_fp = [0]


    def cstr(a):
        s = b''
        while True:
            c = e.read(a, 1)
            if c == b'\0':
                return s.decode('latin-1')
            s += c
            a += 1


    def fopen(emu):
        if os.environ.get('HT2_EMU_FILES'):
            print('# fopen', cstr(emu.arg(0)), cstr(emu.arg(1)))
        path = find_file(cstr(emu.arg(0)))
        if path is None:
            return 0
        fp = FILES + 0x40 * next_fp[0]
        next_fp[0] += 1
        files[fp] = [open(path, 'rb').read(), 0]
        return fp


    def fread(emu):
        buf, size, count, fp = emu.arg(0), emu.arg(1), emu.arg(2), emu.arg(3)
        if fp not in files:
            from unicorn.x86_const import UC_X86_REG_ESP
            sp = emu.mu.reg_read(UC_X86_REG_ESP)
            words = struct.unpack('<2048I', emu.read(sp, 8192))
            calls = [hex(w) for w in words if 0x401000 <= w < 0x61c000]
            print('# fread on unknown file', hex(fp), 'stack', calls[:24])
            raise SystemExit
            return 0
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
        path = find_file(cstr(emu.arg(0)))
        if path is None:
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
        e.write(data + 0x2c, find['list'][find['i']].encode() + bytes(1))


    def find_first(emu):
        import fnmatch
        pattern = os.path.basename(cstr(emu.arg(0)).replace(chr(92), '/')).lower()
        find['list'] = sorted(f for f in os.listdir(folder) if fnmatch.fnmatch(f.lower(), pattern))
        find['i'] = 0
        if not find['list']:
            return 0xffffffff
        fill(emu.arg(1))
        return 0x77

    def find_next(emu):
        find['i'] += 1
        if find['i'] >= len(find['list']):
            return 0
        fill(emu.arg(1))
        return 1

    e.hook_thiscall(STUB + 0x220, find_first, 2)
    e.hook_thiscall(STUB + 0x230, find_next, 2)
    e.hook_thiscall(STUB + 0x240, lambda emu: 1, 1)
    # Win32 calls the start-up code makes: (stack words popped, result)
    simple = {b'HeapSize': (3, 0x10000), b'GetVersion': (0, 0x0a280105), b'TlsAlloc': (0, 1),
              b'GetOEMCP': (0, 866), b'GetACP': (0, 1251), b'RegisterWindowMessageA': (1, 0xc100),
              b'GetCursorPos': (1, 1), b'TlsSetValue': (2, 1), b'TlsGetValue': (1, 0),
              b'GetCurrentThreadId': (0, 1), b'GetTickCount': (0, 1000), b'timeGetTime': (0, 1000),
              b'GetProcessVersion': (1, 0x40000), b'GetCPInfo': (2, 0),
              b'SetUnhandledExceptionFilter': (1, 0), b'GetSystemMetrics': (1, 1024)}
    slot = 0x600
    for imp in pefile.PE(EXE).DIRECTORY_ENTRY_IMPORT:
        for i in imp.imports:
            if i.name in simple:
                n, v = simple[i.name]
                e.hook_thiscall(STUB + slot, lambda emu, v=v: v, n)
                e.write(i.address, struct.pack('<I', STUB + slot))
                slot += 0x10

    def heap_alloc(emu, size_arg):
        n = emu.arg(size_arg)
        a = emu.heap_next
        emu.heap_next += (n + 15) & ~15
        emu.write(a, bytes(n))
        return a

    for name, n, k in ((b'HeapAlloc', 3, 2), (b'GlobalAlloc', 2, 1), (b'LocalAlloc', 2, 1)):
        e.hook_thiscall(STUB + slot, lambda emu, k=k: heap_alloc(emu, k), n)
        for imp in pefile.PE(EXE).DIRECTORY_ENTRY_IMPORT:
            for i in imp.imports:
                if i.name == name:
                    e.write(i.address, struct.pack('<I', STUB + slot))
        slot += 0x10
    for imp in pefile.PE(EXE).DIRECTORY_ENTRY_IMPORT:
        for i in imp.imports:
            if not i.name:
                continue
            if b'CriticalSection' in i.name:
                e.write(i.address, struct.pack('<I', STUB + 0x200))
            target = {b'InterlockedExchange': 0x210, b'FindFirstFileA': 0x220, b'FindNextFileA': 0x230, b'FindClose': 0x240,
                      b'InterlockedIncrement': 0x260, b'InterlockedDecrement': 0x280}.get(i.name)
            if target:
                e.write(i.address, struct.pack('<I', STUB + target))

    if snapshot is not None:
        restore(e, snapshot)
        return e

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
    for a in list(range(0x66c48c, 0x66c4a0, 4)) + list(range(0x66c000, 0x66c488, 4)):   # C, then C++
        fn = struct.unpack('<I', e.read(a, 4))[0]
        if not fn:
            continue
        try:
            e.call(fn, max_insn=200_000_000)
            ok += 1
        except Exception as ex:
            bad += 1
            if os.environ.get('HT2_EMU_INIT'):
                from unicorn.x86_const import UC_X86_REG_EIP
                print('# static init', hex(fn), 'failed at', hex(e.mu.reg_read(UC_X86_REG_EIP)), ex)
    print('static initialisers', ok, 'ran', bad, 'failed')
    from collections import deque
    from unicorn import UC_HOOK_BLOCK
    from unicorn.x86_const import UC_X86_REG_ESP
    last = deque(maxlen=30)
    game = deque(maxlen=6)
    e.mu.hook_add(UC_HOOK_BLOCK, lambda uc, a, s, u: (last.append(a), game.append(a) if a < 0x61c000 else None))
    def full_path(emu):
        name = cstr(emu.arg(0)).encode()
        n, buf = emu.arg(1), emu.arg(2)
        if len(name) + 1 <= n:
            e.write(buf, name + b'\0')
        return len(name)

    e.hook_thiscall(STUB + 0x250, full_path, 4)

    def create_file(emu):
        print('# CreateFileA', cstr(emu.arg(0)), hex(emu.arg(1)), hex(emu.arg(4)))
        return 0xffffffff

    e.hook_thiscall(STUB + 0x290, create_file, 7)
    for imp in pefile.PE(EXE).DIRECTORY_ENTRY_IMPORT:
        for i in imp.imports:
            if i.name == b'GetFullPathNameA':
                e.write(i.address, struct.pack('<I', STUB + 0x250))
            if i.name == b'CreateFileA':
                e.write(i.address, struct.pack('<I', STUB + 0x290))
    try:
        if full:
            def callback(which):
                def cb(emu):
                    if events is not None:
                        events.append((which, [emu.arg(k) for k in range(4)]))
                    return 0
                return cb
            if 'WATCH' in globals():
                WATCH(e)
            e.hook_ret(0x4d76b0, callback('player'))
            e.hook_ret(0x4d8b60, callback('other'))
            path = e.heap_next
            e.heap_next += 0x100
            e.write(path, prefix + bytes(1))
            e.call(0x435730, [path, 0x4d76b0, 0x4d8b60], 'iii', max_insn=8_000_000_000)
        else:
            e.call(0x406600, max_insn=4_000_000_000)
            for fn in (0x403110, 0x406340, 0x4842a0):   # as 0x435730 does next: numbering, roads, route caches
                e.call(fn, max_insn=400_000_000)
    except Exception as ex:
        print('failed', ex, [hex(a) for a in last], 'game', [hex(a) for a in game], 'esp', hex(e.mu.reg_read(UC_X86_REG_ESP)))
        raise


    return e


def save(e, path):
    """Write all mapped memory and the heap pointer of `e` to `path`."""
    import pickle
    regions = [(b, bytes(e.mu.mem_read(b, en - b + 1))) for b, en, _ in e.mu.mem_regions()]
    with open(path, 'wb') as f:
        pickle.dump({'heap_next': e.heap_next, 'regions': regions}, f)


def restore(e, path):
    import pickle
    with open(path, 'rb') as f:
        st = pickle.load(f)
    mapped = {b for b, _, _ in e.mu.mem_regions()}
    for b, data in st['regions']:
        if b not in mapped:
            e.mu.mem_map(b, len(data))
        e.mu.mem_write(b, data)
    e.heap_next = st['heap_next']


if __name__ == "__main__":
    folder, out = sys.argv[1], sys.argv[2]
    e = load(folder)
    u32 = lambda a:     struct.unpack('<I', e.read(a, 4))[0]
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

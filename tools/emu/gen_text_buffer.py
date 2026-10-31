"""Text buffer appends run unmodified: 0x4018b0 (thiscall, C string, stops at capacity) and 0x401870 (thiscall, bytes, count)."""
import pathlib, random, struct
from emu import Emu, OBJ_BASE

r = random.Random(0x4018b0)
e = Emu()
T, D, S = OBJ_BASE + 0x10000, OBJ_BASE + 0x11000, OBJ_BASE + 0x12000
rows = []
for case in range(300):
    which = r.choice(['4018b0', '401870'])
    cap = r.choice([0, 4, 8, 16, 32])
    length = r.randrange(0, max(cap, 1) + 2) if cap else 0
    has = r.choice([1, 1, 1, 0])
    data = bytes(r.randrange(1, 256) for _ in range(64))
    e.write(T, bytes(0x20)); e.set_u32(T + 8, cap); e.set_u32(T + 0xc, D if has else 0); e.set_u32(T + 0x10, length)
    e.write(D, data)
    n = r.choice([0, 1, 3, 4, 5, 9, 20])
    src = bytes(r.randrange(1, 256) for _ in range(n)) + bytes(1) + bytes(r.randrange(1, 256) for _ in range(40))
    e.write(S, src)
    if which == '4018b0':
        e.call(0x4018b0, [S], 'i', this=T)
    else:
        e.call(0x401870, [S, n], 'ii', this=T)
    rows.append(chr(9).join([which, str(cap), str(length), str(has), data.hex(), src.hex(), str(n), e.read(T, 0x14).hex(), e.read(D, 64).hex()]))
pathlib.Path(__file__).with_name('golden_text_buffer.tsv').write_text(chr(10).join(rows) + chr(10))
print(len(rows), 'cases')

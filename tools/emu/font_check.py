"""Compare atlas extraction with original 0x5e8e20. Never writes glyph assets to the repo.

Texture loading is supplied from the input FON; original metrics/mask generation executes
in Unicorn. Requires the release font_info example. Usage: python font_check.py <game dir>
"""
import pathlib
import struct
import subprocess
import sys
import random
from emu import Emu, OBJ_BASE, STUB

root = pathlib.Path(sys.argv[1])
example = pathlib.Path(sys.argv[2]).resolve() if len(sys.argv) > 2 else pathlib.Path(__file__).resolve().parents[2] / 'target/release/examples/font_info.exe'
for path in sorted(root.iterdir()):
    if path.suffix.lower() != '.fon':
        continue
    data = path.read_bytes()
    w, h = struct.unpack_from('<HH', data, 12)
    indexed = data[1:3] == b'\1\1'
    bpp = 1 if indexed else 2
    start = 18 + data[0] + struct.unpack_from('<H', data, 5)[0] * (data[7] // 8) if indexed else 30
    pixels = data[start:start + w * h * bpp]
    if not data[17] & 0x20:
        pixels = b''.join(pixels[y * w * bpp:(y + 1) * w * bpp] for y in reversed(range(h)))
    e = Emu()
    e.install_crt_hooks()
    texture, table, bitmap = OBJ_BASE, OBJ_BASE + 0x1000, OBJ_BASE + 0x2000
    e.set_u32(texture, table)
    e.set_u32(table, STUB + 0x400)
    e.write(STUB + 0x400, b'\xc2\x04\0')
    e.set_u32(texture + 8, bitmap)
    e.write(texture + 20, struct.pack('<HHH', w, h, bpp))
    e.write(bitmap, pixels)
    e.set_u32(0x720908, 0)
    e.hook_ret(0x5e7770, lambda emu: texture)
    e.call(0x5e8e20, [0], 'i', max_insn=50_000_000)
    font = e.eax
    u = lambda a: struct.unpack('<I', e.read(a, 4))[0]
    cw, gh = u(font + 24), u(font + 20)
    stride = (cw + 7) // 8
    advance, left, masks = u(font + 12), u(font + 16), u(font + 28)
    rows = subprocess.check_output([str(example), str(path), '--dump'], text=True).splitlines()
    assert len(rows) == 256
    for code, line in enumerate(rows):
        got_code, got_left, got_advance, got_mask = line.split()
        mask = e.read(masks + code * stride * gh, stride * gh)
        expected = ''.join('1' if mask[y * stride + x // 8] & (0x80 >> (x % 8)) else '0' for y in range(gh) for x in range(cw))
        assert int(got_code) == code
        assert int(got_left) == e.read(left + code, 1)[0], (path.name, code, 'left')
        assert int(got_advance) == e.read(advance + code, 1)[0], (path.name, code, 'advance')
        assert got_mask == expected, (path.name, code, 'mask')
    print(f'{path.name}: 256 glyph metrics and masks exact ({cw}x{gh})', flush=True)
    rng = random.Random(0x5ea0e0)
    strings = [b'', b'\n', b'A\nB\n', b'A\0ignored', bytes(range(1, 256))]
    strings += [bytes(rng.randrange(256) for _ in range(rng.randrange(200))) for _ in range(100)]
    measured = subprocess.check_output([str(example), str(path), '--measure'],
        input=''.join(s.hex() + '\n' for s in strings), text=True).splitlines()
    text = OBJ_BASE + 0x100000
    for string, widths in zip(strings, measured, strict=True):
        e.write(text, string + b'\0')
        for flag, width in zip((1, 0), map(int, widths.split()), strict=True):
            e.set_u32(font, flag)
            e.call(0x5ea0e0, [text], 'i', this=font)
            assert e.eax == width, (path.name, string.hex(), flag, width, e.eax)
    print(f'{path.name}: {len(strings) * 2} string widths exact', flush=True)
    surface, output = OBJ_BASE + 0x200000, OBJ_BASE + 0x210000
    e.write(surface, struct.pack('<HHHH', 80, 64, 1, 80))
    e.set_u32(surface + 24, output)
    e.hook_ret(0x5d3b10, lambda emu: 1)
    e.hook_thiscall(0x5b6710, lambda emu: 0, 1)
    cases = [(x, y, limit, flag, s) for x, y, limit in
        [(3, 40, 100), (-2, 40, 100), (70, 40, 100), (3, 0, 100), (3, 63, 100), (3, 40, 7)]
        for flag in (0, 1) for s in (b'ABC', b'A\nB', b' A\0B', bytes([192, 193, 255]), b'')]
    cases += [(rng.randrange(-30, 100), rng.randrange(-30, 90), rng.randrange(-10, 140),
        rng.randrange(2), bytes(rng.randrange(256) for _ in range(rng.randrange(40)))) for _ in range(300)]
    drawn = subprocess.check_output([str(example), str(path), '--draw'],
        input=''.join(f'{x} {y} {limit} {flag} {s.hex()}\n' for x, y, limit, flag, s in cases), text=True).splitlines()
    for (x, y, limit, flag, string), got in zip(cases, drawn, strict=True):
        e.set_u32(font, flag)
        e.write(output, bytes(80 * 64))
        e.write(text, string + b'\0')
        e.call(0x5ea1b0, [surface, x, y, limit, font, text, 1, 1], 'iiiiiiii')
        expected = ''.join(str(v) for v in e.read(output, 80 * 64))
        assert got == expected, (path.name, x, y, limit, flag, string.hex(), next((i for i, (a,b) in enumerate(zip(got, expected)) if a != b), None))
    print(f'{path.name}: {len(cases)} software text rasters exact', flush=True)

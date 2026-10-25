"""Compare atlas extraction with original 0x5e8e20. Never writes glyph assets to the repo.

Texture loading is supplied from the input FON; original metrics/mask generation executes
in Unicorn. Requires the release font_info example. Usage: python font_check.py <game dir>
"""
import pathlib
import struct
import subprocess
import sys
from emu import Emu, OBJ_BASE, STUB

root = pathlib.Path(sys.argv[1])
example = pathlib.Path(__file__).resolve().parents[2] / 'target/release/examples/font_info.exe'
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

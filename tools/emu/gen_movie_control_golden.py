"""Movie completion and character handling, executed in king.exe without UI/media."""
import pathlib
from emu import Emu, OBJ_BASE, STUB
e = Emu()
obj, message, table = OBJ_BASE, OBJ_BASE + 0x1000, OBJ_BASE + 0x2000
e.set_u32(obj, table)
e.set_u32(table + 0x20, STUB + 0x400)
e.write(STUB + 0x400, b'\xc3')
for a in (0x5c2450, 0x5c12a0, 0x5d88c0):
    e.hook_thiscall(a, lambda emu: 0, 0)
e.hook_thiscall(0x5d8630, lambda emu: 0, 1)
e.set_u32(0x69684c, obj)
menus = []
def menu(emu):
    menus.append(emu.arg(0))
    return 0
e.hook_thiscall(0x51f380, menu, 1)
root = pathlib.Path(__file__).resolve().parents[2] / 'crates/ht2-sim/tests/data'
rows = []
for mode in (2, 3, 4):
    menus.clear()
    e.set_u32(obj + 0x900, 7)
    e.set_u32(obj + 0x904, mode)
    e.call(0x528ca0, this=obj)
    assert len(menus) == 1
    rows.append(f'{mode} {menus[0]}')
(root / 'movie_completion_golden.txt').write_text('\n'.join(rows) + '\n', encoding='utf-8')
rows = []
for mode in range(7):
    for kind in (0x100, 0x101, 0x102, 0x201):
        for char in (0, 13, 27, 32, 65):
            e.set_u32(obj + 0x900, 0)
            e.set_u32(obj + 0x904, mode)
            e.write(message, bytes(16))
            e.write(message, kind.to_bytes(2, 'little'))
            e.write(message + 6, char.to_bytes(2, 'little'))
            e.call(0x529020, [message], 'i', this=obj)
            state = int.from_bytes(e.read(obj + 0x900, 4), 'little')
            rows.append(f'{mode} {kind} {char} {e.eax} {int(state == 7)}')
(root / 'movie_character_golden.txt').write_text('\n'.join(rows) + '\n', encoding='utf-8')
print('3 completion modes; 140 input cases')

"""Run the original game callback 0x4d76b0 for victory event 12.

Observe movie requests or menu transitions, without launching UI or playing media.
The player-info query is unrelated to this branch and is stubbed.
"""
import pathlib
from emu import Emu, OBJ_BASE
e = Emu()
human, other, game = OBJ_BASE, OBJ_BASE + 0x1000, OBJ_BASE + 0x2000
e.set_u32(0x696270, 7)
e.set_u32(0x696330, 0)
e.set_u32(0x69632c, 0)
e.set_u32(0x6cec6c, game)
e.hook_ret(0x43fc40, lambda emu: 0)
flags = 0
e.hook_ret(0x43f620, lambda emu: flags)

def find_human(emu):
    emu.set_u32(emu.arg(0), human)
    return emu.arg(0)

actions = []
def movie(emu):
    name = e.read(emu.arg(0), 32).split(b'\0')[0].decode('ascii')
    actions.append((name, emu.arg(1)))
    return 0
def menu(emu):
    actions.append(('menu', emu.arg(0)))
    return 0
e.hook_ret(0x568f60, find_human)
e.hook_ret(0x5279e0, movie)
e.hook_thiscall(0x51f380, menu, 1)
rows = []
for is_human in (0, 1):
    for flags in (0, 1, 0x10000, 0x10001, 0xffffffff):
        actions.clear()
        e.call(0x4d76b0, [human if is_human else other, 12, 0], 'iii')
        assert len(actions) == 1
        rows.append(f'{is_human} {flags} {actions[0][0]} {actions[0][1]} {e.eax}')
path = pathlib.Path(__file__).resolve().parents[2] / 'crates/ht2-sim/tests/data/victory_action_golden.txt'
path.write_text('\n'.join(rows) + '\n', encoding='utf-8')
print('\n'.join(rows))

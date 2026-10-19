"""A stand-in for the scene-graph collision call (vtable +0x34), mirrored by
ht2_sim::contact::StubWorld. The object at `obj` holds:
  +0  vtable pointer, +8 f32 ground height, +0xc surface record, +0x10 room, +0x14 extra flags."""
import struct

from emu import STUB

WORLD_FN = STUB + 0x100


def f32(x):
    return struct.unpack('<f', struct.pack('<f', x))[0]


def collide(e):
    return _collide(e, e.ecx, e.arg(1))


def _collide(e, obj, rec):
    rd = lambda a: struct.unpack('<f', bytes(e.read(a, 4)))[0]
    ru = lambda a: struct.unpack('<I', bytes(e.read(a, 4)))[0]
    z0 = rd(obj + 8)
    s = [rd(rec + 0x4c + 4 * i) for i in range(3)]
    t = [rd(rec + 0x58 + 4 * i) for i in range(3)]
    extra = ru(obj + 0x14) & 2
    if extra:
        e.write(rec + 0x68, struct.pack('<I', ru(obj + 0x10)))
    if t[2] < z0 <= s[2]:
        k = f32((s[2] - z0) / (s[2] - t[2]))
        hit = [f32(s[i] + f32(f32(t[i] - s[i]) * k)) for i in range(3)]
        hit[2] = z0
        for i in range(3):
            e.write(rec + 0x34 + 4 * i, struct.pack('<f', hit[i]))
        e.write(rec + 0x40, struct.pack('<fff', 0.0, 0.0, 1.0))
        e.write(rec + 0x64, struct.pack('<I', ru(obj + 0xc)))
        return 1 | extra
    return extra


def install(e):
    e.hook_thiscall(WORLD_FN, collide, 2)


def make_obj(obj, vtbl, z0, surf, room, extra):
    """Memory writes for a stub world object at `obj` with its vtable at `vtbl`."""
    return [(vtbl + 0x34, struct.pack('<I', WORLD_FN)), (obj, struct.pack('<I', vtbl)),
            (obj + 8, struct.pack('<f', z0)), (obj + 0xc, struct.pack('<I', surf)),
            (obj + 0x10, struct.pack('<I', room)), (obj + 0x14, struct.pack('<I', extra))]


def room_query(e):
    """`0x4e6059` / `0x4e623e` (cdecl room, mode, rec): the stub object is the room itself."""
    return _collide(e, e.arg(0), e.arg(2))


def install_rooms(e):
    e.hook_ret(0x4e6059, room_query)
    e.hook_ret(0x4e623e, room_query)

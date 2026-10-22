"""The objects of a new game (0x41d340: hidden places, bases, semitrailers on the roads, presents,
parkings) as the original makes them, for comparing the port. Starts from the snapshot after the
vehicle lists (newgame_steps.LISTS, made by `run(folder, 0x42fa90, distances=True)`), makes the
players (0x423c00) and then the objects.

    python objects_check.py <game folder>/ENV > objects.txt
"""
import os
import struct
import sys

from newgame_steps import run

folder = sys.argv[1]
e = run(folder, 0x42fa90, distances=True)
u = lambda a: struct.unpack('<I', e.read(a, 4))[0]
i32 = lambda a: struct.unpack('<i', e.read(a, 4))[0]
f64 = lambda a: struct.unpack('<d', e.read(a, 8))[0]


def rng():
    return ' '.join(str(x) for x in struct.unpack('<55I', e.read(0x68b9c8, 220))) + ' %d %d' % (u(0x68baa4), u(0x68baa8))


# the game gives each vehicle of the catalogue its vehicle.tech record (+0x40, by the model
# prefix) and has the records in memory; the AI reads the capacities from there
e.load_tech()
tech = open(os.path.join(os.path.dirname(os.path.normpath(folder)), 'vehicle.tech'), 'rb').read()
nlen = struct.unpack_from('<I', tech, 0)[0]
count = struct.unpack_from('<I', tech, 4 + nlen + 8)[0]
prefixes = [tech[57 + i * 0x970 + 2302: 57 + i * 0x970 + 2322].split(b'\0')[0].decode('latin1') for i in range(count)]
vb = u(0x68cfd8)
for k in range((u(0x68cfdc) - vb) // 4):
    v = u(vb + 4 * k)
    model = e.read(u(v + 0x14), 32).split(b'\0')[0].decode('latin1')
    if model in prefixes:
        e.set_u32(v + 0x40, prefixes.index(model))
e.call(0x423c00, max_insn=10**12)
print('rng', rng(), flush=True)
for name, want, have in (('hidden', 0x68a98c, 0x68a968), ('base', 0x68cf54, 0x68d01c), ('road', 0x68d0a0, 0x68d0e4),
                         ('presents', 0x68a964, 0x68cfe4), ('parking', 0x68bb54, 0x68cf34)):
    print('count', name, i32(want), i32(have), flush=True)
e.call(0x41d340, (0.0,), 'd', max_insn=10**12)
for name, want, have in (('hidden', 0x68a98c, 0x68a968), ('base', 0x68cf54, 0x68d01c), ('road', 0x68d0a0, 0x68d0e4),
                         ('presents', 0x68a964, 0x68cfe4), ('parking', 0x68bb54, 0x68cf34)):
    print('count', name, i32(want), i32(have), flush=True)
head = u(0x68cf6c)
n = u(head)
k = 0
while n != head:
    o = u(n + 8)
    sub = u(o + 0x14)
    print('object', k, 'id', i32(o), 'flags', hex(u(o + 8)), 'node', i32(o + 0x18), 'pos', repr(f64(o + 0x20)), repr(f64(o + 0x28)), repr(f64(o + 0x30)),
          'place', i32(o + 0x38), i32(o + 0x3c), repr(f64(o + 0x40)), i32(o + 0x48), i32(o + 0x4c), i32(o + 0x50),
          'sub', (i32(sub), i32(sub + 4), repr(f64(sub + 8))) if sub else None, 'x58', hex(u(o + 0x58)), 'x6c', i32(o + 0x6c), flush=True)
    if u(o + 0x58):
        c = u(o + 0x58)
        print('  cargo', i32(c), i32(c + 4), i32(c + 8), i32(c + 12), repr(f64(c + 0x10)), repr(f64(c + 0x18)), flush=True)
    if sub:
        for a in range(u(sub + 0x1c), u(sub + 0x20), 0x90):
            print('  order', ' '.join(hex(u(a + 4 * i)) for i in range(36)), flush=True)
    n = u(n)
    k += 1
print('rng', rng(), flush=True)
# the towns' nodes (+0x1c, 0x10 bytes): name, flags and two pointers
for t in range((u(0x68cec4) - u(0x68cec0)) // 4):
    town = u(u(0x68cec0) + 4 * t)
    print('town', t, [(hex(u(a + 4)), hex(u(a + 8)), hex(u(a + 12))) for a in range(u(town + 0x1c), u(town + 0x20), 0x10)], [i32(a) for a in range(u(town + 0x28), u(town + 0x2c), 4)], flush=True)

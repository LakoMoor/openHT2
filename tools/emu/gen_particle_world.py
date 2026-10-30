"""Whole particle emission chain of the original with real pools: 0x4ba660 (exhaust) and 0x4baf90 (flash)
run unmodified (allocation 0x579380, root constructor 0x59a780, class constructors); only rand,
malloc and error logging are replaced. The pools, materials and option scalars are laid out at the
addresses `ParticleWorld::new` produces, so the Rust side compares object bytes slot by slot.
Pointers into the heaps differ between the two sides and are masked by the test."""
import pathlib, random, struct
from emu import Emu, OBJ_BASE, HEAP_BASE

CAP = 64
POOLS = [0x6f6a7c, 0x6f6a80, 0x6f6a88, 0x6f6a8c, 0x6f6a98, 0x6f6a9c]
SLOT = 0xe0
r = random.Random(0x4ba660)
e = Emu()
seed = [12345]


def rand(em):
    # the MSVC LCG, as globals::rand with RAND_FIXED = 0 and RAND_SEED = 12345
    seed[0] = (seed[0] * 214013 + 2531011) & 0xffffffff
    return (seed[0] >> 16) & 0x7fff


def malloc(em):
    a = em.heap_next
    em.heap_next += (em.arg(0) + 15) & ~15
    return a


e.hook_ret(0x632643, malloc)
e.hook_ret(0x4e260c, lambda em: 0)
e.hook_ret(0x61edde, rand)
e.heap_next = 0x2a800000   # objects built by the original's `new`; masked on comparison

u32 = lambda a: struct.unpack('<I', e.read(a, 4))[0]
nxt = [HEAP_BASE]


def alloc(n):
    a = nxt[0]
    nxt[0] += (n + 15) & ~15
    return a


def f32(a, v):
    e.write(a, struct.pack('<f', v))


# globals.install_constants
for addr, v in ((0x702620, (0, 1, 0)), (0x702630, (0, 0, 1)), (0x702640, (1, 0, 0)), (0x702650, (0, 0, 0))):
    e.write(addr, struct.pack('<3f', *v))
ident = (1, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0)
e.write(0x7025f0, struct.pack('<12f', *ident))
import math
for i in range(360):
    x = i * 3.1415926
    f32(0x696ec0 + 4 * i, math.sin((x + x) / 360.0))

manager = alloc(0x1b4); sort = alloc(0x40); slots = alloc(4); clock = alloc(0xc00); scratch = alloc(0x400); inp = alloc(48)
# ThatSortNode: the real constructor 0x593040 (vtable 0x6508a8) runs; the root constructor then
# registers every particle through its virtual slot +0x14.
e.write(sort + 0x100, b'ThatSortNode' + bytes(1))
e.call(0x593040, [sort + 0x100], 'i', this=sort)
e.set_u32(0x6f6a68, sort); e.set_u32(0x696224, clock); e.set_u32(manager + 0x1b0, 6)
pools = []
for i, a in enumerate(POOLS):
    pool = alloc(16); prev = 0
    for _ in range(CAP):
        o = alloc(SLOT); e.set_u32(o + 16, pool); e.set_u32(o + 8, prev)
        if prev:
            e.set_u32(prev + 12, o)
        prev = o
    e.set_u32(pool + 4, prev); e.set_u32(a, pool); e.set_u32(manager + 0x20 + i * 4, pool); pools.append(pool)
# EffectMaterials in order Smoke, Fume, Soot, Dust: (table, count address, number of materials of the test world)
for table, count_at, n in ((0x695bf0, 0x696208, 10), (0x695c58, 0x69620c, 10), (0x694998, 0x696210, 9), (0x694a00, 0x696218, 10)):
    for k in range(n):
        p = alloc(0x60); e.set_u32(table + 4 * k, p); f32(p + 0x58, 1.0); e.set_u32(p + 0x20, 0x40220)
    e.set_u32(count_at, n)
f32(0x69603c, 1.0); f32(0x696048, 0.0); f32(0x696054, 1.0); e.set_u32(0x671700, 200); f32(0x694830, 3.0)
for i, x in enumerate((0.0, 0.0, 1.0)):
    f32(0x702640 + 4 * i, x)
sky = alloc(0x240); f32(sky + 0x220, 0.5); e.write(sky + 0x78, struct.pack('<d', 0.0)); e.write(sky + 0x68, struct.pack('<d', 0.0)); e.set_u32(0x6f33ac, sky)


def dump():
    out = []
    for pool in pools:
        objs = []
        o = u32(pool + 8)
        while o:
            objs.append(e.read(o, SLOT).hex())
            o = u32(o + 8)
        out.append(','.join(objs))
    return '|'.join(out)


F = lambda *v: struct.pack('<%df' % len(v), *v)
fbits = lambda x: struct.unpack('<I', struct.pack('<f', x))[0]
rows = []
import os
if os.environ.get('HT2_EMU_TRACE'):
    from collections import deque
    from unicorn import UC_HOOK_BLOCK, UC_HOOK_MEM_READ_UNMAPPED
    last = deque(maxlen=10)
    e.mu.hook_add(UC_HOOK_BLOCK, lambda uc, a, sz, u: last.append(a))
    from unicorn import UC_HOOK_MEM_READ_UNMAPPED
    def bad(uc, access, addr, size, value, user):
        from unicorn.x86_const import UC_X86_REG_EIP, UC_X86_REG_ESI, UC_X86_REG_ECX, UC_X86_REG_EBX
        print('unmapped read', hex(addr), 'eip', hex(uc.reg_read(UC_X86_REG_EIP)), 'esi', hex(uc.reg_read(UC_X86_REG_ESI)), 'ecx', hex(uc.reg_read(UC_X86_REG_ECX)), 'ebx', hex(uc.reg_read(UC_X86_REG_EBX)))
        return False
    e.mu.hook_add(UC_HOOK_MEM_READ_UNMAPPED, bad)
pos, dirp = OBJ_BASE + 0x100, OBJ_BASE + 0x120
try:
    for case in range(150):
        if case % 3 == 2:
            ps = F(*[r.uniform(-9, 9) for _ in range(3)]); size = r.choice([0.5, 1.0, 1.5, 2.0])
            e.write(pos, ps)
            e.call(0x4baf90, [pos, fbits(size), 1], 'iii')
            rows.append(chr(9).join(['flash', ps.hex(), F(size).hex(), dump()]))
        else:
            p = r.choice([r.uniform(0.35, 1.3), 0.4, 0.7, 1.0, 0.0]); g = r.choice([0.5, 0.9, 1.0, 0.0])
            d = F(*[r.uniform(-2, 2) for _ in range(3)]); ps = F(*[r.uniform(-9, 9) for _ in range(3)])
            e.write(pos, ps); e.write(dirp, d); f32(0x696048, g)
            e.call(0x4ba660, [pos, dirp, fbits(p), 1], 'iiii')
            rows.append(chr(9).join(['exhaust', ps.hex(), d.hex(), F(p).hex(), F(g).hex(), dump()]))
except Exception:
    if os.environ.get('HT2_EMU_TRACE'):
        print('case', case, [hex(x) for x in last])
    raise
pathlib.Path(__file__).with_name('golden_particle_world.tsv').write_text(chr(10).join(rows) + chr(10))
print(len(rows), 'ops;', 'live', sum(len([x for x in s.split(',') if x]) for s in rows[-1].split(chr(9))[-1].split('|')))

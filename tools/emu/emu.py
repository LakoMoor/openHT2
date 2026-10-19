"""Run functions of the original king.exe in an x86 emulator (for differential testing of ports).

Only for analysis on a machine that owns the game; nothing here is shipped.

    e = Emu()
    e.load_tech()                         # copy vehicle.tech records into the engine's table
    e.call(0x507330, [1.5, 0.1], kinds='ff')   # cdecl-style call, returns ST0 as float
"""
import struct

import pefile
from unicorn import Uc, UC_ARCH_X86, UC_MODE_32, UC_HOOK_MEM_UNMAPPED, UC_HOOK_CODE
from unicorn.x86_const import (UC_X86_REG_ESP, UC_X86_REG_EBP, UC_X86_REG_ECX, UC_X86_REG_EDX, UC_X86_REG_EAX,
                               UC_X86_REG_EIP, UC_X86_REG_FPCW, UC_X86_REG_FS_BASE)

GAME = r'E:\openHT2\Hard Truck 2 King of the Road'
EXE = GAME + r'\king.exe'
TECH_BASE = 0x697868  # engine table of vehicle.tech records, 0x970 bytes each
STACK_TOP = 0x30000000
SCRATCH = 0x20000000
STUB = 0x1f000000
HEAP_BASE = 0x28000000   # bump allocator used by the hooked malloc (mirrors ht2-sim Mem::alloc)
OBJ_BASE = 0x2a000000    # free area for test objects


class Emu:
    def __init__(self):
        self.pe = pefile.PE(EXE)
        self.mu = Uc(UC_ARCH_X86, UC_MODE_32)
        base = self.pe.OPTIONAL_HEADER.ImageBase
        self.mu.mem_map(base, 0x1000)
        self.mu.mem_write(base, self.pe.header[:0x400])
        for s in self.pe.sections:
            va = base + s.VirtualAddress
            size = (max(s.Misc_VirtualSize, s.SizeOfRawData) + 0xfff) & ~0xfff
            self.mu.mem_map(va, size)
            self.mu.mem_write(va, s.get_data()[: s.SizeOfRawData])
        self.mu.mem_map(SCRATCH, 0x1000000)
        self.mu.mem_map(STACK_TOP - 0x100000, 0x100000)
        self.mu.mem_map(STUB, 0x1000)
        self.mu.mem_map(HEAP_BASE, 0x1000000)
        self.mu.mem_map(OBJ_BASE, 0x1000000)
        self.heap_next = HEAP_BASE
        # return stub: fstp qword [SCRATCH]; nop
        self.mu.mem_write(STUB, b'\xdd\x1d' + struct.pack('<I', SCRATCH) + b'\x90')
        self.stub_end = STUB + 7
        self.mu.reg_write(UC_X86_REG_FPCW, 0x27f)  # 53-bit mantissa, round to nearest
        self._patch_fs(0x7ffd0000)

    def _patch_fs(self, teb):
        """Give the emulated CPU a flat GDT and an `fs` segment that points at a fake thread block.

        The original code keeps its SEH chain at `fs:[0]`; unicorn has no TEB of its own."""
        from unicorn.x86_const import (UC_X86_REG_GDTR, UC_X86_REG_FS, UC_X86_REG_CS, UC_X86_REG_DS,
                                       UC_X86_REG_ES, UC_X86_REG_SS, UC_X86_REG_GS)
        self.mu.mem_map(teb, 0x1000)
        gdt = 0x7ffe0000
        self.mu.mem_map(gdt, 0x1000)

        def entry(base, limit, access, flags):
            v = limit & 0xffff
            v |= (base & 0xffffff) << 16
            v |= (access & 0xff) << 40
            v |= ((limit >> 16) & 0xf) << 48
            v |= (flags & 0xf) << 52
            v |= ((base >> 24) & 0xff) << 56
            return struct.pack('<Q', v)

        table = entry(0, 0, 0, 0)                       # null
        table += entry(0, 0xfffff, 0x9a, 0xc)           # 1: flat code
        table += entry(0, 0xfffff, 0x92, 0xc)           # 2: flat data / stack
        table += entry(teb, 0xfff, 0x92, 0x4)           # 3: fs -> thread block
        self.mu.mem_write(gdt, table)
        self.mu.reg_write(UC_X86_REG_GDTR, (0, gdt, len(table) - 1, 0))
        self.mu.reg_write(UC_X86_REG_DS, 2 << 3)
        self.mu.reg_write(UC_X86_REG_ES, 2 << 3)
        self.mu.reg_write(UC_X86_REG_SS, 2 << 3)
        self.mu.reg_write(UC_X86_REG_CS, 1 << 3)
        self.mu.reg_write(UC_X86_REG_GS, 2 << 3)
        self.mu.reg_write(UC_X86_REG_FS, 3 << 3)

    # --- hooks ----------------------------------------------------------
    def hook_ret(self, addr, fn):
        """Replace the function at `addr` (cdecl, caller cleans) with `fn(emu) -> eax`."""
        def cb(uc, address, size, user):
            sp = uc.reg_read(UC_X86_REG_ESP)
            ret = struct.unpack('<I', uc.mem_read(sp, 4))[0]
            uc.reg_write(UC_X86_REG_EAX, fn(self) & 0xffffffff)
            uc.reg_write(UC_X86_REG_ESP, sp + 4)
            uc.reg_write(UC_X86_REG_EIP, ret)
        self.mu.hook_add(UC_HOOK_CODE, cb, begin=addr, end=addr)

    def hook_thiscall(self, addr, fn, nargs):
        """Replace a __thiscall function (callee pops `nargs` stack words) with `fn(emu) -> eax`."""
        def cb(uc, address, size, user):
            sp = uc.reg_read(UC_X86_REG_ESP)
            ret = struct.unpack('<I', uc.mem_read(sp, 4))[0]
            uc.reg_write(UC_X86_REG_EAX, fn(self) & 0xffffffff)
            uc.reg_write(UC_X86_REG_ESP, sp + 4 + 4 * nargs)
            uc.reg_write(UC_X86_REG_EIP, ret)
        self.mu.hook_add(UC_HOOK_CODE, cb, begin=addr, end=addr)

    @property
    def ecx(self):
        return self.mu.reg_read(UC_X86_REG_ECX)

    def arg(self, n):
        """n-th (0-based) stack argument inside a hook."""
        sp = self.mu.reg_read(UC_X86_REG_ESP)
        return struct.unpack('<I', self.mu.mem_read(sp + 4 + 4 * n, 4))[0]

    def install_crt_hooks(self):
        def malloc(emu):
            size = emu.arg(0)
            a = emu.heap_next
            emu.heap_next += (size + 15) & ~15
            return a
        self.hook_ret(0x632643, malloc)          # malloc
        self.hook_ret(0x4e260c, lambda emu: 0)   # error log / assert
        self.hook_ret(0x61edde, lambda emu: 12345)  # rand(): fixed so that results are repeatable
        self.hook_ret(0x61cca9, lambda emu: 0)   # sprintf: messages are not needed (float formatting needs CRT start-up state)

    # --- memory helpers -------------------------------------------------
    def write(self, va, data):
        self.mu.mem_write(va, bytes(data))

    def read(self, va, n):
        return bytes(self.mu.mem_read(va, n))

    def f32(self, va):
        return struct.unpack('<f', self.read(va, 4))[0]

    def set_f32(self, va, v):
        self.write(va, struct.pack('<f', v))

    def set_u32(self, va, v):
        self.write(va, struct.pack('<I', v & 0xffffffff))

    def load_tech(self, path=GAME + r'\vehicle.tech'):
        d = open(path, 'rb').read()
        n = struct.unpack_from('<I', d, 0)[0]
        rec = struct.unpack_from('<I', d, 4 + n + 4)[0]
        count = struct.unpack_from('<I', d, 4 + n + 8)[0]
        assert rec == 0x970
        for i in range(count):
            self.write(TECH_BASE + i * rec, d[57 + i * rec: 57 + (i + 1) * rec])

    # --- calling --------------------------------------------------------
    def call(self, addr, args=(), kinds='', this=None, edx=None, max_insn=2_000_000):
        """Call a function. `kinds` has one letter per argument: f=float32, d=float64, i=int32.
        `this` is passed in ECX (__thiscall/__fastcall), `edx` in EDX. Returns ST0 as a float."""
        mu = self.mu
        sp = STACK_TOP - 0x1000
        stack = b''
        for a, k in zip(args, kinds):
            stack += struct.pack({'f': '<f', 'd': '<d', 'i': '<I'}[k], a if k != 'i' else a & 0xffffffff)
        sp -= len(stack) + 4
        mu.mem_write(sp, struct.pack('<I', STUB) + stack)
        mu.reg_write(UC_X86_REG_ESP, sp)
        mu.reg_write(UC_X86_REG_EBP, 0)
        if this is not None:
            mu.reg_write(UC_X86_REG_ECX, this)
        if edx is not None:
            mu.reg_write(UC_X86_REG_EDX, edx)
        mu.reg_write(UC_X86_REG_FPCW, 0x27f)
        mu.mem_write(SCRATCH, b'\0' * 8)
        mu.emu_start(addr, self.stub_end, count=max_insn)
        return struct.unpack('<d', mu.mem_read(SCRATCH, 8))[0]

    @property
    def eax(self):
        return self.mu.reg_read(UC_X86_REG_EAX)


if __name__ == '__main__':
    e = Emu()
    e.load_tech()
    # FUN_00507330(a, b) = ((a+6)*(a/(a+20)))/(a+3+a+3)*b
    import math
    for a, b in [(0.3, 1.0), (1.5, 0.1), (10.0, 2.0)]:
        want = ((a + 6) * (a / (a + 20))) / (a + 3 + a + 3) * b
        got = e.call(0x507330, [a, b], 'ff')
        print(a, b, got, want, abs(got - want) < 1e-5)

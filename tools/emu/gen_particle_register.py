"""Actual sort-node registration 0x593270 -> 0x5f1cc0."""
import pathlib,random,struct
from emu import Emu,OBJ_BASE
from unicorn.x86_const import UC_X86_REG_EAX
r=random.Random(0x593270);e=Emu();rows=[];node=OBJ_BASE+0x100;slots=OBJ_BASE+0x200;old=OBJ_BASE+0x300
for case in range(200):
 raw=bytearray(r.getrandbits(8) for _ in range(0x400))
 for offset,value in [(0x30,slots),(0x34,case%3),(0x200,old if case%2 else 0)]:struct.pack_into('<I',raw,offset,value)
 e.write(OBJ_BASE,raw);e.call(0x593270,[r.getrandbits(32),node],'ii',this=OBJ_BASE)
 rows.append(raw.hex()+'\t'+e.read(OBJ_BASE,0x400).hex()+'\t'+str(e.mu.reg_read(UC_X86_REG_EAX)))
pathlib.Path(__file__).with_name('golden_particle_register.tsv').write_text('\n'.join(rows)+'\n');print('200 sort-node registrations captured')

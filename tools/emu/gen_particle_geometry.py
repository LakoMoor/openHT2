"""Root particle geometry before attachment; allocator and base identities controlled."""
import pathlib,random,struct
from emu import Emu,OBJ_BASE
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_ESI
r=random.Random(0x59a91d);e=Emu();rows=[];heap=[OBJ_BASE+0x20000];blocks=[];geometry=[0]
e.hook_thiscall(0x579310,lambda em:em.ecx,0);e.hook_thiscall(0x5fa6f0,lambda em:em.ecx,1);e.hook_ret(0x61dac3,lambda em:0)
def alloc(em):
 p=heap[0];heap[0]+=0x100;e.write(p,bytes(0x100));blocks.append(p);return p
e.hook_ret(0x632643,alloc)
def stop(mu,a,s,u):geometry[0]=mu.reg_read(UC_X86_REG_ESI);mu.emu_stop()
e.mu.hook_add(UC_HOOK_CODE,stop,begin=0x59a91d,end=0x59a91d)
for case in range(300):
 heap[0]=OBJ_BASE+0x20000;blocks.clear();x=r.uniform(-20,20);y=r.uniform(-20,20);color=r.getrandbits(32);e.set_u32(OBJ_BASE+0x1000,color)
 try:e.call(0x59a780,[0,0,0.,0.,0.,x,y,OBJ_BASE+0x1000,1,0.,0.],'iifffffiiff',this=OBJ_BASE)
 except RuntimeError:pass
 assert len(blocks)==5
 rows.append(','.join(map(str,[struct.unpack('<I',struct.pack('<f',x))[0],struct.unpack('<I',struct.pack('<f',y))[0],color]))+'\t'+e.read(geometry[0],40).hex()+'\t'+''.join(e.read(p,16).hex() for p in blocks[1:]))
pathlib.Path(__file__).with_name('golden_particle_geometry.tsv').write_text('\n'.join(rows)+'\n');print('300 particle geometries captured')

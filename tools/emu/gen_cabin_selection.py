"""Capture first cabin lookup of actual 0x4dde70 from synthetic tech records."""
import pathlib,random,string
from emu import Emu,OBJ_BASE,TECH_BASE
from unicorn import UC_HOOK_CODE
e=Emu();e.install_crt_hooks();r=random.Random(0x4dde70);c=OBJ_BASE;car=c+0x10000;rows=[];captured=[]
e.set_u32(0x6cec68,1)
def lookup(mu,address,size,user):
 captured.append(e.read(e.arg(1),100).split(b'\0')[0].decode('ascii'));mu.emu_stop()
e.mu.hook_add(UC_HOOK_CODE,lookup,begin=0x5ef2b0,end=0x5ef2b0)
for case in range(120):
 record=case%20;prefix='Body'+str(case);cabin='Cab'+''.join(r.choice(string.ascii_letters) for _ in range(r.randrange(1,15)))
 e.set_u32(car+0x28b4,record)
 e.write(TECH_BASE+record*0x970+2302,prefix.encode().ljust(20,b'\0'))
 e.write(TECH_BASE+record*0x970+2322,cabin.encode().ljust(20,b'\0'));captured.clear()
 try:e.call(0x4dde70,[car],'i',this=c)
 except RuntimeError:
  if not captured:raise
 assert len(captured)==1;rows.append(prefix+'\t'+cabin+'\t'+captured[0])
pathlib.Path(__file__).with_name('golden_cabin_selection.tsv').write_text('\n'.join(rows)+'\n');print('120 original cabin lookups captured')

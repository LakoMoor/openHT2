"""Original primary/fallback cabin lookups in 0x4dde70, before allocation."""
import pathlib,random,string
from emu import Emu,OBJ_BASE,TECH_BASE
e=Emu();e.install_crt_hooks();r=random.Random(0x4dde71);c=OBJ_BASE;car=c+0x10000;requests=[];rows=[];state={}
e.set_u32(0x6cec68,1);e.set_u32(car+0x28b4,0)
def lookup(q):
 name=q.read(q.arg(1),100).split(b'\0')[0].decode('ascii');requests.append(name)
 if len(requests)==1 and not state['first']:return 0
 q.mu.emu_stop();return 1 if len(requests)==1 else state['second']
e.hook_ret(0x5ef2b0,lookup)
for case in range(120):
 primary='Main'+str(case);fallback='Reserve'+str(case);first=case%2;second=(case//2)%2;boundary=r.randrange(2,15)
 e.set_u32(0x6ce338,boundary)
 e.write(TECH_BASE+2322,primary.encode().ljust(20,b'\0'))
 e.write(TECH_BASE+(boundary+1)*0x970+2322,fallback.encode().ljust(20,b'\0'))
 requests.clear();state['first']=first;state['second']=second
 try:e.call(0x4dde70,[car],'i',this=c)
 except RuntimeError:
  if not requests:raise
 assert len(requests)==(1 if first else 2)
 assert e.eax==(1 if first else second)
 rows.append('\t'.join([primary,fallback,str(first),str(second),','.join(requests),primary if first else fallback if second else '',str(boundary)]))
pathlib.Path(__file__).with_name('golden_cabin_fallback.tsv').write_text('\n'.join(rows)+'\n');print('120 original cabin fallback lookup sequences captured')

"""Original 0x507fa0 list reader; only Win32 profile/heap/locks replaced."""
import pathlib,random,struct
from emu import Emu,OBJ_BASE,STUB
e=Emu();e.install_crt_hooks()
e.hook_ret(0x63266c,lambda q:0)
for lib in e.pe.DIRECTORY_ENTRY_IMPORT:
 for imp in lib.imports:
  if imp.name==b'GetPrivateProfileStringA':
   e.set_u32(imp.address,STUB+0x600)
for lib in e.pe.DIRECTORY_ENTRY_IMPORT:
 for imp in lib.imports:
  if imp.name in [b'InitializeCriticalSection',b'DeleteCriticalSection',b'EnterCriticalSection',b'LeaveCriticalSection']:
   e.set_u32(imp.address,STUB+0x620)
e.hook_thiscall(STUB+0x620,lambda q:0,1)
def allocate(q):
 a=q.heap_next;q.heap_next+=(q.arg(2)+15)&~15;return a
e.set_u32(0x64b210,STUB+0x640);e.hook_thiscall(STUB+0x640,allocate,3)
state={};rows=[];r=random.Random(0x507fa0)
def profile(q):
 raw=state['value'].encode() if state['value'] is not None else b'off'
 raw=raw[:q.arg(4)-1];q.write(q.arg(3),raw+b'\0');return len(raw)
e.hook_thiscall(STUB+0x600,profile,6)
for case in range(120):
 values=[];outputs=[]
 for i,default in enumerate([0,10,380,430]):
  value=r.choice([None,'0','', ' '.join(str(r.randrange(-1000,1000)) for _ in range(r.randrange(1,8))), '# 7 # -8', '+1 -2 003'])
  values.append('<missing>' if value is None else value);state['value']=value
  e.call(0x507fa0,[0x672eb8,0x672fe4,default,OBJ_BASE,0x672ec8,5],'iiiiii')
  outputs.extend(struct.unpack('<5i',e.read(OBJ_BASE,20)))
 rows.append('\t'.join(values)+'\t'+','.join(map(str,outputs)))
pathlib.Path(__file__).with_name('golden_view_tables.tsv').write_text('\n'.join(rows)+'\n');print('120 original layout table initializations captured')

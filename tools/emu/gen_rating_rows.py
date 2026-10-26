"""Run original 0x574310 with synthetic player queries; capture row IDs."""
import pathlib,random,struct
from emu import Emu,OBJ_BASE
E=Emu();sent=OBJ_BASE;nodes=[OBJ_BASE+0x1000*(i+1) for i in range(72)]
E.set_u32(0x68b990,sent);state={};draws=[]
def first(e):
 e.set_u32(e.arg(0),nodes[0] if state['players'] else sent);return e.arg(0)
def data(e):
 i=nodes.index(e.arg(0));p=e.arg(1);kind,flags,share=state['players'][i]
 e.write(p,bytes(192));e.set_u32(p+8,kind);e.write(p+0x68,struct.pack('<d',share));return 0
def flags(e): return state['players'][nodes.index(e.arg(0))][1]
def human(e):e.set_u32(e.arg(0),nodes[state['human']]);return e.arg(0)
def draw(e):
 i=nodes.index(e.arg(3));draws.append(i);return int(state['players'][i][0]==1)
E.hook_ret(0x4363c0,first);E.hook_ret(0x43fc40,data);E.hook_ret(0x43f620,flags)
E.hook_ret(0x568f60,human);E.hook_ret(0x574190,draw)
r=random.Random(0x574310);rows=[]
for case in range(320):
 n=r.randrange(1,72);h=r.randrange(n);limit=r.randrange(13)
 players=[(r.choice([2,2,2,5,6,9]),r.choice([0,0,0,0x800]),r.choice([0.,.1,.2,.3,.5,1.])) for _ in range(n)]
 players[h]=(1,0,players[h][2]);state.update(players=players,human=h)
 for i in range(n):E.set_u32(nodes[i],nodes[i+1] if i+1<n else sent)
 draws.clear();E.call(0x574310,[0,0,0,limit],'iiii',max_insn=10000000)
 rows.append(f'{h}\t{limit}\t'+ ';'.join(','.join(map(str,p)) for p in players)+'\t'+','.join(map(str,draws)))
pathlib.Path(__file__).with_name('golden_rating_rows.tsv').write_text('\n'.join(rows)+'\n')
print('320 original rating tables captured')

"""Capture 0x5eb4b0 scene inputs with original transform/matrix arithmetic."""
import pathlib,random,struct
from emu import Emu,OBJ_BASE
e=Emu();r=random.Random(0x5eb4b0)
c=OBJ_BASE;s=c+0x10000;t=s+0x10000;state={};rows=[]
e.write(0x7025f0,struct.pack('<12f',1,0,0,0,1,0,0,0,1,0,0,0))
def clip(q):q.write(q.arg(0),struct.pack('<4i',*state['clip']));return 0
def capture(q):
 state['out']=[struct.unpack('<12I',q.read(q.arg(i),48)) for i in range(2)];return 0
e.hook_thiscall(0x5b6710,clip,1);e.hook_thiscall(0x5ae0e0,capture,2)
for case in range(320):
 e.write(c,bytes(0x100));e.write(s,bytes(0x2200));e.write(t,bytes(0x100));e.set_u32(c+0x50,t);e.set_u32(t+0x38,1)
 values=[r.uniform(-1,1) for _ in range(12)]
 for i in [0,4,8]:values[i]+=3
 raw=struct.pack('<12f',*values);e.write(t+0x40,raw)
 lens=struct.pack('<2f',r.uniform(.2,3),r.uniform(.2,3));e.write(c+0x54,lens)
 mirror=case%2;stereo=(case//2)%2;phase=(case//4)%2
 e.set_u32(c+0x64,mirror);e.set_u32(0x702e8c,stereo*0x100);e.set_u32(s+0x210c,phase)
 shift=r.uniform(-2,2);angle=r.uniform(-.3,.3)
 e.write(s+0x2130,struct.pack('<2d',shift,angle))
 state['clip']=[r.randrange(-100,30),r.randrange(-100,30),r.randrange(100,1000),r.randrange(100,800)]
 e.call(0x5eb4b0,[s],'i',this=c)
 fields=[','.join(map(str,struct.unpack('<12I',raw))),','.join(map(str,struct.unpack('<2I',lens))),str(mirror),str(stereo),str(phase),repr(shift),repr(angle),','.join(map(str,state['clip']))]
 fields.extend(','.join(map(str,v)) for v in state['out']);rows.append('\t'.join(fields))
pathlib.Path(__file__).with_name('golden_camera_scene.tsv').write_text('\n'.join(rows)+'\n');print('320 original camera scene preparations captured')

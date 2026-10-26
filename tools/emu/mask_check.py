"""Compare exported MS16 masks with original software blitter 0x5b8980.
Run mask_info first; input/output assets must remain in an ignored scratch directory.
"""
import pathlib,struct,sys
from emu import Emu,OBJ_BASE
for path in pathlib.Path(sys.argv[1]).glob('*.msk'):
 data=path.read_bytes();w,h=struct.unpack_from('<HH',data,4)
 e=Emu();s=OBJ_BASE;m=s+0x1000;payload=s+0x2000;out=s+0x100000
 e.write(s,struct.pack('<HHHH',w,h,2,w*2));e.set_u32(s+24,out)
 e.write(m+4,struct.pack('<HHH',w,h,2));e.set_u32(m+16,payload);e.set_u32(m+28,1)
 e.write(payload,data[0x308:]);e.write(out,struct.pack('<H',0xdead)*(w*h))
 e.hook_ret(0x5b64c0,lambda emu:0)
 e.call(0x5b8980,[s,0,0,m,0],'iiiii')
 assert e.read(out,w*h*2)==path.with_name(path.name+'.raw').read_bytes(),path.name
 for x,y in [(-10,-7),(20,12),(35,38)]:
  sw,sh=48,48
  e.write(s,struct.pack('<HHHH',sw,sh,2,sw*2));e.write(out,struct.pack('<H',0xdead)*(sw*sh))
  expected=bytearray(struct.pack('<H',0xdead)*(sw*sh));raw=path.with_name(path.name+'.raw').read_bytes()
  for dy in range(h):
   for dx in range(w):
    pixel=raw[(dy*w+dx)*2:(dy*w+dx)*2+2]
    if 0<=x+dx<sw and 0<=y+dy<sh and pixel!=b'\xad\xde':
     pos=((y+dy)*sw+x+dx)*2;expected[pos:pos+2]=pixel
  e.call(0x5b8980,[s,x,y,m,0],'iiiii')
  assert e.read(out,sw*sh*2)==expected,(path.name,x,y)
 print(path.name,'original raster and three clipped positions exact')

import capstone, struct, sys
b=open(r'E:\openHT2\Hard Truck 2 King of the Road\king.exe','rb').read(); base=0x400000
SECT=[(0x401000,0x1000,0x24a000),(0x64b000,0x24b000,0x21000),(0x66c000,0x26c000,0x1f000)]
def raw(va,n):
    for v,o,sz in SECT:
        if v<=va<v+sz: return b[o+va-v:o+va-v+n]
md=capstone.Cs(capstone.CS_ARCH_X86,capstone.CS_MODE_32)
def show(a,maxn=0x400,whole=False):
    s=a-base; n=0
    for i in md.disasm(b[s:s+maxn],a):
        extra=''
        if 'ptr [0x' in i.op_str and i.mnemonic.startswith(('fld','fmul','fadd','fsub','fdiv','fcom','fisub','mov','cmp','push')):
            try:
                va=int(i.op_str.split('[0x')[1].split(']')[0],16)
                r=raw(va,8)
                if r: extra='   ; f32=%.9g f64=%.9g u32=0x%x'%(struct.unpack('<f',r[:4])[0],struct.unpack('<d',r)[0],struct.unpack('<I',r[:4])[0])
            except Exception: pass
        print('%08x  %-7s %s%s'%(i.address,i.mnemonic,i.op_str,extra))
        if not whole and i.mnemonic=='ret' and n>3: break
        n+=1
if __name__=='__main__':
    for a in sys.argv[1:]:
        if ':' in a:
            ad,ln=a.split(':'); print('----',a); show(int(ad,16),int(ln,0),True)
        else:
            print('----',a); show(int(a,16))

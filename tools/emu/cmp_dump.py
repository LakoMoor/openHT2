import sys, struct
a, b = open(sys.argv[1], 'rb').read(), open(sys.argv[2], 'rb').read()
n = 0
for i in range(0, min(len(a), len(b)), 4):
    x, y = a[i:i + 4], b[i:i + 4]
    if x != y:
        fx, fy = struct.unpack('<f', x)[0], struct.unpack('<f', y)[0]
        if abs(fx - fy) <= 2e-5 * max(abs(fx), abs(fy), 1.0) and fx == fx:
            continue
        print(hex(i), 'port', fx, x.hex(), 'orig', fy, y.hex())
        n += 1
        if n > 40:
            break

"""產生 PWA 圖示（標準函式庫，不需要額外套件）。

用法：python3 -I tools/make_icons.py web/public/icons
圖案：深紅底、米色圓、一圈白色針腳與一條交叉的針線。
"""
import math
import os
import struct
import sys
import zlib

RED = (179, 40, 45)
CREAM = (241, 234, 219)
WHITE = (255, 255, 255)


def png(path, size, pixel):
    rows = []
    for y in range(size):
        row = bytearray([0])
        for x in range(size):
            row += bytes(pixel(x / size, y / size))
        rows.append(bytes(row))

    def chunk(t, d):
        c = struct.pack(">I", len(d)) + t + d
        return c + struct.pack(">I", zlib.crc32(t + d) & 0xFFFFFFFF)

    data = (
        b"\x89PNG\r\n\x1a\n"
        + chunk(b"IHDR", struct.pack(">IIBBBBB", size, size, 8, 2, 0, 0, 0))
        + chunk(b"IDAT", zlib.compress(b"".join(rows), 9))
        + chunk(b"IEND", b"")
    )
    with open(path, "wb") as f:
        f.write(data)


def seg_dist(px, py, ax, ay, bx, by):
    dx, dy = bx - ax, by - ay
    t = max(0.0, min(1.0, ((px - ax) * dx + (py - ay) * dy) / (dx * dx + dy * dy)))
    return math.hypot(px - (ax + t * dx), py - (ay + t * dy))


def make(scale):
    """scale：圖案占畫面的比例（maskable 需要留較大的安全邊）。"""

    def pixel(u, v):
        u, v = (u - 0.5) / scale + 0.5, (v - 0.5) / scale + 0.5
        r = math.hypot(u - 0.5, v - 0.5)
        if r > 0.42:
            return RED
        if r > 0.36:
            return CREAM
        # 圓內：一圈虛線針腳
        ang = math.atan2(v - 0.5, u - 0.5)
        if 0.25 < r < 0.31 and int((ang + math.pi) / (2 * math.pi) * 24) % 2 == 0:
            return RED
        # 一條斜向的針線（粗線）
        if seg_dist(u, v, 0.30, 0.68, 0.70, 0.32) < 0.028:
            return RED
        return CREAM

    return pixel


if __name__ == "__main__":
    out = sys.argv[1]
    os.makedirs(out, exist_ok=True)
    png(os.path.join(out, "icon-192.png"), 192, make(1.0))
    png(os.path.join(out, "icon-512.png"), 512, make(1.0))
    png(os.path.join(out, "icon-maskable-512.png"), 512, make(0.78))
    print("done")

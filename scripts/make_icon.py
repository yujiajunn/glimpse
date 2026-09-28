"""
Generate a multi-resolution Glimpse icon for Tauri bundling.

Output: src-tauri/icons/icon.ico (32x32 + 128x128 + 256x256)
"""
import struct
from pathlib import Path

# Sizes Tauri 2 expects: 32x32, 128x128, 256x256
SIZES = [32, 128, 256]

# Glimpse 配色：背景深灰，前景亮蓝
BG = (28, 28, 30, 255)       # 深灰
FG = (10, 132, 255, 255)     # iOS blue


def make_pixels(size):
    """画一个简单的圆环 + G 字开口"""
    pixels = [[BG] * size for _ in range(size)]
    cx, cy = size / 2, size / 2

    # 圆环外径 = size * 0.45
    outer = size * 0.45
    inner = size * 0.32

    for y in range(size):
        for x in range(size):
            dx = x - cx + 0.5
            dy = y - cy + 0.5
            r = (dx * dx + dy * dy) ** 0.5

            if inner <= r <= outer:
                # G 字开口部分（右侧）画横杠
                if dx > 0 and abs(dy) < size * 0.06 and r < outer * 0.85:
                    pixels[y][x] = FG
                else:
                    pixels[y][x] = FG
            elif r < inner and dx > size * 0.15 and dy < -size * 0.05:
                # G 的开口缺口
                pixels[y][x] = (0, 0, 0, 0)

    return pixels


def make_bitmap(pixels, size):
    """打包成 BMP (ICO 用 BITMAPINFOHEADER + 像素数据)"""
    header = struct.pack(
        "<IiiHHIIiiII",
        40,                # biSize
        size,              # biWidth
        size * 2,          # biHeight (XOR + AND mask 合并，所以 *2)
        1,                 # biPlanes
        32,                # biBitCount
        0,                 # biCompression (BI_RGB)
        size * size * 4,   # biSizeImage
        0, 0, 0, 0,        # biXPels/m, biYPels/m, biClrUsed, biClrImportant
    )

    # 像素数据（BGRA, bottom-up）
    pixel_bytes = bytearray()
    for y in range(size - 1, -1, -1):
        for x in range(size):
            r, g, b, a = pixels[y][x]
            pixel_bytes += bytes([b, g, r, a])

    # AND mask（1-bit alpha mask）
    and_mask = bytearray()
    row_size = ((size + 31) // 32) * 4
    for _ in range(size):
        and_mask += b"\x00" * row_size

    return header + bytes(pixel_bytes) + bytes(and_mask)


def make_ico(bitmaps):
    """组装 ICO 文件（多 frame）"""
    num_images = len(bitmaps)
    icondir = struct.pack("<HHH", 0, 1, num_images)

    offset = 6 + 16 * num_images
    entries = b""
    data = b""
    for size, bmp in bitmaps:
        sz = len(bmp)
        entries += struct.pack(
            "<BBBBHHII",
            size if size < 256 else 0,  # bWidth (0 = 256)
            size if size < 256 else 0,  # bHeight
            0,                          # bColorCount
            1,                          # bReserved
            1,                          # wPlanes
            32,                         # wBitCount
            sz,                         # dwBytesInRes
            offset,                     # dwImageOffset
        )
        data += bmp
        offset += sz

    return icondir + entries + data


def main():
    bitmaps = []
    for size in SIZES:
        pixels = make_pixels(size)
        bmp = make_bitmap(pixels, size)
        bitmaps.append((size, bmp))

    ico = make_ico(bitmaps)

    out = Path(__file__).parent.parent / "src-tauri" / "icons" / "icon.ico"
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_bytes(ico)
    print(f"wrote {out} ({len(ico)} bytes, sizes={SIZES})")


if __name__ == "__main__":
    main()
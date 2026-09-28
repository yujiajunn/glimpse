"""
Generate a minimal Glimpse icon (32x32 ICO with one BMP frame).

Output: src-tauri/icons/icon.ico
"""
import struct
from pathlib import Path

# 32x32 RGBA pixel data: simple "G" letter on transparent background
W, H = 32, 32

# Glimpse 配色：背景深灰，前景亮蓝
BG = (28, 28, 30, 255)       # 深灰
FG = (10, 132, 255, 255)     # iOS blue
ALPHA = (0, 0, 0, 0)


def make_pixels():
    """画一个简单的 'G' 字母 + 圆环"""
    pixels = [[BG] * W for _ in range(H)]
    cx, cy = W // 2, H // 2

    for y in range(H):
        for x in range(W):
            dx = x - cx + 0.5
            dy = y - cy + 0.5
            r2 = dx * dx + dy * dy

            # 外圆
            if 9.5 ** 2 <= r2 <= 11.5 ** 2:
                pixels[y][x] = FG
            # G 字的内缺口（横杠）
            elif 10 <= y <= 12 and 16 <= x <= 22:
                pixels[y][x] = FG if abs(cis_dy := dy) < 1.5 else BG
            # G 字的开口
            elif r2 <= 8 ** 2 and (x >= cx + 4 or y <= cy - 2):
                pixels[y][x] = ALPHA

    return pixels


def make_bitmap(pixels):
    """打包成 BMP (ICO 用 BITMAPINFOHEADER + 像素数据)"""
    # BITMAPINFOHEADER (40 bytes)
    # 注：ICO 的 BMP 数据是 BGRA，图像是上下颠倒的
    header = struct.pack(
        "<IiiHHIIiiII",
        40,                # biSize
        W,                 # biWidth
        H * 2,             # biHeight (XOR + AND mask 合并，所以 *2)
        1,                 # biPlanes
        32,                # biBitCount
        0,                 # biCompression (BI_RGB)
        W * H * 4,         # biSizeImage
        0, 0, 0, 0,        # biXPels/m, biYPels/m, biClrUsed, biClrImportant
    )

    # 像素数据（BGRA, bottom-up）
    pixel_bytes = bytearray()
    for y in range(H - 1, -1, -1):
        for x in range(W):
            r, g, b, a = pixels[y][x]
            pixel_bytes += bytes([b, g, r, a])

    # AND mask（1-bit alpha mask）
    and_mask = bytearray()
    row_size = ((W + 31) // 32) * 4
    for _ in range(H):
        and_mask += b"\x00" * row_size

    return header + bytes(pixel_bytes) + bytes(and_mask)


def make_ico(bitmaps):
    """组装 ICO 文件（一个 frame）"""
    # ICONDIR
    num_images = len(bitmaps)
    icondir = struct.pack("<HHH", 0, 1, num_images)

    # 写入实际位图
    offset = 6 + 16 * num_images
    entries = b""
    data = b""
    for bmp in bitmaps:
        size = len(bmp)
        # ICONDIRENTRY
        entries += struct.pack(
            "<BBBBHHII",
            32 if W >= 256 else W,  # bWidth
            32 if H >= 256 else H,  # bHeight
            0,                       # bColorCount
            1,                       # bReserved
            1,                       # wPlanes
            32,                      # wBitCount
            size,                    # dwBytesInRes
            offset,                  # dwImageOffset
        )
        data += bmp
        offset += size

    return icondir + entries + data


def main():
    pixels = make_pixels()
    bmp = make_bitmap(pixels)
    ico = make_ico([bmp])

    out = Path(__file__).parent.parent / "src-tauri" / "icons" / "icon.ico"
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_bytes(ico)
    print(f"wrote {out} ({len(ico)} bytes)")


if __name__ == "__main__":
    main()
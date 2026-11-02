#!/usr/bin/env python3
"""Writes resources/packaging/windows/openpacketmon.ico from the PNG icons of the Linux package.

The icon of openpacketmon.exe (build.rs embeds it on Windows). Since Windows Vista an ICO file may hold its
images as PNG, so the PNG files are copied in unchanged and no image library is needed. Run it again after the
icons in resources/packaging/linux/graphics/ have changed and commit the result.

Usage: dev/make-ico.py
"""

import pathlib
import struct
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
SOURCE = ROOT / "resources" / "packaging" / "linux" / "graphics"
TARGET = ROOT / "resources" / "packaging" / "windows" / "openpacketmon.ico"
# the sizes Windows asks for most (Explorer, taskbar, title bar, Alt+Tab, at 100 to 200 % scaling)
SIZES = [16, 24, 32, 48, 64, 256]

PNG_SIGNATURE = b"\x89PNG\r\n\x1a\n"


def png_size(data: bytes) -> tuple[int, int]:
    """Width and height from the IHDR chunk, which always comes first."""
    if data[:8] != PNG_SIGNATURE or data[12:16] != b"IHDR":
        raise ValueError("not a PNG file")
    return struct.unpack(">II", data[16:24])


def main() -> int:
    images = []
    for size in SIZES:
        path = SOURCE / f"openpacketmon_{size}x{size}.png"
        data = path.read_bytes()
        if png_size(data) != (size, size):
            print(f"{path}: expected {size}x{size}, found {png_size(data)}", file=sys.stderr)
            return 1
        images.append((size, data))

    # ICONDIR, then one ICONDIRENTRY per image, then the images
    out = bytearray(struct.pack("<HHH", 0, 1, len(images)))
    offset = 6 + 16 * len(images)
    for size, data in images:
        side = 0 if size >= 256 else size  # 0 stands for 256
        out += struct.pack("<BBBBHHII", side, side, 0, 0, 1, 32, len(data), offset)
        offset += len(data)
    for _, data in images:
        out += data

    TARGET.parent.mkdir(parents=True, exist_ok=True)
    TARGET.write_bytes(out)
    print(f"{TARGET.relative_to(ROOT)}: {len(images)} images, {len(out)} bytes")
    return 0


if __name__ == "__main__":
    sys.exit(main())

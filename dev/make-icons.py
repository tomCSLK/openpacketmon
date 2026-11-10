#!/usr/bin/env python3
"""Draws the program icons and the picture of the README from the mark of openpacketmon (logo C, "magnifier over
packets", resources/icons/logo-mark.svg): white on a blue square with rounded corners, as in the draft
(docs/entwurf/, `.appicon`: corners 22 %, the mark 75 % of the side).

Writes resources/packaging/linux/graphics/openpacketmon_<n>x<n>.png and docs/assets/openpacketmon-logo{,-dark}.png.
Run dev/make-ico.py afterwards for the icon of Windows, and commit the results. Needs Pillow.

Usage: dev/make-icons.py
"""

import pathlib
import sys

from PIL import Image, ImageDraw, ImageFont

ROOT = pathlib.Path(__file__).resolve().parent.parent
GRAPHICS = ROOT / "resources" / "packaging" / "linux" / "graphics"
ASSETS = ROOT / "docs" / "assets"
FONTS = ROOT / "resources" / "fonts"
SIZES = [8, 16, 22, 24, 32, 36, 42, 48, 64, 72, 96, 128, 160, 192, 256, 384, 512, 1024]

# the colors of the draft (docs/entwurf/assets/tokens.css)
ACCENT = (0x1C, 0x5E, 0xA8)
WHITE = (0xFF, 0xFF, 0xFF)
TEXT = {"light": (0x15, 0x18, 0x1C), "dark": (0xE5, 0xE8, 0xEC)}
TEXT_3 = {"light": (0x63, 0x6C, 0x78), "dark": (0x8D, 0x96, 0xA1)}

# drawn this many times larger, then scaled down, for smooth edges
SCALE = 8


def mark(draw, left, top, side, color):
    """The mark in a box of `side` px at (left, top): its 24 units of the SVG, stroke 2, round ends."""
    unit = side / 24

    def at(x, y):
        return (left + x * unit, top + y * unit)

    stroke = 2 * unit
    # the lens: a circle of radius 7.5 around (10.5, 10.5)
    r_out = (7.5 + 1) * unit
    r_in = (7.5 - 1) * unit
    cx, cy = at(10.5, 10.5)
    draw.ellipse((cx - r_out, cy - r_out, cx + r_out, cy + r_out), fill=color)
    draw.ellipse((cx - r_in, cy - r_in, cx + r_in, cy + r_in), fill=(0, 0, 0, 0))
    # the handle from (21, 21) to (15.8, 15.8) with round ends
    a, b = at(15.8, 15.8), at(21, 21)
    draw.line((a, b), fill=color, width=round(stroke))
    for x, y in (a, b):
        draw.ellipse((x - stroke / 2, y - stroke / 2, x + stroke / 2, y + stroke / 2), fill=color)
    # three packets: squares of 2.8 with corners of 0.5
    for x in (5.8, 9.1, 12.4):
        x0, y0 = at(x, 9)
        x1, y1 = at(x + 2.8, 9 + 2.8)
        draw.rounded_rectangle((x0, y0, x1, y1), radius=0.5 * unit, fill=color)


def icon(size):
    """The program icon of `size` px: the white mark on the blue square."""
    big = size * SCALE
    image = Image.new("RGBA", (big, big), (0, 0, 0, 0))
    draw = ImageDraw.Draw(image)
    draw.rounded_rectangle((0, 0, big - 1, big - 1), radius=0.22 * big, fill=ACCENT)
    # the lens is cut out of a layer of its own, so it stays blue inside
    layer = Image.new("RGBA", (big, big), (0, 0, 0, 0))
    mark(ImageDraw.Draw(layer), 0.125 * big, 0.125 * big, 0.75 * big, WHITE)
    image.alpha_composite(layer)
    return image.resize((size, size), Image.Resampling.LANCZOS)


def banner(theme):
    """The picture of the README: the icon, the name and what it is, on a transparent ground."""
    name = ImageFont.truetype(str(FONTS / "FiraSans-SemiBold.ttf"), 92)
    what = ImageFont.truetype(str(FONTS / "FiraSans-Regular.ttf"), 40)
    side, margin, gap = 160, 40, 44
    x = margin + side + gap
    width = round(x + name.getlength("openpacketmon") + margin)
    height = 240
    image = Image.new("RGBA", (width, height), (0, 0, 0, 0))
    image.alpha_composite(icon(side), (margin, (height - side) // 2))
    draw = ImageDraw.Draw(image)
    draw.text((x, 40), "openpacketmon", font=name, fill=TEXT[theme])
    draw.text((x + 4, 150), "packet monitoring", font=what, fill=TEXT_3[theme])
    return image


def main() -> int:
    GRAPHICS.mkdir(parents=True, exist_ok=True)
    for size in SIZES:
        path = GRAPHICS / f"openpacketmon_{size}x{size}.png"
        icon(size).save(path, optimize=True)
    ASSETS.mkdir(parents=True, exist_ok=True)
    for theme, suffix in (("light", ""), ("dark", "-dark")):
        banner(theme).save(ASSETS / f"openpacketmon-logo{suffix}.png", optimize=True)
    print(f"{len(SIZES)} icons in {GRAPHICS.relative_to(ROOT)}, 2 pictures in {ASSETS.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())

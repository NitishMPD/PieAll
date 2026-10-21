"""Regenerate PieAll's vector and Windows icons. Requires Pillow (artwork only)."""

import io
import math
import struct
from pathlib import Path

from PIL import Image, ImageDraw

ROOT = Path(__file__).resolve().parent.parent
BRAND = ROOT / "public" / "brand"
ICONS = ROOT / "src-tauri" / "icons"
BACKGROUND = "#1c1c1e"
CREAM = "#f4f4f6"
COPPER = "#ff8a3d"
SIZE = 80
RADIUS = 25
STROKE = 10


def point(angle, radius=RADIUS):
    radians = math.radians(angle)
    return 40 + radius * math.cos(radians), 40 + radius * math.sin(radians)


def artwork(size):
    scale = size * 4 / SIZE
    image = Image.new("RGBA", (size * 4, size * 4))
    draw = ImageDraw.Draw(image)
    draw.rounded_rectangle((0, 0, size * 4 - 1, size * 4 - 1), 19 * scale, fill=BACKGROUND)
    for index in range(6):
        start, end = -105 + index * 60, -75 + index * 60
        color = COPPER if index == 1 else CREAM
        # A filled outline avoids raster line-join artifacts on short arc segments.
        angles = [start + (end - start) * step / 88 for step in range(89)]
        outline = [point(angle, RADIUS + STROKE / 2) for angle in angles]
        outline += [point(angle, RADIUS - STROKE / 2) for angle in reversed(angles)]
        draw.polygon([tuple(v * scale for v in p) for p in outline], fill=color)
        for angle in (start, end):
            x, y = (v * scale for v in point(angle))
            r = STROKE * scale / 2
            draw.ellipse((x - r, y - r, x + r, y + r), fill=color)
    return image.resize((size, size), Image.Resampling.LANCZOS)


def main():
    BRAND.mkdir(parents=True, exist_ok=True)
    parts = [f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 80 80" fill="none">',
             f'<rect width="80" height="80" rx="19" fill="var(--pieall-tile, {BACKGROUND})"/>']
    for index in range(6):
        x1, y1 = point(-105 + index * 60)
        x2, y2 = point(-75 + index * 60)
        color = f"var(--pieall-accent, {COPPER})" if index == 1 else f"var(--pieall-segment, {CREAM})"
        parts.append(f'<path d="M{x1:.4f} {y1:.4f} A25 25 0 0 1 {x2:.4f} {y2:.4f}" '
                     f'stroke="{color}" stroke-width="{STROKE}" stroke-linecap="round"/>')
    parts.append('</svg>')
    (BRAND / "pieall.svg").write_text("\n".join(parts) + "\n", encoding="utf-8")
    artwork(512).save(ICONS / "icon.png")
    sizes = (16, 20, 24, 32, 40, 48, 64, 128, 256)
    # Store independently rendered PNG frames in the Windows ICO container.
    frames = []
    for size in sizes:
        buffer = io.BytesIO()
        artwork(size).save(buffer, format="PNG")
        frames.append(buffer.getvalue())
    offset = 6 + 16 * len(frames)
    entries = []
    for size, frame in zip(sizes, frames):
        entries.append(struct.pack("<BBBBHHII", size % 256, size % 256, 0, 0,
                                   1, 32, len(frame), offset))
        offset += len(frame)
    (ICONS / "icon.ico").write_bytes(struct.pack("<HHH", 0, 1, len(frames))
                                     + b"".join(entries) + b"".join(frames))
    tray = artwork(32).tobytes()
    assert len(tray) == 32 * 32 * 4
    (ICONS / "tray.rgba").write_bytes(tray)
    with Image.open(ICONS / "icon.ico") as icon:
        assert icon.ico.sizes() == {(size, size) for size in sizes}
    assert artwork(32).getpixel((0, 0))[3] == 0


if __name__ == "__main__":
    main()

"""Rebuild app icons from the flat geometry in app-icon.svg (Pillow)."""
from pathlib import Path
from xml.etree import ElementTree

from PIL import Image, ImageDraw

ROOT = Path(__file__).resolve().parents[1]
SCALE = 4
image = Image.new("RGBA", (512 * SCALE, 512 * SCALE))
draw = ImageDraw.Draw(image)
for shape in ElementTree.parse(ROOT / "app-icon.svg").getroot():
    kind = shape.tag.rsplit("}", 1)[-1]
    if kind == "rect":
        x, y, w, h, radius = (int(shape.attrib[k]) for k in ("x", "y", "width", "height", "rx"))
        draw.rounded_rectangle((x*SCALE, y*SCALE, (x+w)*SCALE-1, (y+h)*SCALE-1), radius=radius*SCALE, fill=shape.attrib["fill"])
    elif kind == "polygon":
        points = [tuple(int(v)*SCALE for v in point.split(",")) for point in shape.attrib["points"].split()]
        draw.polygon(points, fill=shape.attrib["fill"])
assets = ROOT / "gpui" / "assets"
image = image.resize((1024, 1024), Image.Resampling.LANCZOS)
image.resize((512, 512), Image.Resampling.LANCZOS).save(assets / "icon.png")
image.save(assets / "icon.ico", sizes=[(s, s) for s in (16, 24, 32, 48, 64, 128, 256)])
image.save(assets / "icon.icns")
print("Updated PNG, ICO and ICNS in", assets)

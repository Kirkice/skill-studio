"""Rebuild raster app icons (Pillow). Geometry mirrors ../app-icon.svg."""
from pathlib import Path
from PIL import Image, ImageDraw

ROOT = Path(__file__).resolve().parents[1]
SCALE = 4
image = Image.new("RGBA", (512 * SCALE, 512 * SCALE))
d = ImageDraw.Draw(image)
def box(xy): return tuple(round(v * SCALE) for v in xy)
def line(points, color, width):
    d.line([(x*SCALE,y*SCALE) for x,y in points], fill=color, width=width*SCALE, joint="curve")
    r=width/2
    for x,y in [points[0],points[-1]]: d.ellipse(box((x-r,y-r,x+r,y+r)),fill=color)
def circle(x,y,r,color): d.ellipse(box((x-r,y-r,x+r,y+r)),fill=color)
d.rounded_rectangle(box((8,8,504,504)),radius=112*SCALE,fill="#101824",outline="#25374c",width=4*SCALE)
d.rounded_rectangle(box((112,80,396,432)),radius=52*SCALE,fill="#1b2a40",outline="#415d7d",width=12*SCALE)
# Cut the upper-right paper corner out of the rounded rectangle.
d.polygon([(x*SCALE,y*SCALE) for x,y in [(300,74),(408,74),(408,176),(396,176),(300,80)]],fill="#101824")
line([(300,80),(396,176)],"#415d7d",12)
d.polygon([(x*SCALE,y*SCALE) for x,y in [(300,80),(300,148),(328,176),(396,176)]],fill="#315578")
line([(228,166),(228,340)],"#8cbbfa",20)
line([(228,232),(300,232),(316,248),(316,292)],"#47d8c1",20)
line([(228,312),(164,312)],"#47d8c1",20)
for x,y,r,c in [(228,166,30,"#9cc7ff"),(228,166,12,"#1b2a40"),(228,248,22,"#d8e9ff"),(228,340,26,"#9cc7ff"),(228,340,10,"#1b2a40"),(316,292,24,"#47d8c1"),(164,312,16,"#47d8c1")]: circle(x,y,r,c)
assets=ROOT/"gpui"/"assets"
image=image.resize((1024,1024),Image.Resampling.LANCZOS)
image.resize((512,512),Image.Resampling.LANCZOS).save(assets/"icon.png")
image.save(assets/"icon.ico",sizes=[(s,s) for s in (16,24,32,48,64,128,256)])
image.save(assets/"icon.icns")
print("Updated PNG, multi-resolution ICO and ICNS in",assets)

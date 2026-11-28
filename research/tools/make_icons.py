"""Propositions d'icones du jeu (pixel art 32x32, agrandi sans flou).

Sorties : assets/icons/proposals/icon_<lettre>_<taille>.png et une planche de comparaison.
Usage : python research/tools/make_icons.py
"""
import math
import re
from pathlib import Path

from PIL import Image, ImageDraw

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "assets/icons/proposals"
N = 32

ATLAS = Image.open(ROOT / "assets/gen/atlas.png").convert("RGBA")
META = (ROOT / "assets/gen/atlas.ron").read_text(encoding="utf-8")


def sprite(name, anim="0"):
    m = re.search(r'"%s": \{[^}]*?"%s": \(frames: \[\(x: (\d+), y: (\d+), w: (\d+), h: (\d+)' % (name, anim), META, re.S)
    x, y, w, h = map(int, m.groups())
    return ATLAS.crop((x, y, x + w, y + h))


def hexc(s, a=255):
    s = s.lstrip("#")
    return (int(s[0:2], 16), int(s[2:4], 16), int(s[4:6], 16), a)


def lerp(c1, c2, t):
    return tuple(int(round(a + (b - a) * t)) for a, b in zip(c1, c2))


def gradient(inner, outer, n=N):
    """Degrade radial sur n x n, de meme echelle que l'icone de 32 (centres alignes)."""
    im = Image.new("RGBA", (n, n), (0, 0, 0, 0))
    ci, co = hexc(inner), hexc(outer)
    for y in range(n):
        for x in range(n):
            d = math.hypot(x + 0.5 - n / 2, y + 0.5 - n / 2) / (N * 0.62)
            im.putpixel((x, y), lerp(ci, co, min(1.0, d)))
    return im


def base(inner, outer, rim="#0c0c14", rounded=True):
    """Carre aux coins arrondis, degrade radial, liseré sombre ; `rounded=False` : carre plein."""
    im = gradient(inner, outer)
    if not rounded:
        return im
    # coins arrondis (rayon 4) + liseré
    mask = Image.new("L", (N, N), 0)
    ImageDraw.Draw(mask).rounded_rectangle((0, 0, N - 1, N - 1), radius=5, fill=255)
    out = Image.new("RGBA", (N, N), (0, 0, 0, 0))
    out.paste(im, (0, 0), mask)
    px = out.load()
    rc = hexc(rim)
    for y in range(N):
        for x in range(N):
            if px[x, y][3] and any(
                not (0 <= x + dx < N and 0 <= y + dy < N) or mask.getpixel((x + dx, y + dy)) == 0
                for dx, dy in ((1, 0), (-1, 0), (0, 1), (0, -1))
            ):
                px[x, y] = rc
    return out


def outline(shape, color):
    """Ajoute un contour d'un pixel autour des pixels opaques."""
    out = shape.copy()
    p, q = shape.load(), out.load()
    for y in range(shape.height):
        for x in range(shape.width):
            if p[x, y][3] == 0 and any(
                0 <= x + dx < shape.width and 0 <= y + dy < shape.height and p[x + dx, y + dy][3] > 0
                for dx, dy in ((1, 0), (-1, 0), (0, 1), (0, -1))
            ):
                q[x, y] = hexc(color)
    return out


def triforce(side):
    """Triforce rasterisee (trois triangles equilateraux), doree, ombree a droite."""
    h = side * math.sqrt(3) / 2
    w_img, h_img = int(math.ceil(side)) + 2, int(math.ceil(h)) + 2
    im = Image.new("RGBA", (w_img, h_img), (0, 0, 0, 0))
    s2 = side / 2

    def inside(px, py, ax, ay, s):
        # triangle pointe en haut de sommet (ax, ay), cote s
        hh = s * math.sqrt(3) / 2
        if py < ay or py > ay + hh:
            return False
        half = (py - ay) / math.sqrt(3)
        return ax - half <= px <= ax + half

    ox = w_img / 2
    tris = [(ox, 1, s2), (ox - s2 / 2, 1 + h / 2, s2), (ox + s2 / 2, 1 + h / 2, s2)]
    for y in range(h_img):
        for x in range(w_img):
            cx, cy = x + 0.5, y + 0.5
            for ax, ay, s in tris:
                if inside(cx, cy, ax, ay, s):
                    t = (cx - (ax - (cy - ay) / math.sqrt(3))) / max(1e-6, 2 * (cy - ay) / math.sqrt(3) + 1e-6)
                    col = "#fff2a0" if t < 0.22 else ("#f8c800" if t < 0.7 else "#d89800")
                    im.putpixel((x, y), hexc(col))
                    break
    return outline(im, "#5a3800")


SWORD = [
    # epee de legende pointe en bas (lignes de haut en bas) ; . = vide
    "...BBB...",
    "..BbbbB..",
    "...BPB...",
    "...BPB...",
    "...BPB...",
    "BBBBPBBBB",
    "BbbbYbbbB",
    ".BBBBBBB.",
    "...BWB...",
    "...BWS...",
    "...BWS...",
    "...BWS...",
    "...BWS...",
    "...BWS...",
    "...BWS...",
    "...BWS...",
    "...BWS...",
    "...BWS...",
    "...BWS...",
    "...BWS...",
    "...BWS...",
    "...BWS...",
    "....BS...",
    "....B....",
]
SWORD_COL = {"B": "#1a1848", "b": "#5c5cd8", "P": "#8a2ab0", "Y": "#f8d040", "W": "#f0f0ff", "S": "#9aa8d8"}


def sword():
    im = Image.new("RGBA", (len(SWORD[0]), len(SWORD)), (0, 0, 0, 0))
    for y, row in enumerate(SWORD):
        for x, c in enumerate(row):
            if c != ".":
                im.putpixel((x, y), hexc(SWORD_COL[c]))
    return im


def icon_a():
    im = base("#3a4cc0", "#141a50")
    tf = triforce(27)
    im.alpha_composite(tf, ((N - tf.width) // 2, 5))
    sw = sword()
    im.alpha_composite(sw, ((N - sw.width) // 2 + 0, 4))
    return im


def icon_b(rounded=True):
    im = base("#5aa850", "#0e2414", rounded=rounded)
    px = im.load()
    # projecteur clair sous Link
    for y in range(N):
        for x in range(N):
            d = math.hypot((x + 0.5 - 16) / 1.6, y + 0.5 - 27)
            if d < 4.5 and px[x, y][3]:
                px[x, y] = lerp(px[x, y], hexc("#a8e070"), 0.55)
    # yeux rouges dans l'ombre (la horde)
    for ex, ey in ((4, 6), (25, 5), (3, 17), (27, 15), (6, 26), (24, 25), (15, 3)):
        px[ex, ey] = hexc("#ff3030")
        px[ex + 2, ey] = hexc("#ff3030")
    link = sprite("link", "idle_down")
    sh = Image.new("RGBA", (12, 3), (0, 0, 0, 0))
    ImageDraw.Draw(sh).ellipse((0, 0, 11, 2), fill=(0, 0, 0, 110))
    im.alpha_composite(sh, (10, 27))
    im.alpha_composite(link, (8, 5))
    return im


def icon_c():
    im = base("#7a1830", "#1c0610")
    d = ImageDraw.Draw(im)
    gold = hexc("#f0c040")
    for k in range(12):
        a = k * math.pi / 6
        r = 13.5 if k % 3 else 13.0
        x, y = 16 + r * math.sin(a), 16 - r * math.cos(a)
        im.putpixel((int(x), int(y)), gold if k % 3 else hexc("#fff0a0"))
        if k % 3 == 0:
            x2, y2 = 16 + 12 * math.sin(a), 16 - 12 * math.cos(a)
            im.putpixel((int(x2), int(y2)), gold)
    heart = sprite("heart_container").resize((28, 26), Image.NEAREST)
    im.alpha_composite(heart, (2, 4))
    return im


def icon_d():
    im = base("#78b8f0", "#1c3c78")
    link = sprite("link", "idle_down")
    head = link.crop((0, 0, link.width, 15)).resize((32, 30), Image.NEAREST)
    im.alpha_composite(head, (0, 3))
    return im


def finish(im, rim="#0c0c14"):
    """Recoupe les coins arrondis et redessine le liseré par-dessus le contenu."""
    mask = Image.new("L", (N, N), 0)
    ImageDraw.Draw(mask).rounded_rectangle((0, 0, N - 1, N - 1), radius=5, fill=255)
    out = Image.new("RGBA", (N, N), (0, 0, 0, 0))
    out.paste(im, (0, 0), mask)
    px = out.load()
    for y in range(N):
        for x in range(N):
            if px[x, y][3] and any(
                not (0 <= x + dx < N and 0 <= y + dy < N) or mask.getpixel((x + dx, y + dy)) == 0
                for dx, dy in ((1, 0), (-1, 0), (0, 1), (0, -1))
            ):
                px[x, y] = hexc(rim)
    return out


def main():
    OUT.mkdir(parents=True, exist_ok=True)
    icons = {k: finish(f()) for k, f in (("a", icon_a), ("b", icon_b), ("c", icon_c), ("d", icon_d))}
    labels = {
        "a": "A  Triforce & epee",
        "b": "B  Link et la horde",
        "c": "C  Coeur & horloge",
        "d": "D  Portrait de Link",
    }
    for k, im in icons.items():
        for s in (16, 32, 48, 256, 512):
            r = im.resize((s, s), Image.NEAREST if s >= 32 else Image.LANCZOS)
            r.save(OUT / f"icon_{k}_{s}.png")
    # planche : 256 / 64 / 32 / 16 pour chaque proposition
    W, H = 4 * 300, 420
    sheet = Image.new("RGBA", (W, H), hexc("#20222c"))
    d = ImageDraw.Draw(sheet)
    for i, (k, im) in enumerate(icons.items()):
        x0 = i * 300 + 22
        sheet.alpha_composite(im.resize((256, 256), Image.NEAREST), (x0, 40))
        sheet.alpha_composite(im.resize((64, 64), Image.NEAREST), (x0, 320))
        sheet.alpha_composite(im, (x0 + 90, 336))
        sheet.alpha_composite(im.resize((16, 16), Image.LANCZOS), (x0 + 150, 344))
        d.text((x0, 12), labels[k], fill=(240, 240, 240, 255))
        d.text((x0, 300), "64 / 32 / 16 px", fill=(160, 160, 170, 255))
    sheet.save(OUT / "comparaison.png")
    print("ok :", OUT)


if __name__ == "__main__":
    main()

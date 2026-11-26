"""0.6.4 : nouvelles tenues de la garde-robe, par changement de couleurs des planches des personnages
(teinte remplacee sur une plage de teintes), plus Link lapin (planche de Link, Monde des Tenebres).
Ecrit assets/source/Derived - <tenue>.png et data/sprites/v064_wardrobe.ron (les sprites copient les
cadres du personnage de base, seule la planche change). Lecture seule des planches d'origine.
Les dessins restent ceux des auteurs des planches ; seules les couleurs sont modifiees (CREDITS.md).
"""
import colorsys
import re
from pathlib import Path

from PIL import Image

ROOT = Path(__file__).resolve().parents[2]
SRC = ROOT / "assets" / "source"
SPRITES = ROOT / "data" / "sprites"
OUT_RON = SPRITES / "v064_wardrobe.ron"

# (tenue, sprite de base, [(teinte min, teinte max, nouvelle teinte, x saturation, x valeur)], decalage y des cadres)
# teintes en degres ; plage qui passe par 0 si min > max
RECOLORS = [
    ("link_adult_goron", "link_adult", [(65, 170, 2, 1.0, 0.95)], 0),
    ("link_adult_zora", "link_adult", [(65, 170, 215, 1.0, 1.0)], 0),
    ("toon_link_red", "toon_link", [(70, 165, 0, 1.0, 0.95)], 0),
    ("toon_link_blue", "toon_link", [(70, 165, 215, 1.0, 1.0)], 0),
]

# Link lapin : cadres de la planche de Link (rangee y 193), bas x3, profil x2, haut x3
LINK_SHEET = "SNES - The Legend of Zelda_ A Link to the Past - Playable Characters - Link.png"
BUNNY = '''        // Link lapin (Monde des Tenebres sans la Perle de Lune) : planche de Link, rangee des lapins
        (
            source: "%s",
            transparent: ["#008080", "#004040"],
            sprites: {
                "link_bunny": {
                    "idle_down": (frames: [(361, 194, 16, 23)], pivot: (0, -1), align: Bottom),
                    "idle_side": (frames: [(412, 193, 17, 24)], pivot: (0, -1), align: Bottom),
                    "idle_up":   (frames: [(448, 194, 16, 23)], pivot: (0, -1), align: Bottom),
                    "walk_down": (frames: [(361, 194, 16, 23), (378, 193, 16, 24), (395, 194, 16, 23), (378, 193, 16, 24)], fps: 8, pivot: (0, -1), align: Bottom),
                    "walk_side": (frames: [(412, 193, 17, 24), (430, 194, 17, 23)], fps: 8, pivot: (0, -1), align: Bottom),
                    "walk_up":   (frames: [(448, 194, 16, 23), (465, 193, 16, 24), (482, 194, 16, 23), (465, 193, 16, 24)], fps: 8, pivot: (0, -1), align: Bottom),
                },
            },
        ),
''' % LINK_SHEET


def find_sprite(name):
    """Bloc de cadres du sprite `name`, avec la planche et ses couleurs transparentes."""
    for f in sorted(SPRITES.glob("*.ron")):
        t = f.read_text(encoding="utf-8")
        key = f'"{name}": {{'
        i = t.find(key)
        if i < 0:
            continue
        depth, j = 0, i + len(key) - 1
        while True:
            depth += {"{": 1, "}": -1}.get(t[j], 0)
            if depth == 0:
                break
            j += 1
        head = t[:i]
        source = re.findall(r'source:\s*"([^"]+)"', head)[-1]
        k = head.rfind("source:")
        trans = re.match(r'[^\n]*\n\s*transparent:\s*(\[[^\]]*\])', head[k:])
        return source, (trans.group(1) if trans else None), t[i + len(key):j]
    raise SystemExit(f"sprite introuvable : {name}")


def in_range(h, lo, hi):
    return lo <= h <= hi if lo <= hi else (h >= lo or h <= hi)


def recolor(im, rules, keep):
    px = im.load()
    for y in range(im.height):
        for x in range(im.width):
            r, g, b, a = px[x, y]
            if a == 0 or (r, g, b) in keep:
                continue
            h, s, v = colorsys.rgb_to_hsv(r / 255, g / 255, b / 255)
            if s < 0.25 or v < 0.12:
                continue
            for lo, hi, to, ks, kv in rules:
                if in_range(h * 360, lo, hi):
                    nr, ng, nb = colorsys.hsv_to_rgb(to / 360, min(1.0, s * ks), min(1.0, v * kv))
                    px[x, y] = (round(nr * 255), round(ng * 255), round(nb * 255), a)
                    break
    return im


def main():
    blocks = [BUNNY]
    for out, base, rules, dy in RECOLORS:
        source, trans, body = find_sprite(base)
        keep = {tuple(int(c[k:k + 2], 16) for k in (1, 3, 5)) for c in re.findall(r'"(#[0-9a-fA-F]{6})"', trans or "")}
        if rules:
            im = recolor(Image.open(SRC / source).convert("RGBA"), rules, keep)
            sheet = f"Derived - {out}.png"
            im.save(SRC / sheet)
            print(f"ecrit {sheet}")
        else:
            sheet = source
        if dy:
            body = re.sub(r"\((\d+), (\d+), (\d+), (\d+)\)", lambda m: f"({m[1]}, {int(m[2]) + dy}, {m[3]}, {m[4]})", body)
        tr = f"\n            transparent: {trans}," if trans else ""
        blocks.append(f'''        // {out} : {base} recolorie (gen_wardrobe_v064.py)
        (
            source: "{sheet}",{tr}
            sprites: {{
                "{out}": {{{body}}},
            }},
        ),
''')
    OUT_RON.write_text("// 0.6.4 : tenues de la garde-robe. Genere par research/tools/gen_wardrobe_v064.py, ne pas modifier.\n(\n    sheets: [\n"
                       + "".join(blocks) + "    ],\n)\n", encoding="utf-8")
    print(f"ecrit {OUT_RON.name}")


if __name__ == "__main__":
    main()

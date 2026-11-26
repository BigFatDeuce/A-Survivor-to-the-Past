"""0.6.3 : garde-robe. Recolorie la planche de Link avec les palettes du jeu (cotte de mailles bleue
et rouge), relevees dans l'encadre « Palettes » de la planche elle-meme (bandes de couleurs sous
chaque Link, lignes y = 226 / 254 / 282, 2 px par couleur a partir de x = 542).
Ecrit assets/source/Derived - link_blue.png et Derived - link_red.png, lus par
data/sprites/v063_npcs.ron. Lecture seule de la planche d'origine.
"""
from pathlib import Path

from PIL import Image

ROOT = Path(__file__).resolve().parents[2] / "assets" / "source"
SRC = ROOT / "SNES - The Legend of Zelda_ A Link to the Past - Playable Characters - Link.png"
ROWS = {"green": 226, "blue": 254, "red": 282}
# couleurs de la palette qui changent avec la tenue (tunique, bonnet, ceinture)
SLOTS = [8, 9, 10, 11, 13]


def palette(im, y):
    return [im.getpixel((542 + 2 * k, y)) for k in range(15)]


def main():
    im = Image.open(SRC).convert("RGB")
    green = palette(im, ROWS["green"])
    for name in ("blue", "red"):
        pal = palette(im, ROWS[name])
        swap = {green[k]: pal[k] for k in SLOTS}
        out = im.copy()
        px = out.load()
        for y in range(out.height):
            for x in range(out.width):
                c = px[x, y]
                if c in swap:
                    px[x, y] = swap[c]
        path = ROOT / f"Derived - link_{name}.png"
        out.save(path)
        print(f"ecrit {path.name} : {swap}")


if __name__ == "__main__":
    main()

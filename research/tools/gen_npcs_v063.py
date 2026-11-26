"""0.6.3 : recompose la diseuse de bonne aventure (planche « Other Light World NPCs »).

La planche ne donne que des morceaux de 8 px : pour chacune des deux images, le haut du corps
(16x15) et la moitie gauche du bas de la robe (8x8). Le jeu d'origine affiche cette moitie une
seconde fois, retournee, pour la moitie droite. On assemble les deux images (16x23 chacune) dans
assets/source/Derived - fortune_teller.png, lu par data/sprites/v063_npcs.ron.
Lecture seule de la planche d'origine.
"""
from pathlib import Path

from PIL import Image, ImageOps

ROOT = Path(__file__).resolve().parents[2] / "assets" / "source"
SRC = ROOT / "SNES - The Legend of Zelda_ A Link to the Past - Non-Playable Characters - Other Light World NPCs.png"
OUT = ROOT / "Derived - fortune_teller.png"
KEY = (255, 0, 255)
# fonds de case de la planche (rose clair) : transparents eux aussi
BG = {(255, 0, 255), (248, 128, 248), (255, 192, 255), (255, 128, 255)}

# (haut du corps 16x15, moitie gauche du bas 8x8)
FRAMES = [((448, 593), (448, 608)), ((464, 593), (464, 608))]


def clean(im):
    px = im.load()
    for y in range(im.height):
        for x in range(im.width):
            if px[x, y] in BG:
                px[x, y] = KEY
    return im


def main():
    sheet = Image.open(SRC).convert("RGB")
    out = Image.new("RGB", (16 * len(FRAMES), 23), KEY)
    for k, ((ux, uy), (lx, ly)) in enumerate(FRAMES):
        top = clean(sheet.crop((ux, uy, ux + 16, uy + 15)))
        low = clean(sheet.crop((lx, ly, lx + 8, ly + 8)))
        out.paste(top, (16 * k, 0))
        out.paste(low, (16 * k, 15))
        out.paste(ImageOps.mirror(low), (16 * k + 8, 15))
    out.save(OUT)
    print(f"ecrit {OUT.name} ({out.width}x{out.height})")


if __name__ == "__main__":
    main()

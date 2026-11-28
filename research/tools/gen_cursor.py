"""Outil de developpement : curseur de la souris, l'Epee de legende en diagonale (pointe en haut a gauche).

Usage : python research/tools/gen_cursor.py [apercu.png]
Ecrit assets/source/Derived - cursor_master_sword.png (16 x 16), declare dans data/sprites/menu.ron.
Puis : cargo run -p forge -- sprites

Dessin d'apres le modele fourni (lame blanche et lavande, garde doree, gemme rouge, contour noir),
redessine a la main : l'image fournie est agrandie et lissee, sans grille de pixels reguliere.
Pointe = pixel (0, 0) = point actif du curseur.
"""
import sys
from pathlib import Path
from PIL import Image

N = 16
COLORS = {
    "k": (16, 16, 24),     # contour
    "w": (248, 248, 255),  # lame, reflet
    "l": (196, 200, 248),  # lame, lavande
    "b": (128, 136, 224),  # lame, ombre
    "y": (248, 200, 48),   # or
    "o": (200, 128, 24),   # or sombre
    "r": (232, 32, 40),    # gemme
}


def cells():
    """Pixels de couleur (hors contour), par diagonale : d = x - y, s = x + y."""
    out = {}
    for y in range(N):
        for x in range(N):
            d, s = x - y, x + y
            if x >= 1 and y >= 1 and abs(d) <= 1 and s <= 19:
                out[x, y] = {-1: "b", 0: "l", 1: "w"}[d]  # lame
            elif s in (20, 21) and abs(d) <= 6:
                out[x, y] = "y" if s == 20 else "o"  # garde
            # s = 22 et 26 restent vides : trait noir entre garde, poignee et pommeau
            elif 23 <= s <= 25 and abs(d) <= 1:
                out[x, y] = "o" if d else "y"  # poignee
            elif 27 <= s <= 28 and abs(d) <= 1:
                out[x, y] = "y" if s == 28 or d < 0 else "o"  # pommeau
    out[10, 10] = "r"
    return out


def build():
    fill = cells()
    img = Image.new("RGBA", (N, N), (0, 0, 0, 0))
    for y in range(N):
        for x in range(N):
            if (x, y) in fill:
                img.putpixel((x, y), COLORS[fill[x, y]] + (255,))
            elif any((x + dx, y + dy) in fill for dx in (-1, 0, 1) for dy in (-1, 0, 1)):
                img.putpixel((x, y), COLORS["k"] + (255,))
    return img


def main():
    root = Path(__file__).resolve().parents[2]
    img = build()
    assert img.getpixel((0, 0))[3] == 255, "la pointe doit etre en (0, 0)"
    img.save(root / "assets/source/Derived - cursor_master_sword.png")
    if len(sys.argv) > 1:
        img.resize((N * 12, N * 12), Image.NEAREST).save(sys.argv[1])
    print("ok")


if __name__ == "__main__":
    main()

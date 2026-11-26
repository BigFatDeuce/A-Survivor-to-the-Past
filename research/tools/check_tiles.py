"""Verifie que les tuiles de sol se raccordent sans couture : le bord de chaque tuile doit avoir la
couleur moyenne du bord de la tuile de base (sinon on voit des carres sur le sol en jeu).
Usage : python research/tools/check_tiles.py [seuil=30]   (lit data/sprites/*.ron)"""
import glob, os, re, sys
import numpy as np
from PIL import Image

ROOT = os.path.join(os.path.dirname(__file__), "..", "..")
PRE = "SNES - The Legend of Zelda_ A Link to the Past - "
limit = float(sys.argv[1]) if len(sys.argv) > 1 else 30.0
bad = 0
for f in sorted(glob.glob(os.path.join(ROOT, "data", "sprites", "*.ron"))):
    text = open(f, encoding="utf-8").read()
    for m in re.finditer(r'name: "(\w+)",\s*source: "([^"]+)",\s*size: 16,\s*tiles: \[([^\]]*)\]', text):
        name, src, tiles = m.group(1), m.group(2), [tuple(map(int, t)) for t in re.findall(r"\((\d+), (\d+)\)", m.group(3))]
        a = np.array(Image.open(os.path.join(ROOT, "assets", "source", PRE + src)).convert("RGB")).astype(float)

        def border(t):
            x, y = t
            c = a[y:y + 16, x:x + 16]
            return np.concatenate([c[0], c[-1], c[:, 0], c[:, -1]]).mean(0)

        base = border(tiles[0])
        for t in tiles[1:]:
            d = float(np.abs(border(t) - base).sum())
            if d > limit:
                bad += 1
                print(f"{os.path.basename(f)} {name} : tuile {t} ecart {d:.0f}")
print("tuiles qui font des carres :", bad)

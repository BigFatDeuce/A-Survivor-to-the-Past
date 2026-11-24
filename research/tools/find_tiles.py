"""Repere les tuiles 16x16 les plus frequentes d'une region d'une carte (rendu d'overworld).
Sert a choisir le sol d'un stage : tuile de base, touffes, fleurs, cailloux.

Usage : python find_tiles.py <rendu.png> x0 y0 x1 y1 <planche_out.png> [n=48] [ox oy]
Sans ox/oy, l'alignement de la grille est detecte (celui qui donne le moins de tuiles distinctes).
La planche montre les n tuiles les plus frequentes (x3) avec leur numero ; la liste des
coordonnees (x, y) dans le rendu est ecrite sur la sortie standard.
"""
import sys
import numpy as np
from PIL import Image, ImageDraw


def tiles_at(a, ox, oy):
    h, w = a.shape[:2]
    ny, nx = (h - oy) // 16, (w - ox) // 16
    c = a[oy:oy + ny * 16, ox:ox + nx * 16]
    t = c.reshape(ny, 16, nx, 16, -1).transpose(0, 2, 1, 3, 4).reshape(ny * nx, -1)
    return t, nx


def main():
    src = sys.argv[1]
    x0, y0, x1, y1 = map(int, sys.argv[2:6])
    out = sys.argv[6]
    n = int(sys.argv[7]) if len(sys.argv) > 7 else 48
    img = np.array(Image.open(src).convert("RGB"))
    region = img[y0:y1, x0:x1]
    if len(sys.argv) > 9:
        ox, oy = (int(sys.argv[8]) - x0) % 16, (int(sys.argv[9]) - y0) % 16
    else:
        best = None
        for oy in range(16):
            for ox in range(16):
                t, _ = tiles_at(region, ox, oy)
                u = len(np.unique(t, axis=0))
                if best is None or u < best[0]:
                    best = (u, ox, oy)
        _, ox, oy = best
    t, nx = tiles_at(region, ox, oy)
    vals, idx, counts = np.unique(t, axis=0, return_index=True, return_counts=True)
    order = np.argsort(-counts)[:n]
    print(f"grille : decalage absolu ({(x0 + ox) % 16}, {(y0 + oy) % 16}) mod 16, {len(vals)} tuiles distinctes")
    cols = 8
    rows = (len(order) + cols - 1) // cols
    sheet = Image.new("RGB", (cols * 56, rows * 64), (40, 40, 40))
    d = ImageDraw.Draw(sheet)
    total = counts.sum()
    for k, o in enumerate(order):
        i = idx[o]
        ty, tx = divmod(i, nx)
        x, y = x0 + ox + tx * 16, y0 + oy + ty * 16
        tile = Image.fromarray(img[y:y + 16, x:x + 16]).resize((48, 48), Image.NEAREST)
        cx, cy = (k % cols) * 56 + 4, (k // cols) * 64 + 2
        sheet.paste(tile, (cx, cy))
        d.text((cx, cy + 49), f"{k}", fill=(255, 255, 0))
        print(f"{k:3d} ({x}, {y})  {counts[o] / total * 100:5.1f} %")
    sheet.save(out)


if __name__ == "__main__":
    main()

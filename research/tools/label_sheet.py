"""Outil de developpement : numerote les sprites (composantes) d'une zone de planche
et produit une image agrandie annotee, pour ecrire les manifestes d'assets.
Usage : python label_sheet.py <planche.png> <x> <y> <w> <h> <echelle> <sortie.png> [--cells COULEUR_HEX]
  --cells : la planche range chaque frame dans une case d'une couleur unie (ex. Link : 004040)
Affiche aussi la liste "index: x y w h" (coordonnees dans la planche d'origine).
"""
import sys
import numpy as np
from PIL import Image, ImageDraw

def comps(mask):
    h, w = mask.shape
    seen = np.zeros_like(mask)
    out = []
    ys, xs = np.nonzero(mask)
    for y0, x0 in zip(ys, xs):
        if seen[y0, x0]:
            continue
        st = [(y0, x0)]; seen[y0, x0] = True
        x1 = x2 = x0; y1 = y2 = y0
        while st:
            y, x = st.pop()
            x1 = min(x1, x); x2 = max(x2, x); y1 = min(y1, y); y2 = max(y2, y)
            for ny in (y - 1, y, y + 1):
                if 0 <= ny < h:
                    for nx in (x - 1, x, x + 1):
                        if 0 <= nx < w and mask[ny, nx] and not seen[ny, nx]:
                            seen[ny, nx] = True; st.append((ny, nx))
        out.append((x1, y1, x2 - x1 + 1, y2 - y1 + 1))
    return out

def main():
    a = sys.argv
    src = a[1]; x, y, w, h, sc = map(int, a[2:7]); out = a[7]
    cells = None
    if "--cells" in a:
        cells = int(a[a.index("--cells") + 1], 16)
    im = Image.open(src).convert("RGB")
    arr = np.array(im).astype(np.int32)
    key = arr[..., 0] * 65536 + arr[..., 1] * 256 + arr[..., 2]
    sub = key[y:y + h, x:x + w]
    if cells is not None:
        mask = sub != int(np.bincount(sub.ravel() & 0xFFFFFF).argmax()) if False else None
        # les cases : tout ce qui n'est pas le fond principal
        vals, cnt = np.unique(key, return_counts=True)
        bg = vals[cnt.argmax()]
        mask = sub != bg
    else:
        vals, cnt = np.unique(key, return_counts=True)
        bg = vals[cnt.argmax()]
        mask = ~((sub == bg) | (sub == 0xF880F8) | (sub == 0xFFC0FF) | (sub == 0xFF80FF))
    boxes = [b for b in comps(mask) if b[2] >= 4 and b[3] >= 4]
    boxes.sort(key=lambda b: (b[1] // 12, b[0]))
    big = im.crop((x, y, x + w, y + h)).resize((w * sc, h * sc), Image.NEAREST)
    d = ImageDraw.Draw(big)
    for i, (bx, by, bw, bh) in enumerate(boxes):
        d.rectangle([bx * sc, by * sc, (bx + bw) * sc - 1, (by + bh) * sc - 1], outline=(255, 255, 0))
        d.text((bx * sc + 1, by * sc + 1), str(i), fill=(255, 255, 255), stroke_width=2, stroke_fill=(0, 0, 0))
        print(f"{i}: {bx + x} {by + y} {bw} {bh}")
    big.save(out)

main()

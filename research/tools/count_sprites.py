"""Compte les sprites detectables automatiquement dans chaque planche (fond + cadres ignores)."""
import sys, glob, os
import numpy as np
from PIL import Image
sys.setrecursionlimit(10000)

def count(path):
    a = np.array(Image.open(path).convert("RGB")).astype(np.int32)
    key = a[..., 0] * 65536 + a[..., 1] * 256 + a[..., 2]
    vals, counts = np.unique(key, return_counts=True)
    bg = vals[counts.argmax()]
    mask = ~((key == bg) | (key == 0xF880F8) | (key == 0xFFC0FF) | (key == 0xFF80FF))
    h, w = mask.shape
    seen = np.zeros_like(mask)
    n = 0
    ys, xs = np.nonzero(mask)
    for y0, x0 in zip(ys, xs):
        if seen[y0, x0]: continue
        st = [(y0, x0)]; seen[y0, x0] = True
        x1 = x2 = x0; y1 = y2 = y0; c = 0
        while st:
            y, x = st.pop(); c += 1
            if x < x1: x1 = x
            if x > x2: x2 = x
            if y < y1: y1 = y
            if y > y2: y2 = y
            for ny in (y - 1, y, y + 1):
                if ny < 0 or ny >= h: continue
                for nx in (x - 1, x, x + 1):
                    if 0 <= nx < w and mask[ny, nx] and not seen[ny, nx]:
                        seen[ny, nx] = True; st.append((ny, nx))
        bw, bh = x2 - x1 + 1, y2 - y1 + 1
        if 8 <= bw <= 96 and 8 <= bh <= 96 and c / (bw * bh) < 0.97:
            n += 1
    return n

root = sys.argv[1]
tot = 0
for f in sorted(glob.glob(os.path.join(root, "*.png"))):
    b = os.path.basename(f).replace("SNES - The Legend of Zelda_ A Link to the Past - ", "")
    if any(k in b for k in ("Overworld", "Dungeons", "Caves", "House", "Map", "Title", "Status", "Input", "Fog", "Canopy", "Rain", "Font")):
        continue
    n = count(f); tot += n
    print(f"{n:5d}  {b}")
print(f"{tot:5d}  TOTAL")

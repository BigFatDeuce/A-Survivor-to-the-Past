"""Prototype de decoupage automatique d'une planche Spriters Resource.
Detecte la couleur de fond, isole les sprites par composantes connexes,
ignore le texte et les nuanciers, et exporte un atlas + un JSON de cadres.
Usage: python autoslice.py <planche.png> <atlas_out.png> [cell=32] [max=64]
"""
import sys, json
import numpy as np
from PIL import Image

def components(mask):
    h, w = mask.shape
    lab = np.zeros((h, w), np.int32); boxes = []; cur = 0
    ys, xs = np.nonzero(mask)
    for y0, x0 in zip(ys, xs):
        if lab[y0, x0]: continue
        cur += 1; st = [(y0, x0)]; lab[y0, x0] = cur
        x1 = x2 = x0; y1 = y2 = y0; cnt = 0
        while st:
            y, x = st.pop(); cnt += 1
            x1 = min(x1, x); x2 = max(x2, x); y1 = min(y1, y); y2 = max(y2, y)
            for dy in (-1, 0, 1):
                for dx in (-1, 0, 1):
                    ny, nx = y + dy, x + dx
                    if 0 <= ny < h and 0 <= nx < w and mask[ny, nx] and not lab[ny, nx]:
                        lab[ny, nx] = cur; st.append((ny, nx))
        boxes.append((x1, y1, x2 + 1, y2 + 1, cnt))
    return boxes

def main():
    src, out = sys.argv[1], sys.argv[2]
    cell = int(sys.argv[3]) if len(sys.argv) > 3 else 32
    mx = int(sys.argv[4]) if len(sys.argv) > 4 else 64
    a = np.array(Image.open(src).convert("RGBA"))
    rgb = a[..., :3].astype(np.int32)
    flat = rgb.reshape(-1, 3)
    vals, counts = np.unique(flat[:, 0] * 65536 + flat[:, 1] * 256 + flat[:, 2], return_counts=True)
    bg = vals[counts.argmax()]
    key = rgb[..., 0] * 65536 + rgb[..., 1] * 256 + rgb[..., 2]
    # fond principal + rose clair des cadres (#f880f8) consideres comme transparents
    transparent = (key == bg) | (key == 0xF880F8) | (key == 0xFFC0FF) | (key == 0xFF80FF)
    mask = ~transparent
    boxes = components(mask)
    frames = []
    for (x1, y1, x2, y2, cnt) in boxes:
        w, h = x2 - x1, y2 - y1
        fill = cnt / (w * h)
        # filtres : taille d'un sprite de jeu, pas une lettre isolee ni un nuancier plein
        if 10 <= w <= cell and 10 <= h <= cell and fill < 0.97:
            frames.append((y1 // 8, x1, x1, y1, x2, y2))
    frames.sort()
    frames = frames[:mx]
    cols = 8; rows = (len(frames) + cols - 1) // cols
    atlas = Image.new("RGBA", (cols * cell, rows * cell), (0, 0, 0, 0))
    meta = []
    src_img = Image.fromarray(np.where(transparent[..., None], 0, a).astype(np.uint8), "RGBA")
    for i, (_, _, x1, y1, x2, y2) in enumerate(frames):
        crop = src_img.crop((x1, y1, x2, y2))
        cx, cy = (i % cols) * cell, (i // cols) * cell
        atlas.paste(crop, (cx + (cell - crop.width) // 2, cy + cell - crop.height))
        meta.append({"src": [int(x1), int(y1), int(x2 - x1), int(y2 - y1)], "cell": i})
    atlas.save(out)
    json.dump({"source": src, "cell": cell, "cols": cols, "frames": meta}, open(out + ".json", "w"), indent=1)
    print(f"fond=#{bg:06x} composantes={len(boxes)} sprites retenus={len(frames)} atlas={atlas.size}")

main()

"""Construit la planche du decor des stages : assets/source/props/stage_props.png.

Les elements (arbres, souches, squelettes...) sont decoupes dans les rendus d'overworld : le sol
autour est retire par remplissage depuis les bords (couleurs du bord du cadre), puis seuls les
gros morceaux sont gardes (les touffes d'herbe voisines disparaissent). Les textures de
brouillard et de feuillage deviennent des masques blancs / noirs avec transparence.

Usage : python research/tools/make_props.py [apercu.png]
Ecrit la planche et affiche les cadres a recopier dans data/sprites/stages.ron.
"""
import sys
from collections import deque
import numpy as np
from PIL import Image

SRC = "assets/source/SNES - The Legend of Zelda_ A Link to the Past - "
LW = SRC + "Overworld - Light World.png"
DW = SRC + "Overworld - Dark World.png"
OBJ = SRC + "Enemies - Object Enemies & Traps.png"
OUT = "assets/source/props/stage_props.png"

# nom, planche, cadre approximatif (x, y, l, h) avec une marge de sol autour ; option : tuile de sol
# de reference (x, y) dont les couleurs servent de fond quand le cadre colle a l'element
PROPS = [
    ("tree", LW, (1232, 3686, 72, 90)),
    ("stump", LW, (1248, 3816, 40, 38)),
    ("bush", LW, (2472, 3932, 16, 16), (2408, 3708)),
    ("skull", LW, (604, 4744, 42, 44)),
    ("desert_plant", LW, (572, 4954, 26, 34)),
    ("cactus", OBJ, (4, 916, 24, 32), [0xF880F8]),
    ("boulder", OBJ, (334, 292, 40, 40), [0xF880F8]),
    ("tree_dark", DW, (1276, 2748, 72, 88)),
    ("stump_dark", DW, (1290, 2876, 44, 42)),
]

# textures 256x256 (interieur du cadre de la planche)
TEXTURES = [
    ("fog", SRC + "Effects - Fog.png", (2, 2, 256, 256)),
    ("canopy", SRC + "Effects - Lost Woods Canopy.png", (2, 2, 256, 256)),
]

_cache = {}


def sheet(path):
    if path not in _cache:
        _cache[path] = np.array(Image.open(path).convert("RGB"))
    return _cache[path]


def keys(a):
    return a[..., 0].astype(np.int32) << 16 | a[..., 1].astype(np.int32) << 8 | a[..., 2]


def cut(path, rect, ground=None):
    x, y, w, h = rect
    a = sheet(path)[y:y + h, x:x + w]
    key = keys(a)
    if isinstance(ground, tuple):
        gx, gy = ground
        border = set(keys(sheet(path)[gy:gy + 16, gx:gx + 16]).ravel().tolist())
    else:
        border = set(key[0].tolist()) | set(key[-1].tolist()) | set(key[:, 0].tolist()) | set(key[:, -1].tolist())
        # couleurs de fond supplementaires (cadres rose clair des planches)
        border |= set(ground or [])
    bg = np.zeros((h, w), bool)
    q = deque()
    for yy in range(h):
        for xx in (0, w - 1):
            q.append((yy, xx))
    for xx in range(w):
        for yy in (0, h - 1):
            q.append((yy, xx))
    while q:
        yy, xx = q.popleft()
        if bg[yy, xx] or key[yy, xx] not in border:
            continue
        bg[yy, xx] = True
        for dy, dx in ((1, 0), (-1, 0), (0, 1), (0, -1)):
            ny, nx = yy + dy, xx + dx
            if 0 <= ny < h and 0 <= nx < w and not bg[ny, nx]:
                q.append((ny, nx))
    # garde les gros morceaux (8-connexes)
    fg = ~bg
    lab = np.zeros((h, w), np.int32)
    sizes = [0]
    for sy in range(h):
        for sx in range(w):
            if fg[sy, sx] and not lab[sy, sx]:
                n = len(sizes)
                lab[sy, sx] = n
                st = [(sy, sx)]
                cnt = 0
                while st:
                    yy, xx = st.pop()
                    cnt += 1
                    for dy in (-1, 0, 1):
                        for dx in (-1, 0, 1):
                            ny, nx = yy + dy, xx + dx
                            if 0 <= ny < h and 0 <= nx < w and fg[ny, nx] and not lab[ny, nx]:
                                lab[ny, nx] = n
                                st.append((ny, nx))
                sizes.append(cnt)
    big = max(sizes)
    keep = np.isin(lab, [i for i, s in enumerate(sizes) if i and s >= big * 0.15])
    ys, xs = np.nonzero(keep)
    y0, y1, x0, x1 = ys.min(), ys.max() + 1, xs.min(), xs.max() + 1
    rgba = np.zeros((h, w, 4), np.uint8)
    rgba[..., :3] = a
    rgba[..., 3] = np.where(keep, 255, 0)
    return rgba[y0:y1, x0:x1]


def texture(path, rect, mode):
    x, y, w, h = rect
    a = np.array(Image.open(path).convert("L"))[y:y + h, x:x + w].astype(np.float32) / 255.0
    rgba = np.zeros((h, w, 4), np.uint8)
    if mode == "fog":
        # blanc, opacite selon la luminosite (noir = trou dans la brume)
        rgba[..., :3] = 255
        rgba[..., 3] = (a * 255).astype(np.uint8)
    else:
        # ombre du feuillage : noir partout, sauf les taches de lumiere
        rgba[..., 3] = ((1.0 - a) * 255).astype(np.uint8)
    return rgba


def main():
    items = [(e[0], cut(*e[1:])) for e in PROPS] + [(n, texture(p, r, n)) for n, p, r in TEXTURES]
    # rangement simple en etageres
    W = 512
    x = y = row = 0
    places = []
    for n, im in items:
        h, w = im.shape[:2]
        if x + w > W:
            x, y, row = 0, y + row + 2, 0
        places.append((n, x, y, w, h))
        x += w + 2
        row = max(row, h)
    H = y + row
    out = np.zeros((H, W, 4), np.uint8)
    for (n, x, y, w, h), (_, im) in zip(places, items):
        out[y:y + h, x:x + w] = im
    Image.fromarray(out, "RGBA").save(OUT)
    for n, x, y, w, h in places:
        print(f'"{n}": ({x}, {y}, {w}, {h}),')
    if len(sys.argv) > 1:
        prev = Image.new("RGBA", (W, H), (60, 60, 90, 255))
        prev.alpha_composite(Image.fromarray(out, "RGBA"))
        prev.resize((W * 2, H * 2), Image.NEAREST).save(sys.argv[1])


if __name__ == "__main__":
    main()

"""Retrouve comment le jeu d'origine compose chaque image de marche de Link a partir de sa planche
de tuiles (.zspr) : pour chaque image de la planche « Playable Characters - Link » utilisee par le
jeu, cherche les blocs 16x16 et 8x8 (eventuellement retournes) qui la reconstituent, en comparant
les indices de couleur. Ecrit research/zspr_poses.json, lu par gen_zspr_chars.py pour dessiner les
memes poses avec les tuiles de n'importe quel sprite du Randomizer.

Usage : python research/tools/zspr_poses.py <link.zspr du Randomizer (Nintendo)>
"""
import json
import sys
from pathlib import Path

import numpy as np
from PIL import Image

sys.path.insert(0, str(Path(__file__).parent))
import zspr  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
SHEET = ROOT / "assets" / "source" / "SNES - The Legend of Zelda_ A Link to the Past - Playable Characters - Link.png"
OUT = ROOT / "research" / "zspr_poses.json"
TRANSPARENT = {(0, 128, 128), (0, 64, 64)}
# images du sprite « link » (data/sprites/j1_base.ron) : (x, y, l, h)
FRAMES = {
    "idle_down": [(1, 3, 16, 24)],
    "idle_side": [(1, 58, 17, 24)],
    "idle_up": [(1, 111, 16, 24)],
    "walk_down": [(19, 3, 16, 24), (36, 2, 16, 25), (53, 1, 16, 26), (70, 3, 16, 24), (87, 3, 16, 24), (104, 2, 16, 25), (121, 1, 16, 26)],
    "walk_side": [(20, 57, 17, 25), (38, 57, 16, 25), (55, 58, 17, 24), (73, 58, 17, 24), (91, 57, 17, 25), (109, 57, 16, 25), (126, 58, 16, 24)],
    "walk_up": [(19, 111, 16, 24), (36, 110, 16, 25), (53, 110, 16, 25), (70, 111, 16, 24), (87, 111, 16, 24), (104, 110, 16, 25), (121, 110, 16, 25)],
}
# lapin (Monde des Tenebres sans la Perle de Lune), palette 3 : images de « link_bunny » (v064_wardrobe.ron)
BUNNY = {
    "bunny_idle_down": [(361, 194, 16, 23)],
    "bunny_idle_side": [(412, 193, 17, 24)],
    "bunny_idle_up": [(448, 194, 16, 23)],
    "bunny_walk_down": [(361, 194, 16, 23), (378, 193, 16, 24), (395, 194, 16, 23), (378, 193, 16, 24)],
    "bunny_walk_side": [(412, 193, 17, 24), (430, 194, 17, 23)],
    "bunny_walk_up": [(448, 194, 16, 23), (465, 193, 16, 24), (482, 194, 16, 23), (465, 193, 16, 24)],
}


def target(im, pal, box):
    """Indices de couleur de l'image (-1 : transparent), couleur la plus proche de la palette."""
    x, y, w, h = box
    pal = np.array(pal[1:], dtype=np.int32)
    t = np.full((h, w), -1, dtype=np.int32)
    for j in range(h):
        for i in range(w):
            c = im.getpixel((x + i, y + j))[:3]
            if c in TRANSPARENT:
                continue
            d = ((pal - np.array(c)) ** 2).sum(1)
            t[j, i] = int(d.argmin()) + 1
    return t


def blocks(sheet):
    """Blocs candidats : (taille, x, y, retourne, pixels)."""
    out = []
    h, w = sheet.shape
    for size, sx in ((16, 16), (8, 8)):
        for y in range(0, h - size + 1, 8):
            for x in range(0, w - size + 1, sx):
                b = sheet[y:y + size, x:x + size]
                if (b > 0).sum() < 6:
                    continue
                out.append((size, x, y, False, b))
                out.append((size, x, y, True, b[:, ::-1]))
    return out


def solve(t, cands):
    """Couverture gloutonne : a chaque tour, le bloc qui explique le plus de pixels encore libres.
    Un bloc peut differer de l'image la ou un bloc deja choisi (dessine par-dessus) la couvre."""
    h, w = t.shape
    covered = np.zeros((h, w), bool)
    picks = []
    pad = 16
    tp = np.full((h + 2 * pad, w + 2 * pad), -1, dtype=np.int32)
    tp[pad:pad + h, pad:pad + w] = t
    for _ in range(8):
        cov = np.ones_like(tp, dtype=bool)
        cov[pad:pad + h, pad:pad + w] = covered
        free = (tp > 0) & ~cov
        if free.sum() < 3:
            break
        best = None
        for (size, bx, by, flip, b) in cands:
            nz = b > 0
            # toutes les positions (dx, dy) ou le bloc chevauche l'image
            win = np.lib.stride_tricks.sliding_window_view(tp, (size, size))
            cwin = np.lib.stride_tricks.sliding_window_view(cov, (size, size))
            fwin = np.lib.stride_tricks.sliding_window_view(free, (size, size))
            ok = ((win == b) | ~nz | cwin).all(axis=(2, 3))
            # le bloc ne doit pas dessiner sur du transparent libre
            ok &= ~((win == -1) & nz & ~cwin).any(axis=(2, 3))
            gain = np.where(ok, (fwin & nz).sum(axis=(2, 3)), 0)
            k = np.unravel_index(gain.argmax(), gain.shape)
            g = gain[k]
            if best is None or g > best[0]:
                best = (g, size, bx, by, flip, k[1] - pad, k[0] - pad)
        if best is None or best[0] < 3:
            break
        g, size, bx, by, flip, dx, dy = best
        picks.append({"size": int(size), "sx": int(bx), "sy": int(by), "flip": bool(flip), "dx": int(dx), "dy": int(dy)})
        b = next(c[4] for c in cands if c[:4] == (size, bx, by, flip))
        for j in range(size):
            for i in range(size):
                y, x = dy + j, dx + i
                if b[j, i] > 0 and 0 <= y < h and 0 <= x < w:
                    covered[y, x] = True
    miss = int(((t > 0) & ~covered).sum())
    return picks, miss


def main():
    z = zspr.read(sys.argv[1])
    sheet = np.array(zspr.indices(z["gfx"]), dtype=np.int32)
    pals = zspr.palettes(z["pal"])
    im = Image.open(SHEET).convert("RGB")
    cands = blocks(sheet)
    print(f"{len(cands)} blocs candidats")
    out = {}
    for anim, boxes in list(FRAMES.items()) + list(BUNNY.items()):
        out[anim] = []
        for box in boxes:
            t = target(im, pals[3 if anim in BUNNY else 0], box)
            picks, miss = solve(t, cands)
            out[anim].append({"w": box[2], "h": box[3], "pieces": picks, "missing": miss})
            print(f"{anim} {box} : {len(picks)} blocs, {miss} pixels non expliques (bouclier, epee...)")
    OUT.write_text(json.dumps(out, indent=1), encoding="utf-8")
    print(f"ecrit {OUT}")


if __name__ == "__main__":
    main()

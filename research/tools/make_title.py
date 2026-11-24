"""Prepare l'ecran titre a partir de menubg.webp (1920x1080, pixel art 256x144 agrandi x7,5).

Sorties (assets/source/title/) :
  - title_bg.png   : decor 256x144 sans le logo (le decor derriere le logo est reconstruit ligne par ligne)
  - title_logo.png : logo (epee + Z + Triforce) detoure, fond transparent

Usage : python research/tools/make_title.py
"""
from collections import Counter, deque
from pathlib import Path

import numpy as np
from PIL import Image

ROOT = Path(__file__).resolve().parents[2]
SRC = ROOT / "assets/source/menubg.webp"
OUT = ROOT / "assets/source/title"
NW, NH = 256, 144
# colonnes du logo (pixels natifs) et bande de decor propre a sa droite (avant le chateau)
LOGO_X0, LOGO_X1 = 84, 174
STRIP_X0, STRIP_X1 = 172, 214


def to_native(path: Path) -> np.ndarray:
    im = np.array(Image.open(path).convert("RGB")).astype(float)
    s = im.shape[1] / NW
    out = np.zeros((NH, NW, 3))
    for j in range(NH):
        for i in range(NW):
            y0, y1 = int(j * s + 1.5), int((j + 1) * s - 1.5)
            x0, x1 = int(i * s + 1.5), int((i + 1) * s - 1.5)
            out[j, i] = np.median(im[y0 : y1 + 1, x0 : x1 + 1].reshape(-1, 3), axis=0)
    return out


def quantize(o: np.ndarray, tol: int = 14) -> np.ndarray:
    """Regroupe les couleurs bruitees (compression webp) en une palette propre."""
    px = o.reshape(-1, 3)
    cols, counts = np.unique(px.round(), axis=0, return_counts=True)
    centers, members = [], []
    for k in np.argsort(-counts):
        c = cols[k]
        for ci, cc in enumerate(centers):
            if np.abs(cc - c).max() <= tol:
                members[ci].append((c, counts[k]))
                break
        else:
            centers.append(c.copy())
            members.append([(c, counts[k])])
    cent = np.array([np.average([m[0] for m in ms], axis=0, weights=[m[1] for m in ms]) for ms in members])
    lab = ((px[:, None, :] - cent[None, :, :]) ** 2).sum(-1).argmin(1)
    return cent[lab].round().astype(int).reshape(o.shape)


def components(mask: np.ndarray):
    h, w = mask.shape
    lab = -np.ones((h, w), int)
    comps = []
    for y in range(h):
        for x in range(w):
            if mask[y, x] and lab[y, x] < 0:
                i = len(comps)
                dq = deque([(y, x)])
                lab[y, x] = i
                pts = []
                while dq:
                    cy, cx = dq.popleft()
                    pts.append((cy, cx))
                    for dy, dx in ((1, 0), (-1, 0), (0, 1), (0, -1)):
                        ny, nx = cy + dy, cx + dx
                        if 0 <= ny < h and 0 <= nx < w and mask[ny, nx] and lab[ny, nx] < 0:
                            lab[ny, nx] = i
                            dq.append((ny, nx))
                comps.append(pts)
    return comps


def fill_holes(m: np.ndarray) -> np.ndarray:
    h, w = m.shape
    outside = np.zeros_like(m)
    dq = deque()
    for x in range(w):
        for y in (0, h - 1):
            if not m[y, x]:
                outside[y, x] = True
                dq.append((y, x))
    for y in range(h):
        for x in (0, w - 1):
            if not m[y, x] and not outside[y, x]:
                outside[y, x] = True
                dq.append((y, x))
    while dq:
        y, x = dq.popleft()
        for dy, dx in ((1, 0), (-1, 0), (0, 1), (0, -1)):
            ny, nx = y + dy, x + dx
            if 0 <= ny < h and 0 <= nx < w and not m[ny, nx] and not outside[ny, nx]:
                outside[ny, nx] = True
                dq.append((ny, nx))
    return ~outside


def main():
    q = quantize(to_native(SRC))
    key = q[:, :, 0] * 65536 + q[:, :, 1] * 256 + q[:, :, 2]
    inside = np.zeros(key.shape, bool)
    inside[:, LOGO_X0:LOGO_X1] = True
    # couleurs presentes uniquement dans la zone du logo = couleurs du logo
    only = [c for c in np.unique(key) if not ((key == c) & ~inside).any()]
    seed = np.isin(key, only)
    logo = fill_holes(seed)
    # les trous du logo qui montrent la foret (centre de la Triforce) restent du decor
    for pts in components(logo & ~seed):
        greens = sum(1 for y, x in pts if q[y, x, 1] > q[y, x, 0] + 40 and q[y, x, 1] > q[y, x, 2] + 20)
        if len(pts) > 60 and greens > 0:
            for y, x in pts:
                logo[y, x] = False

    bg = q.copy()
    for y in range(NH):
        xs = np.flatnonzero(logo[y])
        strip = key[y, STRIP_X0:STRIP_X1]
        c, n = Counter(strip.tolist()).most_common(1)[0]
        flat = n / len(strip) >= 0.8
        if len(xs) and flat:
            bg[y, xs] = q[y, STRIP_X0 + np.flatnonzero(strip == c)[0]]
        elif len(xs):
            # ligne a motif (foret, reflets) : periode la plus probable, puis recopie en phase
            best = min(range(4, 33), key=lambda p: (strip[:-p] != strip[p:]).mean())
            for x in xs:
                bg[y, x] = q[y, STRIP_X0 + (x - STRIP_X0) % best]
        if n / len(strip) >= 0.95:
            # pixels isoles laisses par l'anticrenelage du logo
            outside = set(key[y, :LOGO_X0].tolist()) | set(key[y, LOGO_X1:].tolist())
            ref = q[y, STRIP_X0 + np.flatnonzero(strip == c)[0]]
            for x in range(LOGO_X0, LOGO_X1):
                if key[y, x] not in outside:
                    bg[y, x] = ref

    OUT.mkdir(parents=True, exist_ok=True)
    Image.fromarray(bg.astype(np.uint8)).save(OUT / "title_bg.png")
    ys, xs = np.nonzero(logo)
    y0, y1, x0, x1 = ys.min(), ys.max() + 1, xs.min(), xs.max() + 1
    rgba = np.zeros((y1 - y0, x1 - x0, 4), np.uint8)
    rgba[..., :3] = q[y0:y1, x0:x1]
    rgba[..., 3] = np.where(logo[y0:y1, x0:x1], 255, 0)
    Image.fromarray(rgba).save(OUT / "title_logo.png")
    print(f"title_bg.png {NW}x{NH}, title_logo.png {x1 - x0}x{y1 - y0} (centre d'origine x={(x0 + x1) / 2:.1f}, y={(y0 + y1) / 2:.1f})")


if __name__ == "__main__":
    main()

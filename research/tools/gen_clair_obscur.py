"""Outil de developpement : personnages de Clair Obscur (planches 4 x 4 fournies, pixel art agrandi).

Usage : python research/tools/gen_clair_obscur.py <dossier des planches>
  planche <Nom>.png : rangees bas, haut, gauche, droite ; colonnes repos, pas 1, pas 2, attaque.
Ecrit :
  assets/source/clair_obscur/<id>.png   repos et pas de chaque direction, a la taille du jeu
  data/sprites/clair_obscur.ron         animations idle_* / walk_* (comme les autres personnages)
Puis : cargo run -p forge -- sprites

Noirs remontes au niveau du contour d'ALttP (40,40,40) : `soften_darks`.

Reduction : le pixel art d'origine n'a pas une grille reguliere ; chaque pixel du jeu prend la couleur
la plus frequente de son bloc (pas de melange de couleurs), opaque si la moitie du bloc l'est. Les trois
images d'une direction sont decoupees dans le meme cadre (pas de tremblement pendant la marche).
"""
import sys
from collections import Counter
from pathlib import Path

import numpy as np
from PIL import Image

ROOT = Path(__file__).resolve().parents[2]
OUT_PNG = ROOT / "assets" / "source" / "clair_obscur"
OUT_RON = ROOT / "data" / "sprites" / "clair_obscur.ron"

# planche, identifiant, hauteur a l'ecran (pixels du jeu : Link en fait 24) ; Esquie est un geant
CHARS = [
    ("Gustave", "co_gustave", 28),
    ("Maelle", "co_maelle", 26),
    ("Lune", "co_lune", 27),
    ("Sciel", "co_sciel", 27),
    ("Monoco", "co_monoco", 28),
    ("Verso", "co_verso", 28),
    ("Esquie", "co_esquie", 34),
]
ROWS = {"down": 0, "up": 1, "side": 2}  # rangee 2 : regarde a gauche (side_faces_left)
COLS = 3  # repos, pas 1, pas 2 (l'attaque n'est pas utilisee)


def cells(img):
    a = np.array(img.convert("RGBA"))
    c = a.shape[1] / 4
    for r in range(4):
        for col in range(4):
            keep_body(a[int(r * c):int((r + 1) * c), int(col * c):int((col + 1) * c)])
    return a, c


def keep_body(cell):
    """Efface les morceaux detaches (bouts de la case voisine) : garde le plus grand ensemble de pixels
    opaques et ceux d'au moins un cinquieme de sa taille (arme tenue a part)."""
    s = 4
    m = cell[::s, ::s, 3] > 128
    lab = np.zeros(m.shape, int)
    sizes = [0]
    for y0, x0 in zip(*np.where(m)):
        if lab[y0, x0]:
            continue
        n = len(sizes)
        lab[y0, x0] = n
        todo, count = [(y0, x0)], 0
        while todo:
            y, x = todo.pop()
            count += 1
            for dy, dx in ((1, 0), (-1, 0), (0, 1), (0, -1)):
                v, u = y + dy, x + dx
                if 0 <= v < m.shape[0] and 0 <= u < m.shape[1] and m[v, u] and not lab[v, u]:
                    lab[v, u] = n
                    todo.append((v, u))
        sizes.append(count)
    if len(sizes) <= 2:
        return
    big = max(sizes)
    drop = [k for k, n in enumerate(sizes) if k and n * 5 < big]
    full = np.kron(np.isin(lab, drop), np.ones((s, s), bool))[:cell.shape[0], :cell.shape[1]]
    cell[full, 3] = 0


def union_box(a, c, row):
    """Cadre commun (x0, y0, x1, y1) des images repos / pas d'une rangee, en coordonnees de cellule."""
    box = None
    for col in range(COLS):
        cell = a[int(row * c):int((row + 1) * c), int(col * c):int((col + 1) * c)]
        ys, xs = np.where(cell[:, :, 3] > 128)
        b = (xs.min(), ys.min(), xs.max() + 1, ys.max() + 1)
        box = b if box is None else (min(box[0], b[0]), min(box[1], b[1]), max(box[2], b[2]), max(box[3], b[3]))
    return box


def reduce(block_img, f):
    """Reduction par la couleur majoritaire de chaque bloc (f : pixels d'origine par pixel du jeu)."""
    h, w = block_img.shape[:2]
    ow, oh = max(1, round(w / f)), max(1, round(h / f))
    out = np.zeros((oh, ow, 4), np.uint8)
    for y in range(oh):
        y0, y1 = int(y * h / oh), max(int(y * h / oh) + 1, int((y + 1) * h / oh))
        for x in range(ow):
            x0, x1 = int(x * w / ow), max(int(x * w / ow) + 1, int((x + 1) * w / ow))
            blk = block_img[y0:y1, x0:x1].reshape(-1, 4)
            solid = blk[blk[:, 3] > 128]
            if len(solid) * 2 < len(blk):
                continue
            # couleurs proches regroupees (bruit de compression), puis la plus frequente
            q = Counter(tuple((p[:3] // 8).tolist()) for p in solid).most_common(1)[0][0]
            pick = solid[np.all(solid[:, :3] // 8 == q, axis=1)]
            out[y, x, :3] = np.median(pick[:, :3], axis=0).astype(np.uint8)
            out[y, x, 3] = 255
    return out


def frames_of(sheet, target_h):
    a, c = cells(Image.open(sheet))
    # echelle commune a tout le personnage : hauteur de l'image de repos de face
    d = union_box(a, c, ROWS["down"])
    f = (d[3] - d[1]) / target_h
    out = {}
    for name, row in ROWS.items():
        x0, y0, x1, y1 = union_box(a, c, row)
        imgs = []
        for col in range(COLS):
            cell = a[int(row * c):int((row + 1) * c), int(col * c):int((col + 1) * c)]
            imgs.append(reduce(cell[y0:y1, x0:x1], f))
        out[name] = imgs
    return out


# Contour des sprites d'A Link to the Past (et des sprites Randomizer) : gris tres sombre 40,40,40.
OUTLINE = 40
DARK = 80


def soften_darks(a):
    """Noirs des planches (contour presque noir) remontes au niveau du contour d'ALttP : les couleurs
    sombres (canaux < DARK) sont ramenees dans [OUTLINE, DARK], ordre et teinte gardes ; le reste ne
    change pas. Sinon ces personnages ressortent bien plus que les autres."""
    rgb = a[..., :3].astype(np.float32)
    dark = (rgb.max(axis=2) < DARK) & (a[..., 3] > 0)
    rgb[dark] = OUTLINE + rgb[dark] * (DARK - OUTLINE) / DARK
    out = a.copy()
    out[..., :3] = np.round(rgb).astype(np.uint8)
    return out


def main(src):
    OUT_PNG.mkdir(parents=True, exist_ok=True)
    lines = ["// Personnages de Clair Obscur : genere par research/tools/gen_clair_obscur.py (ne pas modifier a la main).", "(", "    sheets: ["]
    for sheet, cid, h in CHARS:
        fr = frames_of(Path(src) / f"{sheet}.png", h)
        # une rangee : repos, pas 1, pas 2 de chaque direction, 1 pixel d'ecart
        flat = [(d, k, img) for d in ROWS for k, img in enumerate(fr[d])]
        width = sum(img.shape[1] + 1 for _, _, img in flat)
        height = max(img.shape[0] for _, _, img in flat)
        canvas = np.zeros((height, width, 4), np.uint8)
        rects = {}
        x = 0
        for d, k, img in flat:
            ih, iw = img.shape[:2]
            canvas[height - ih:height, x:x + iw] = img
            rects[(d, k)] = (x, height - ih, iw, ih)
            x += iw + 1
        Image.fromarray(soften_darks(canvas)).save(OUT_PNG / f"{cid}.png")
        r = lambda d, k: "({}, {}, {}, {})".format(*rects[(d, k)])
        lines += [f"        // {sheet}", "        (", f'            source: "clair_obscur/{cid}.png",', "            sprites: {", f'                "{cid}": {{']
        for d in ROWS:
            lines.append(f'                    "idle_{d}": (frames: [{r(d, 0)}], pivot: (0, -1), align: Bottom),')
            lines.append(f'                    "walk_{d}": (frames: [{r(d, 1)}, {r(d, 0)}, {r(d, 2)}, {r(d, 0)}], fps: 8, pivot: (0, -1), align: Bottom),')
        lines += ["                },", "            },", "        ),"]
        print(cid, "hauteur", height, "largeur de face", fr["down"][0].shape[1])
    lines += ["    ],", ")", ""]
    OUT_RON.write_text("\n".join(lines), encoding="utf-8", newline="\n")


if __name__ == "__main__":
    main(sys.argv[1])

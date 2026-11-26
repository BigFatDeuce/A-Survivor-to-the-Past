"""Dessine les poses de Link (research/zspr_poses.json) avec les tuiles et une palette d'un .zspr.

Usage (apercu) : python research/tools/zspr_render.py <dossier des .zspr> <sortie.png>
Planche d'apercu : chaque sprite au repos de face, de profil et de dos, avec son nom.
"""
import json
import sys
from pathlib import Path

from PIL import Image, ImageDraw

sys.path.insert(0, str(Path(__file__).parent))
import zspr  # noqa: E402

POSES = json.loads((Path(__file__).resolve().parents[1] / "zspr_poses.json").read_text(encoding="utf-8"))


def frame(rows, pal, pose):
    """Une image : les blocs dans l'ordre inverse du choix (le premier choisi est dessine dessus)."""
    im = Image.new("RGBA", (pose["w"], pose["h"]), (0, 0, 0, 0))
    px = im.load()
    for p in reversed(pose["pieces"]):
        s = p["size"]
        for j in range(s):
            for i in range(s):
                si = s - 1 - i if p["flip"] else i
                v = rows[p["sy"] + j][p["sx"] + si]
                x, y = p["dx"] + i, p["dy"] + j
                if v and 0 <= x < pose["w"] and 0 <= y < pose["h"]:
                    px[x, y] = pal[v] + (255,)
    return im


def frames(z, palette=0):
    rows = zspr.indices(z["gfx"])
    pal = zspr.palettes(z["pal"])[palette]
    return {anim: [frame(rows, pal, p) for p in poses] for anim, poses in POSES.items()}


def preview(folder, out):
    # fichiers manquants sur le site : page d'erreur HTML au lieu d'un .zspr
    files = [f for f in sorted(Path(folder).glob("*.zspr")) if f.read_bytes()[:4] == b"ZSPR"]
    cols = 12
    cw, ch = 64, 40
    sheet = Image.new("RGBA", (cols * cw, ((len(files) + cols - 1) // cols) * ch), (40, 40, 60, 255))
    d = ImageDraw.Draw(sheet)
    for k, f in enumerate(files):
        z = zspr.read(f)
        fr = frames(z)
        x, y = (k % cols) * cw, (k // cols) * ch
        for j, a in enumerate(("idle_down", "idle_side", "idle_up")):
            im = fr[a][0]
            sheet.alpha_composite(im, (x + 2 + j * 20, y + 2))
        d.text((x + 1, y + 28), z["name"][:12], fill=(230, 230, 230))
    sheet.save(out)
    print(f"{len(files)} sprites -> {out}")


if __name__ == "__main__":
    preview(sys.argv[1], sys.argv[2])

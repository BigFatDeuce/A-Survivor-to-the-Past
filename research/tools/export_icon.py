"""Icone officielle du jeu (proposition B « Link et la horde », choisie le 24/11/2026).

Sorties dans assets/icons/ :
  icon_<taille>.png  16 (lisse), 32 (natif), puis multiples entiers sans flou : 64 128 192 256 512
  icon.ico           16 32 48 64 128 256 (exe Windows)
  favicon.ico        16 32 48 (site, navigateur)
  icon_full_<taille>.png  sans coins ni liseré, fond jusqu'au bord : iOS (apple-touch-icon) et
                          Android (icone adaptative : le lanceur decoupe sa forme ; une icone a
                          coins transparents y est posee sur un fond blanc)
Usage : python research/tools/export_icon.py
"""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import make_icons as mk  # noqa: E402
from PIL import Image  # noqa: E402

OUT = mk.ROOT / "assets/icons"


def sized(im, s):
    if s % mk.N == 0:
        return im.resize((s, s), Image.NEAREST)
    return im.resize((s, s), Image.LANCZOS)


def full(n=mk.N):
    """Icone pleine sur n x n (n >= 32) : l'icone de 32 sans coins ni liseré, au centre d'un fond
    prolonge. Android : n = 48 (zone visible 72 dp sur 108, le contenu reste dans le cercle sur)."""
    im = mk.gradient("#5aa850", "#0e2414", n)
    o = (n - mk.N) // 2
    im.alpha_composite(mk.icon_b(rounded=False), (o, o))
    return im


def main():
    for s in (180, 192):
        full().resize((s, s), Image.NEAREST if s % mk.N == 0 else Image.LANCZOS).save(OUT / f"icon_full_{s}.png")
    full(48).resize((432, 432), Image.NEAREST).save(OUT / "icon_adaptive_432.png")
    icon = mk.finish(mk.icon_b())
    OUT.mkdir(parents=True, exist_ok=True)
    for s in (16, 32, 64, 128, 192, 256, 512):
        sized(icon, s).save(OUT / f"icon_{s}.png")
    # chaque taille est fournie telle quelle (pas de reechantillonnage par l'encodeur ICO)
    for name, sizes in (("icon.ico", (16, 32, 48, 64, 128, 256)), ("favicon.ico", (16, 32, 48))):
        imgs = [sized(icon, s) for s in sizes]
        imgs[-1].save(OUT / name, sizes=[(s, s) for s in sizes], append_images=imgs[:-1])
    print("ok :", OUT)


if __name__ == "__main__":
    main()

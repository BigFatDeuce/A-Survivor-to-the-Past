"""Icones des armes de la 0.6.2 : ecrit assets/source/Derived - items_v062.png et data/sprites/v062_items.ron.
Usage : python gen_items_v062.py <racine du projet> [apercu.png]

Planche « Items (The Minish Cap-Style) » de ChaosMiles07 (et contributeurs nommes sur la planche).
Chaque case de 16x16 (grille de 17 px) a son propre fond : il est retire par remplissage depuis les bords.
Lanterne bleue et jarre de Vaati : recolorations des icones d'origine (teinte decalee)."""
import colorsys
import os
import sys
from PIL import Image

ROOT = sys.argv[1]
SRC = os.path.join(ROOT, 'assets', 'source')
SHEET = 'Custom _ Edited - The Legend of Zelda Customs - Items - Items (The Minish Cap-Style).png'

# nom de l'icone -> (colonne, ligne) dans la grille, decalage de teinte (degres) ou None
CELLS = [
    ('icon_slingshot', (15, 29), None),
    ('icon_hyper_slingshot', (7, 48), None),
    ('icon_roc_feather', (1, 48), None),
    ('icon_roc_cape', (2, 48), None),
    ('icon_lantern', (3, 85), None),
    ('icon_blue_lantern', (3, 85), 190),
    ('icon_gust_jar', (7, 79), None),
    ('icon_vaati_jar', (7, 79), 75),
    ('icon_ocarina', (16, 29), None),
    ('icon_ocarina_of_time', (16, 30), None),
    ('icon_din_fire', (19, 27), None),
    ('icon_din_wrath', (15, 28), None),
    ('icon_lens_of_truth', (11, 31), None),
    ('icon_kinstone', (26, 81), None),
]
SEED = (8, 47)


def grid_line(im, x, y0):
    """La colonne x est-elle une ligne de grille : meme couleur sur toute la hauteur de la case,
    differente du fond de la case juste a droite."""
    col = {im.getpixel((x, y))[:3] for y in range(y0, y0 + 16)}
    return len(col) == 1 and im.getpixel((x, y0))[:3] != im.getpixel((x + 1, y0))[:3]


def cell(im, k, j):
    """Case (k, j) sans son fond (pixels de la couleur du coin relies au bord).
    Dans le bas de la planche, la grille est decalee d'un pixel vers la gauche : on le detecte."""
    x0 = 17 * k + 1
    if not grid_line(im, x0 - 1, 17 * j + 1) and grid_line(im, x0 - 2, 17 * j + 1):
        x0 -= 1
    c = im.crop((x0, 17 * j + 1, x0 + 16, 17 * j + 17)).convert('RGBA')
    px = c.load()
    bg = px[0, 0][:3]
    todo = [(x, y) for x in range(16) for y in (0, 15)] + [(x, y) for y in range(16) for x in (0, 15)]
    seen = set()
    while todo:
        x, y = todo.pop()
        if (x, y) in seen or not (0 <= x < 16 and 0 <= y < 16):
            continue
        seen.add((x, y))
        if px[x, y][3] == 0 or px[x, y][:3] != bg:
            continue
        px[x, y] = (0, 0, 0, 0)
        todo += [(x + 1, y), (x - 1, y), (x, y + 1), (x, y - 1)]
    return c


def hue(c, deg):
    """Decale la teinte des pixels colores (le blanc et le noir restent)."""
    px = c.load()
    for y in range(c.height):
        for x in range(c.width):
            r, g, b, a = px[x, y]
            if not a:
                continue
            h, s, v = colorsys.rgb_to_hsv(r / 255, g / 255, b / 255)
            if s < 0.15:
                continue
            r, g, b = colorsys.hsv_to_rgb((h + deg / 360) % 1.0, s, v)
            px[x, y] = (round(r * 255), round(g * 255), round(b * 255), a)
    return c


def main():
    im = Image.open(os.path.join(SRC, SHEET)).convert('RGBA')
    out = Image.new('RGBA', (17 * (len(CELLS) + 1), 16), (0, 0, 0, 0))
    lines = []
    for n, (name, (k, j), shift) in enumerate(CELLS):
        c = cell(im, k, j)
        if shift:
            c = hue(c, shift)
        out.paste(c, (17 * n, 0))
        lines.append(f'                "{name}": {{ "0": (frames: [({17 * n}, 0, 16, 16)], pivot: (0, 0), align: Center) }},')
    # graine du lance-pierre : rognee a sa boite
    s = cell(im, *SEED)
    bb = s.getbbox()
    s = s.crop(bb)
    x0 = 17 * len(CELLS)
    out.paste(s, (x0, 0))
    lines.append(f'                "proj_seed": {{ "0": (frames: [({x0}, 0, {s.width}, {s.height})], pivot: (0, 0), align: Center) }},')
    out.save(os.path.join(SRC, 'Derived - items_v062.png'))
    ron = ['// Version 0.6.2 : icones des nouvelles armes et reliques. Genere par research/tools/gen_items_v062.py.',
           '// Planche « Items (The Minish Cap-Style) », ChaosMiles07 et contributeurs. Voir CREDITS.md.',
           '(', '    sheets: [', '        (', '            source: "Derived - items_v062.png",', '            sprites: {']
    ron += lines
    ron += ['            },', '        ),', '    ],', ')', '']
    with open(os.path.join(ROOT, 'data', 'sprites', 'v062_items.ron'), 'w', encoding='utf-8', newline='\n') as f:
        f.write('\n'.join(ron))
    if len(sys.argv) > 2:
        bg = Image.new('RGBA', out.size, (60, 60, 60, 255))
        bg.alpha_composite(out)
        bg.resize((out.width * 4, out.height * 4), Image.NEAREST).save(sys.argv[2])
    print('ok', bb)


main()

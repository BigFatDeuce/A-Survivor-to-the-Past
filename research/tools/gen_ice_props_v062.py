"""Decors du Lac gele (0.6.2) : ecrit assets/source/Derived - ice_props_v062.png et data/sprites/v062_ice_props.ron.
Usage : python gen_ice_props_v062.py <racine du projet> [apercu.png]

Tout vient du vrai Lac de Glace d'A Link to the Past (monde des tenebres, autour du Palais de Glace) :
blocs de glace flottants et pieux en bois decoupes dans le rendu de la carte (l'eau verte est retiree),
et le bloc du Palais de Glace de la planche des pieges (Barack Obama). Remplace arbres, souches et buissons."""
import os
import sys
from PIL import Image

ROOT = sys.argv[1]
SRC = os.path.join(ROOT, 'assets', 'source')
DW = 'SNES - The Legend of Zelda_ A Link to the Past - Overworld - Dark World.png'
TRAPS = 'SNES - The Legend of Zelda_ A Link to the Past - Enemies - Object Enemies & Traps.png'

# nom -> (zone de recherche dans la carte du monde des tenebres, point de depart dans l'objet)
FROM_MAP = [
    ('prop_ice_floe', (3176, 3416, 3264, 3484), (3208, 3456)),
    ('prop_ice_floe_big', (3432, 3928, 3496, 3990), (3464, 3958)),
    ('prop_ice_floe_small', (3546, 3980, 3590, 4020), (3568, 4000)),
    ('prop_stake', (2996, 3420, 3020, 3446), (3008, 3432)),
]
ICE_BLOCK = (336, 912, 16, 16)


def water(p):
    """Eau du lac : vert dominant."""
    r, g, b = p[:3]
    return g > r + 20 and g > b + 12


def cut(im, box, seed):
    """Objet relie au point de depart (tout ce qui n'est pas de l'eau), rogne a sa boite."""
    x0, y0, x1, y1 = box
    keep = set()
    todo = [seed]
    while todo:
        x, y = todo.pop()
        if (x, y) in keep or not (x0 <= x < x1 and y0 <= y < y1) or water(im.getpixel((x, y))):
            continue
        keep.add((x, y))
        todo += [(x + 1, y), (x - 1, y), (x, y + 1), (x, y - 1)]
    xs = [p[0] for p in keep]
    ys = [p[1] for p in keep]
    bx, by = min(xs), min(ys)
    out = Image.new('RGBA', (max(xs) - bx + 1, max(ys) - by + 1), (0, 0, 0, 0))
    for x, y in keep:
        out.putpixel((x - bx, y - by), im.getpixel((x, y)) + (255,))
    return out


def main():
    Image.MAX_IMAGE_PIXELS = None
    dw = Image.open(os.path.join(SRC, DW)).convert('RGB')
    traps = Image.open(os.path.join(SRC, TRAPS)).convert('RGB')
    parts = [(name, cut(dw, box, seed)) for name, box, seed in FROM_MAP]
    x, y, w, h = ICE_BLOCK
    parts.append(('prop_ice_block', traps.crop((x, y, x + w, y + h)).convert('RGBA')))
    W = sum(p.width + 1 for _, p in parts)
    H = max(p.height for _, p in parts)
    sheet = Image.new('RGBA', (W, H), (0, 0, 0, 0))
    lines = []
    x = 0
    for name, p in parts:
        sheet.paste(p, (x, 0))
        lines.append(f'                "{name}": {{ "0": (frames: [({x}, 0, {p.width}, {p.height})], pivot: (0, -2), align: Bottom) }},')
        x += p.width + 1
    sheet.save(os.path.join(SRC, 'Derived - ice_props_v062.png'))
    ron = ['// Version 0.6.2 : decors du Lac gele, tires du vrai Lac de Glace (monde des tenebres) et du bloc du',
           '// Palais de Glace. Genere par research/tools/gen_ice_props_v062.py. Voir CREDITS.md.',
           '(', '    sheets: [', '        (', '            source: "Derived - ice_props_v062.png",', '            sprites: {']
    ron += lines + ['            },', '        ),', '    ],', ')', '']
    with open(os.path.join(ROOT, 'data', 'sprites', 'v062_ice_props.ron'), 'w', encoding='utf-8', newline='\n') as f:
        f.write('\n'.join(ron))
    if len(sys.argv) > 2:
        bg = Image.new('RGBA', sheet.size, (190, 215, 235, 255))
        bg.alpha_composite(sheet)
        bg.resize((sheet.width * 4, sheet.height * 4), Image.NEAREST).save(sys.argv[2])
    print('ok', [(n, p.size) for n, p in parts])


main()

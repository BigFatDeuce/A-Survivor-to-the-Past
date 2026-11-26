"""Decoupe les personnages de la 0.6.2 : ecrit data/sprites/v062_chars.ron, les planches derivees et un apercu.
Usage : python gen_chars_v062.py <racine du projet> <dossier scratchpad>

Sheik : planche d'Oldskoolmario completee (dos et profil dessines pour le jeu, meme palette).
Mario (facon Link's Awakening, AquariusAlexx) : marche avec le bouclier.
Link de Link's Awakening (Super Mario Bros. Crossover) : deuxieme apparence du Link de Link's Awakening.
Skull Kid (facon A Link to the Past, Bad Ass Bill), Simon Belmont (Konami Wai Wai Sokoban),
Ganondorf (facon Wind Waker / Minish Cap, bucketboy), Vivi (Final Fantasy Record Keeper)."""
import os
import sys
from PIL import Image, ImageOps

ROOT, SCR = sys.argv[1], sys.argv[2]
SRC = os.path.join(ROOT, 'assets', 'source')


def rgb(h):
    return tuple(int(h.lstrip('#')[i:i + 2], 16) for i in (0, 2, 4))


def load(name, bgs):
    """Planche en RGBA, fonds rendus transparents."""
    im = Image.open(os.path.join(SRC, name)).convert('RGBA')
    px = im.load()
    for y in range(im.height):
        for x in range(im.width):
            if px[x, y][3] and px[x, y][:3] in bgs:
                px[x, y] = (0, 0, 0, 0)
    return im


def box(im, x0, y0, x1, y1):
    px = im.load()
    xs, ys = [], []
    for y in range(y0, y1):
        for x in range(x0, x1):
            if px[x, y][3] > 0:
                xs.append(x)
                ys.append(y)
    return (min(xs), min(ys), max(xs), max(ys))


def cells(im, zones):
    """Images de meme taille, alignees en bas et centrees."""
    boxes = [box(im, *z) for z in zones]
    H = max(d - b + 1 for _, b, _, d in boxes)
    W = max(c - a + 1 for a, _, c, _ in boxes)
    W += W % 2
    out = []
    for (a, b, c, d) in boxes:
        piece = im.crop((a, b, c + 1, d + 1))
        cell = Image.new('RGBA', (W, H), (0, 0, 0, 0))
        cell.alpha_composite(piece, ((W - piece.width) // 2, H - piece.height))
        out.append(cell)
    return out


def at(xs, y, w, h):
    return [(x, y, x + w, y + h) for x in xs]


def mirror(c):
    return ImageOps.mirror(c)


def strip_shadow(im, zone, dark):
    """Retire l'ombre dessinee sous les pieds : garde une ligne de contour sous le dernier pixel colore."""
    x0, y0, x1, y1 = zone
    px = im.load()
    last = y0
    for y in range(y0, y1):
        if any(px[x, y][3] and px[x, y][:3] not in dark for x in range(x0, x1)):
            last = y
    for y in range(last + 2, y1):
        for x in range(x0, x1):
            px[x, y] = (0, 0, 0, 0)


def pack(name, rows):
    order = [k for k in ('down', 'side', 'up') if k in rows]
    uniq = []
    for k in order:
        for c in rows[k]:
            if not any(c is u for u in uniq):
                uniq.append(c)
    W = sum(c.width + 1 for c in uniq)
    H = max(c.height for c in uniq)
    sheet = Image.new('RGBA', (W, H), (0, 0, 0, 0))
    pos = {}
    x = 0
    for c in uniq:
        sheet.alpha_composite(c, (x, H - c.height))
        pos[id(c)] = (x, H - c.height, c.width, c.height)
        x += c.width + 1
    fname = f'Derived - {name}.png'
    sheet.save(os.path.join(SRC, fname))
    return fname, {k: [pos[id(c)] for c in v] for k, v in rows.items()}


def frames(cs):
    return '[' + ', '.join(f'({x}, {y}, {w}, {h})' for x, y, w, h in cs) + ']'


def sprite_block(name, rows, fps):
    lines = [f'                "{name}": {{']
    for d in ('down', 'side', 'up'):
        cs = rows[d]
        lines.append(f'                    "idle_{d}": (frames: {frames(cs[:1])}, pivot: (0, -1), align: Bottom),')
        lines.append(f'                    "walk_{d}": (frames: {frames(cs)}, fps: {fps}, pivot: (0, -1), align: Bottom),')
    lines.append('                },')
    return '\n'.join(lines)


previews = []
sheets = []


def add(name, rows, fps):
    strip = rows['down'] + rows['side'] + rows['up']
    previews.append((name, strip))
    f, cs = pack(name, rows)
    sheets.append((f, sprite_block(name, cs, fps)))


# --- Sheik : face de la planche (ombre retiree), dos et profil ajoutes dans la bande du bas ---
SHEIK_BG = [(123, 138, 74), (57, 89, 57)]
im = load('Custom - Sheik (Oldskoolmario, back and side added).png', SHEIK_BG)
DOWN = {k: (x, 37, x + 16, 61) for k, x in (('f1', 25), ('f3', 77), ('f4', 101))}
for z in DOWN.values():
    strip_shadow(im, z, [(32, 32, 32)])
f1, f3, f4 = cells(im, [DOWN['f1'], DOWN['f3'], DOWN['f4']])
b3, b1, b4 = cells(im, at([18, 37, 56], 129, 16, 24))
sa, sb, sc = cells(im, at([81, 100, 119], 129, 16, 24))
add('sheik', {'down': [f3, f1, f4, f1], 'up': [b3, b1, b4, b1], 'side': [sa, sb, sc, sb]}, 8)

# --- Mario (facon Link's Awakening) : marche avec le bouclier, profil regarde a droite ---
im = load("Custom - Mario (Link's Awakening-Style).png", [(114, 133, 0), (68, 80, 0)])
d1, d2, u1, u2, s1, s2 = cells(im, at([1, 18, 35, 52, 69, 86], 42, 16, 16))
add('mario', {'down': [d1, d2], 'up': [u1, u2], 'side': [s1, s2]}, 6)

# --- Link de Link's Awakening (Super Mario Bros. Crossover) : apparence Game Boy, profil regarde a gauche ---
im = load("Browser Games - Super Mario Bros. Crossover - Link Skins - Link - Link's Awakening.png", [(111, 49, 152)])
d1, d2 = cells(im, [(53, 12, 66, 28), (54, 36, 67, 52)])
u1, u2 = cells(im, [(78, 12, 90, 28), (78, 36, 90, 52)])
s1, s2 = cells(im, [(5, 12, 19, 28), (29, 13, 42, 28)])
add('link_awakening_gb', {'down': [d1, d2], 'up': [u1, u2], 'side': [s1, s2]}, 6)

# --- Skull Kid (facon A Link to the Past) : 10 images de face et de dos, 8 de profil (regarde a gauche) ---
im = load('Custom - Skull Kid (A Link to the Past-Style).png', [(255, 255, 255)])
FX = [32, 51, 70, 89, 108, 127, 146, 165, 184, 203]
add('skull_kid', {
    'down': cells(im, at(FX, 142, 18, 31)),
    'side': cells(im, at([1, 17, 32, 47, 62, 77, 92, 107], 172, 15, 30)),
    'up': cells(im, at(FX, 202, 18, 30)),
}, 12)

# --- Simon Belmont (Konami Wai Wai Sokoban) : ombre retiree (gris et noir ne servent qu'a l'ombre sur cette
# planche), profil regarde a gauche ---
im = load('Mobile - Konami Wai Wai Sokoban - Playable Characters - Simon Belmont.png', [(26, 26, 26), (0, 0, 0)])
d1, d2, u1, u2 = cells(im, at([2, 17, 33, 50], 21, 13, 19))
side = cells(im, at([2, 18, 34, 50], 41, 12, 19))
add('simon', {'down': [d1, d2], 'up': [u1, u2], 'side': side}, 8)

# --- Ganondorf (facon Wind Waker / Minish Cap) : 8 images par direction, profil regarde a gauche ---
im = load('Custom - Ganondorf (Wind Waker, Minish Cap-Style).png', [(255, 255, 255)])
SX = [(1, 17), (19, 17), (37, 18), (56, 18), (75, 17), (93, 18), (112, 18), (131, 18)]
add('ganondorf', {
    'down': cells(im, at([1 + 25 * k for k in range(8)], 1, 24, 26)),
    'side': cells(im, [(x, 332, x + w, 360) for x, w in SX]),
    'up': cells(im, at([292 + 25 * k for k in range(8)], 1, 24, 27)),
}, 12)

# --- Vivi (Final Fantasy Record Keeper) : planche compacte, profil regarde a gauche ---
im = load('Mobile - Final Fantasy_ Record Keeper - Playable Characters_ Final Fantasy IX - Vivi.png', [])
d1, d2 = cells(im, at([93, 110], 26, 16, 24))
u1, u2 = cells(im, at([18, 35], 43, 16, 24))
# profil : le cycle de marche (5 images, seules les jambes bougent ; retour 0.6.2 : les deux poses
# de combat prises avant donnaient l'impression qu'il glissait)
side = cells(im, [(18, 93, 34, 117), (35, 118, 51, 141), (52, 118, 68, 141), (69, 118, 85, 141), (86, 126, 102, 149)])
add('vivi', {'down': [d1, d2], 'up': [u1, u2], 'side': side}, 7)

out = ['// Version 0.6.2 : personnages ajoutes. Genere par research/tools/gen_chars_v062.py.',
       '// Credits : Sheik (Oldskoolmario, dos et profil ajoutes), Mario facon Link\'s Awakening (AquariusAlexx),',
       '// Link de Super Mario Bros. Crossover, Skull Kid (Bad Ass Bill), Simon Belmont (Konami Wai Wai Sokoban),',
       '// Ganondorf (bucketboy), Vivi (Final Fantasy Record Keeper). Voir CREDITS.md.',
       '(', '    sheets: [']
for fname, block in sheets:
    out += ['        (', f'            source: "{fname}",', '            sprites: {', block, '            },', '        ),']
out += ['    ],', ')', '']
open(os.path.join(ROOT, 'data', 'sprites', 'v062_chars.ron'), 'w', encoding='utf-8', newline='\n').write('\n'.join(out))

rowsimg = []
for label, strip in previews:
    w = sum(s.width + 2 for s in strip)
    h = max(s.height for s in strip)
    r = Image.new('RGBA', (w, h), (72, 152, 72, 255))
    x = 0
    for s in strip:
        r.alpha_composite(s, (x, h - s.height))
        x += s.width + 2
    rowsimg.append(r)
W = max(r.width for r in rowsimg)
H = sum(r.height + 4 for r in rowsimg)
o = Image.new('RGBA', (W, H), (30, 30, 30, 255))
y = 0
for r in rowsimg:
    o.paste(r, (0, y))
    y += r.height + 4
o.resize((o.width * 3, o.height * 3), Image.NEAREST).save(os.path.join(SCR, 'chars062_preview.png'))
print('ok')

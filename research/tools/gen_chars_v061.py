"""Decoupe les personnages de la 0.6.1 : ecrit data/sprites/v061_chars.ron, les planches derivees et un apercu.
Usage : python gen_chars_v061.py <racine du projet> <dossier scratchpad>"""
import os
import sys
from PIL import Image, ImageOps

ROOT, SCR = sys.argv[1], sys.argv[2]
SRC = os.path.join(ROOT, 'assets', 'source')


def rgb(h):
    return tuple(int(h.lstrip('#')[i:i + 2], 16) for i in (0, 2, 4))


def solid_box(im, x0, y0, x1, y1, bgs):
    """Boite englobante des pixels non transparents dans une zone."""
    px = im.load()
    xs, ys = [], []
    for y in range(y0, y1):
        for x in range(x0, x1):
            p = px[x, y]
            if p[3] > 0 and p[:3] not in bgs:
                xs.append(x)
                ys.append(y)
    return (min(xs), min(ys), max(xs), max(ys))


def cells(im, zones, bgs, align):
    """Images detourees (fond transparent), toutes de meme taille, alignees en bas et au centre ou sur un bord."""
    boxes = [solid_box(im, *z, bgs) for z in zones]
    bottom = max(d for _, _, _, d in boxes)
    top = min(b for _, b, _, _ in boxes)
    H = bottom - top + 1
    W = max(c - a + 1 for a, _, c, _ in boxes)
    W += W % 2
    out = []
    for (a, b, c, d) in boxes:
        piece = im.crop((a, b, c + 1, d + 1)).convert('RGBA')
        px = piece.load()
        for yy in range(piece.height):
            for xx in range(piece.width):
                if px[xx, yy][:3] in bgs:
                    px[xx, yy] = (0, 0, 0, 0)
        if align == 'left':
            x = 0
        elif align == 'right':
            x = W - piece.width
        else:
            x = (W - piece.width) // 2
        cell = Image.new('RGBA', (W, H), (0, 0, 0, 0))
        cell.alpha_composite(piece, (x, (b - top)))
        out.append(cell)
    return out


def mirror(c):
    return ImageOps.mirror(c)


def pack(name, rows):
    """Range les images dans une planche derivee ; renvoie le nom du fichier et les cases."""
    order = [k for k in ('down', 'side', 'up', 'walk_down', 'walk_side', 'walk_up') if k in rows]
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
        walk = rows.get('walk_' + d, cs)
        lines.append(f'                    "walk_{d}": (frames: {frames(walk)}, fps: {fps}, pivot: (0, -1), align: Bottom),')
    lines.append('                },')
    return '\n'.join(lines)


previews = []


def preview(rows, label):
    strip = []
    for d in ('down', 'side', 'up'):
        strip.extend(rows.get('walk_' + d, rows[d]))
    previews.append((label, strip))


sheets = []

# --- Toon Link (facon Mario & Luigi Superstar Saga) : 8 images par direction, profil regarde a gauche ---
im = Image.open(os.path.join(SRC, 'Custom - Toon Link (Mario & Luigi Superstar Saga-Style).png')).convert('RGBA')
bgs = [rgb('#b8b088')]
TX = [5, 33, 61, 89, 117, 145, 173, 201]
SX = [(7, 28), (34, 56), (60, 84), (91, 113), (119, 140), (147, 169), (173, 197), (203, 225)]
rows = {
    'down': cells(im, [(x, 24, x + 24, 57) for x in TX], bgs, 'center'),
    'up': cells(im, [(x, 67, x + 24, 100) for x in TX], bgs, 'center'),
    'side': cells(im, [(a, 112, b + 1, 145) for a, b in SX], bgs, 'center'),
}
preview(rows, 'toon_link')
f, cs = pack('toon_link', rows)
sheets.append((f, [sprite_block('toon_link', cs, 12)]))

# --- Link (Oracle of Ages, Game Boy Color) : une image par direction, la marche alterne avec son miroir ---
im = Image.open(os.path.join(SRC, 'Game Boy _ GBC - The Legend of Zelda_ Oracle of Ages - Playable Characters - Link.png')).convert('RGBA')
bgs = [rgb('#ffffff'), rgb('#008000')]
down, up, s1, s2 = cells(im, [(x, 38, x + 16, 54) for x in (895, 912, 929, 946)], bgs, 'center')
rows = {
    'down': [down, mirror(down)],
    'up': [up, mirror(up)],
    'side': [s1, s2],
}
preview(rows, 'link_ooa')
f, cs = pack('link_ooa', rows)
sheets.append((f, [sprite_block('link_ooa', cs, 6)]))

# --- Zora des mers (style A Link to the Past) : 2 images par direction, profil regarde a gauche ---
im = Image.open(os.path.join(SRC, 'Custom - Sea Zora (A Link to the Past-Style).png')).convert('RGBA')
bgs = []
rows = {
    'down': cells(im, [(1, 0, 21, 23), (24, 0, 44, 23)], bgs, 'center'),
    'side': cells(im, [(50, 0, 71, 23), (75, 0, 96, 23)], bgs, 'center'),
    'up': cells(im, [(102, 0, 117, 23), (123, 0, 138, 23)], bgs, 'center'),
}
preview(rows, 'sea_zora')
f, cs = pack('sea_zora', rows)
sheets.append((f, [sprite_block('sea_zora', cs, 6)]))

# --- Crewmate (Among Us, style A Link to the Past) : 8 images par direction, profil regarde a droite ;
# couleurs du jeu d'origine par echange de palette (corps, ombre du corps) ---
im = Image.open(os.path.join(SRC, 'Custom - Among Us Crewmate (A Link to the Past-Style).png')).convert('RGBA')
bgs = [rgb('#658dd1')]
SIDE = [(3, 14), (17, 28), (31, 43), (46, 58), (61, 72), (75, 86), (89, 103), (106, 118)]
UP = [(2, 14), (17, 29), (32, 44), (47, 59), (62, 74), (77, 89), (92, 104), (107, 119)]
DOWN = [(3, 13), (16, 26), (29, 39), (42, 54), (57, 67), (70, 80), (83, 93), (96, 108)]
base = {
    'side': cells(im, [(a, 17, b + 1, 36) for a, b in SIDE], bgs, 'center'),
    'up': cells(im, [(a, 36, b + 1, 57) for a, b in UP], bgs, 'center'),
    'down': cells(im, [(a, 57, b + 1, 77) for a, b in DOWN], bgs, 'center'),
}
BODY, SHADE = (184, 16, 32), (149, 22, 34)
CREW = {
    'red': None,
    'blue': ('#132ed1', '#09158e'),
    'green': ('#117f2d', '#0a4d2e'),
    'pink': ('#ed54ba', '#ab2bad'),
    'orange': ('#ef7d0d', '#b33e15'),
    'yellow': ('#f5f557', '#c38823'),
    'purple': ('#6b2fbb', '#3b177c'),
    'cyan': ('#38fedc', '#24a8be'),
}


def recolor(c, body, shade):
    c = c.copy()
    px = c.load()
    for y in range(c.height):
        for x in range(c.width):
            p = px[x, y]
            if p[3] and p[:3] == BODY:
                px[x, y] = body + (p[3],)
            elif p[3] and p[:3] == SHADE:
                px[x, y] = shade + (p[3],)
    return c


for color, pal in CREW.items():
    if pal is None:
        rows = base
    else:
        b, s = rgb(pal[0]), rgb(pal[1])
        rows = {k: [recolor(c, b, s) for c in v] for k, v in base.items()}
    name = f'crewmate_{color}'
    preview(rows, name)
    f, cs = pack(name, rows)
    sheets.append((f, [sprite_block(name, cs, 12)]))

out = ['// Version 0.6.1 : personnages ajoutes. Genere par research/tools/gen_chars_v061.py.',
       '// Credits : Toon Link facon Mario & Luigi (Oldskoolmario), Link d\'Oracle of Ages (rip de Mister Mike),',
       '// Zora des mers (Toni), Crewmate facon A Link to the Past (Spikira). Voir CREDITS.md.',
       '(', '    sheets: [']
for fname, blocks in sheets:
    out.append('        (')
    out.append(f'            source: "{fname}",')
    out.append('            sprites: {')
    out.extend(blocks)
    out.append('            },')
    out.append('        ),')
out += ['    ],', ')', '']
open(os.path.join(ROOT, 'data', 'sprites', 'v061_chars.ron'), 'w', encoding='utf-8', newline='\n').write('\n'.join(out))

rowsimg = []
for label, strip in previews:
    w = sum(s.width + 2 for s in strip)
    h = max(s.height for s in strip)
    r = Image.new('RGBA', (w, h), (60, 60, 70, 255))
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
o.resize((o.width * 3, o.height * 3), Image.NEAREST).save(os.path.join(SCR, 'chars061_preview.png'))
print('ok')

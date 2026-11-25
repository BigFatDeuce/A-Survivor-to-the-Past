"""Decoupe les personnages de la 0.5 (suite) : ecrit data/sprites/v05_more_chars.ron et un apercu des cadres.
Usage : python gen_chars.py <racine du projet> <dossier scratchpad>"""
import os
import sys
from PIL import Image

ROOT, SCR = sys.argv[1], sys.argv[2]
SRC = os.path.join(ROOT, 'assets', 'source')


def rgb(h):
    return tuple(int(h.lstrip('#')[i:i + 2], 16) for i in (0, 2, 4))


def solid_box(im, x0, y0, x1, y1, bgs):
    """Boite englobante des pixels non transparents dans une zone."""
    px = im.load()
    a = b = None
    xs, ys = [], []
    for y in range(y0, y1):
        for x in range(x0, x1):
            p = px[x, y]
            if p[3] > 0 and p[:3] not in bgs:
                xs.append(x)
                ys.append(y)
    return (min(xs), min(ys), max(xs), max(ys))


def cells(im, zones, bgs, align, cut=0):
    """zones : zones (x0, y0, x1, y1) contenant chacune une image. Renvoie les images detourees (fond
    transparent), toutes de meme taille : alignees en bas (la marche garde ses petits sauts) et au centre ou
    sur le bord avant (profil)."""
    boxes = [solid_box(im, *z, bgs) for z in zones]
    boxes = [(a, b, c, d - cut) for a, b, c, d in boxes]
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


def pack(name, rows):
    """Range les images dans une planche derivee (une rangee par direction) ; renvoie les cases."""
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
    """rows : dict anim -> cases ; idle = premiere case de chaque direction."""
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

# --- Link (Four Swords Adventures) : 4 couleurs + Shadow Link, meme disposition ---
FSA = {
    'green': ('GameCube - The Legend of Zelda_ Four Swords Adventures - Playable Characters - Link (Green) (GCN Palette) (1).png', '#74E496'),
    'red': ('GameCube - The Legend of Zelda_ Four Swords Adventures - Playable Characters - Link (Red) (GCN Palette).png', '#FFB7B9'),
    'blue': ('GameCube - The Legend of Zelda_ Four Swords Adventures - Playable Characters - Link (Blue) (GCN Palette) (1).png', '#BCE7F1'),
    'purple': ('GameCube - The Legend of Zelda_ Four Swords Adventures - Playable Characters - Link (Purple) (GCN Palette) (1).png', '#EBC5FC'),
    'shadow': ('GameCube - The Legend of Zelda_ Four Swords Adventures - Enemies & Bosses - Shadow Link (Itemless) (GCN Colours).png', '#B8B8B8'),
}
# images de la marche : rangees haut (dos), profil (regarde a gauche), face ; 6 images chacune
XS = [(2, 34), (34, 66), (66, 99), (99, 131), (131, 165), (165, 205)]
fsa_rows = None
for color, (fname, bg) in FSA.items():
    im = Image.open(os.path.join(SRC, fname)).convert('RGBA')
    bgs = [rgb(bg)]
    rows = {
        'up': cells(im, [(a, 181, b, 213) for a, b in XS], bgs, 'center'),
        'side': cells(im, [(a, 214, b, 241) for a, b in XS], bgs, 'left'),
        'down': cells(im, [(a, 242, b, 276) for a, b in XS], bgs, 'center'),
    }
    name = 'shadow_link' if color == 'shadow' else f'link_fsa_{color}'
    preview(rows, name)
    f, cs = pack(name, rows)
    sheets.append((f, [], [sprite_block(name, cs, 10)]))

# --- Deku Link (style A Link to the Past), planche convertie en PNG ---
deku = 'Custom - Deku Link (A Link to the Past-Style).png'
im = Image.open(os.path.join(SRC, deku)).convert('RGBA')
bgs = [rgb('#C0C0C0'), rgb('#FFFFFF')]
DX = [(0, 21), (21, 41), (41, 61), (61, 81), (81, 101)]
rows = {
    'down': cells(im, [(a, 1, b, 22) for a, b in DX], bgs, 'center'),
    'up': cells(im, [(a, 22, b, 43) for a, b in DX], bgs, 'center'),
    'side': cells(im, [(a, 44, b, 64) for a, b in DX], bgs, 'left'),
}
for d in ('down', 'up', 'side'):
    rows['walk_' + d] = rows[d][1:]
preview(rows, 'deku_link')
f, cs = pack('deku_link', rows)
sheets.append((f, [], [sprite_block('deku_link', cs, 8)]))

# --- Shy Guy (style A Link to the Past) ---
shy = 'Custom - Shy Guy (A Link to the Past-Style).png'
im = Image.open(os.path.join(SRC, shy)).convert('RGBA')
bgs = [rgb('#808080')]
rows = {
    'down': cells(im, [(72, 5, 93, 30), (96, 5, 117, 30)], bgs, 'center'),
    'side': cells(im, [(72, 31, 94, 53), (96, 31, 118, 53)], bgs, 'right'),
    'up': cells(im, [(72, 53, 94, 77), (96, 53, 118, 77)], bgs, 'center'),
}
preview(rows, 'shy_guy')
f, cs = pack('shy_guy', rows)
sheets.append((f, [], [sprite_block('shy_guy', cs, 6)]))

# --- Link adulte (style Chrono Trigger) : ombre au sol retiree (2 dernieres lignes) ---
cht = 'Custom - Link (Chrono Trigger-Style).png'
im = Image.open(os.path.join(SRC, cht)).convert('RGBA')
bgs = [rgb('#395939'), rgb('#7B8A4A')]
CX = [(2, 31), (31, 59), (59, 84), (84, 110), (110, 137), (137, 163), (163, 193)]
SX = [(2, 31), (31, 58), (58, 90), (90, 127), (127, 156), (156, 191), (191, 228)]
rows = {
    'down': cells(im, [(a, 24, b, 61) for a, b in CX], bgs, 'center', cut=2),
    'up': cells(im, [(a, 63, b, 99) for a, b in CX], bgs, 'center', cut=2),
    'side': cells(im, [(a, 101, b, 138) for a, b in SX], bgs, 'right', cut=2),
}
for d in ('down', 'up'):
    rows['walk_' + d] = rows[d][1:]
rows['walk_side'] = rows['side'][2:]
preview(rows, 'link_adult')
f, cs = pack('link_adult', rows)
sheets.append((f, [], [sprite_block('link_adult', cs, 10)]))

# --- Link (Zelda II) : profil seulement (jeu de plateforme) ---
z2 = 'NES - Zelda 2_ The Adventure of Link - Playable Characters - Link.png'
im = Image.open(os.path.join(SRC, z2)).convert('RGBA')
bgs = [rgb('#008000'), rgb('#800080')]
side = cells(im, [(1, 11, 17, 43), (29, 11, 45, 43), (46, 11, 62, 43), (63, 11, 79, 43)], bgs, 'right')
rows = {'down': side, 'side': side, 'up': side, 'walk_down': side[1:], 'walk_side': side[1:], 'walk_up': side[1:]}
preview(rows, 'link_zelda2')
f, cs = pack('link_zelda2', rows)
sheets.append((f, [], [sprite_block('link_zelda2', cs, 8)]))

# --- Link (Link's Awakening DX) : planche derivee (images retournees pour la marche) ---
la = "Custom - Link's Awakening DX Link (walk).png"
im = Image.open(os.path.join(SRC, la)).convert('RGBA')
bgs = []
rows = {
    'down': cells(im, [(0, 0, 16, 16), (16, 0, 32, 16)], bgs, 'center'),
    'up': cells(im, [(32, 0, 48, 16), (48, 0, 64, 16)], bgs, 'center'),
    'side': cells(im, [(64, 0, 80, 16), (80, 0, 96, 16)], bgs, 'left'),
}
preview(rows, 'link_awakening')
f, cs = pack('link_awakening', rows)
sheets.append((f, [], [sprite_block('link_awakening', cs, 6)]))

out = ['// Version 0.5 (suite) : personnages de AdditionalChars et Shadow Link (MoreSprites). Genere par',
       '// research/tools/gen_chars.py. Credits : Four Swords Adventures (rip de Vaati), Deku Link (amh1010),',
       '// Shy Guy (yesilovenachos), Link adulte facon Chrono Trigger (Oldskoolmario, Redfusion), Zelda II et',
       "// Link's Awakening (Mister Mike).",
       '(', '    sheets: [']
for fname, tr, blocks in sheets:
    out.append('        (')
    out.append(f'            source: "{fname}",')
    if tr:
        out.append('            transparent: [' + ', '.join(f'"{t}"' for t in tr) + '],')
    out.append('            sprites: {')
    out.extend(blocks)
    out.append('            },')
    out.append('        ),')
out += ['    ],', ')', '']
open(os.path.join(ROOT, 'data', 'sprites', 'v05_more_chars.ron'), 'w', encoding='utf-8', newline='\n').write('\n'.join(out))

# apercu : chaque rangee = un personnage, images en x3
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
o.resize((o.width * 3, o.height * 3), Image.NEAREST).save(os.path.join(SCR, 'chars_preview.png'))
print('ok')

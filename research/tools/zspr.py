"""Lecture des fichiers .zspr (sprites de Link du A Link to the Past Randomizer).

Format : en-tete ZSPR (magie, version, somme, decalages et longueurs des graphismes et des
palettes, type), noms UTF-16 (sprite, auteur), puis 0x7000 octets de tuiles SNES 4 bits par pixel
(896 tuiles 8x8, rangees par 16 : planche de 128 x 448) et 4 palettes de 15 couleurs BGR555
(cottes de mailles verte, bleue, rouge, lapin) + 2 couleurs de gants.
"""
import struct

from PIL import Image

TILES_W = 16


def read(path):
    b = open(path, "rb").read()
    assert b[:4] == b"ZSPR", path
    gfx_off, gfx_len = struct.unpack_from("<IH", b, 9)
    pal_off, pal_len = struct.unpack_from("<IH", b, 15)
    names = b[29:].split(b"\x00\x00", 2)
    name = author = ""
    # noms en UTF-16 : on relit caractere par caractere (paires d'octets)
    k = 29
    out = []
    for _ in range(2):
        s = bytearray()
        while k + 1 < len(b) and b[k:k + 2] != b"\x00\x00":
            s += b[k:k + 2]
            k += 2
        k += 2
        out.append(s.decode("utf-16-le", "replace"))
    name, author = out
    del names
    return {
        "name": name,
        "author": author,
        "gfx": b[gfx_off:gfx_off + gfx_len],
        "pal": b[pal_off:pal_off + pal_len],
    }


def palettes(pal):
    """4 palettes de 16 couleurs RGB (indice 0 : transparent)."""
    out = []
    for p in range(4):
        cols = [(0, 0, 0)]
        for c in range(15):
            off = (p * 15 + c) * 2
            if off + 2 > len(pal):
                cols.append((255, 0, 255))
                continue
            v = struct.unpack_from("<H", pal, off)[0]
            r, g, bl = v & 31, (v >> 5) & 31, (v >> 10) & 31
            cols.append((r * 255 // 31, g * 255 // 31, bl * 255 // 31))
        out.append(cols)
    return out


def indices(gfx):
    """Planche d'indices de couleur (liste de rangees), 128 x (8 * tuiles / 16)."""
    n = len(gfx) // 32
    h = (n + TILES_W - 1) // TILES_W * 8
    rows = [[0] * (TILES_W * 8) for _ in range(h)]
    for t in range(n):
        tx, ty = (t % TILES_W) * 8, (t // TILES_W) * 8
        d = gfx[t * 32:(t + 1) * 32]
        for r in range(8):
            p0, p1, p2, p3 = d[2 * r], d[2 * r + 1], d[16 + 2 * r], d[16 + 2 * r + 1]
            for c in range(8):
                bit = 7 - c
                v = ((p0 >> bit) & 1) | (((p1 >> bit) & 1) << 1) | (((p2 >> bit) & 1) << 2) | (((p3 >> bit) & 1) << 3)
                rows[ty + r][tx + c] = v
    return rows


def image(rows, pal):
    h, w = len(rows), len(rows[0])
    im = Image.new("RGBA", (w, h), (0, 0, 0, 0))
    px = im.load()
    for y in range(h):
        for x in range(w):
            v = rows[y][x]
            if v:
                px[x, y] = pal[v] + (255,)
    return im

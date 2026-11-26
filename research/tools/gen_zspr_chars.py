"""Personnages du A Link to the Past Randomizer (0.6.5) : un personnage jouable par sprite .zspr
(https://snesrev.github.io/sprites-gfx/snes/zelda3/link/), dessine avec les poses de Link
(research/zspr_poses.json, zspr_poses.py).

Usage : python research/tools/gen_zspr_chars.py <dossier des .zspr> <sprites.json (index du site)>

Ecrit :
  assets/source/zspr/<id>.png      planche : une rangee par cotte de mailles (palettes 0 a 2, sans les
                                   doublons), puis la rangee du lapin (palette 3)
  data/sprites/zspr.ron            manifeste des planches
  data/zspr_characters.ron         personnages (stats, deblocage, couleurs, univers, auteur)
  research/tools/lang_zspr.py      noms et descriptions (7 langues)
  CREDITS.md                       liste des auteurs (entre les marqueurs zspr)
et met a jour les personnages remplaces (data/characters/<id>.ron : sprite et couleurs).
Stats et deblocages sont tires d'un hachage de l'identifiant : relancer donne le meme resultat.
"""
import hashlib
import json
import re
import sys
from pathlib import Path

from PIL import Image

sys.path.insert(0, str(Path(__file__).parent))
import zspr  # noqa: E402
import zspr_render  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
OUT_PNG = ROOT / "assets" / "source" / "zspr"
CW, CH = 18, 26

# sprites qui remplacent un personnage existant (meme identifiant : progression du profil gardee)
REPLACE = {
    "Minish Cap Link": "link_minish",
    "NES Link": "link_nes",
    "Shy Guy": "shy_guy",
    "Vivi": "vivi",
    "Sonic the Hedgehog": "sonic",
    "Crewmate": "crewmate",
    "Ganondorf": "ganondorf",
    "Zelda": "zelda",
    "Four Swords Link": "link_fsa",
    "Dark Link": "shadow_link",
    "Hero of Awakening": "link_awakening",
    "Mario (Classic)": "mario",
    "Zora": "sea_zora",
}
# deja present tel quel (le Link d'origine)
SKIP = {"001.link.1"}

GROUPS = ["zelda", "nintendo", "rpg", "games", "screen", "community"]
ZELDA = """Link|Four Swords Link|Arrghus|Big Key|Blacksmith Link|Bob|Bottle o' Goo|BotW Link|BotW Zelda|
B.S. Boy|B.S. Girl|Casual Zelda|CD-i Link|Cucco|Dark Link|Dark Link (Tunic)|Dark Zelda|Dark Zora|Demon Link|Ezlo|
Fierce Deity Link|Fox Link|Frog Link|Mini Ganon|Ganondorf|GBC Link|Hardhat Beetle|Headless Link|Hero of Awakening|
Hero Of Hyrule|Hint Tile|Hoarder (Bush)|Hoarder (Pot)|Hoarder (Rock)|Hyrule Knight|Informant Woman|Invisible Link|
Kholdstare|Korok|Link Redrawn|Hat Color Link|Tunic Color Link|Little Hylian|Lynel (BotW)|Maiden|Manga Link|Marin|
Medallions|Medli|Minish Link|Minish Cap Link|Moblin|Modern Link|Moosh|Nature Link|Navi|Negative Link|NES Link|Old Man|
Outline Link|Parallel Worlds Link|Penguin Link|Pink Ribbon Link|Pride Link|Purple Chest|Rainbow Link|Robo-Link 9000|
Santa Hat Link|Santa Link|Hyrule Soldier|Stalfos|Static Link|Super Bomb|Super Bunny|Tetra Sheet|Thief|Tile|Tingle|
TotemLinks|TP Zelda|VanillaLink|Vitreous|Wizzrobe|Wolf Link (Festive)|Wolf Link (TP)|Zelda|Zora|Dark Boy|Dark Girl|
Linja|Rat|Pony|Ganon|Agahnim"""
NINTENDO = """Bandit|Beau|Birdo|Boo|Boo 2|Bowser|Bowsette|Bowsette Red|Bronzor|Bullet Bill|Cat Boo|Charizard|Cheep Cheep|
Decidueye|Dragonite|Geno|Goomba|Gooey|Inkling|Kecleon|Kirby (Dreamland 3)|Kirby|Lakitu|Lapras|Lucario|Luigi|
Mario (Classic)|Tanooki Mario|Mario and Cappy|Baby Metroid|Ness|Paula|Princess Peach|Pikachu|Piranha Plant|Pokey|
Psyduck|Roy Koopa|Samus|Samus (Super Metroid)|Samus (Classic)|Zero Suit Samus|Shuppet|Shy Gal|Shy Guy|Slowpoke|Squirtle|
Toad|Toadette|Captain Toadette|Vaporeon|Vivian|Wario|Yoshi|Rover|missingno|Power Up with Pride Mushroom|Mallow (Cat)"""
RPG = """Adol|Ark (Cape)|Ark (No Cape)|Black Mage|Blazer|Boco the Chocobo|Cactuar|Celes|Cornelius|Dekar|Freya Crescent|
Garnet|Kain|Kefka|Locke|Figaro Merchant|Magus|Mog|Popoi|Primm|Red Mage|Rydia|Selan|Sora|Sora (KH1)|Squall|
Terra (Esper)|Ultros|Vivi|White Mage|DQ Slime|Yunica Tovah|Lest|Jack Frost|Will|Asuna|Niddraig|Gretis"""
GAMES = """Badeline|Madeline|Cadence|Clyde|Conker|Crewmate|Frisk|Susie|Hat Kid|Hollow Knight|Hollow Knight (Malmo/Winter)|
Jason Frudnick|Megaman X|Ori|Plague Knight|Shantae|Solaire of Astora|Sonic the Hedgehog|Super Meat Boy|
Trogdor the Burninator|Ty the Tasmanian Tiger|Valeera|Cirno|Marisa Kirisame|Rumia|Momiji Inubashiri|Ryu|Terry|Niko|
Abigail|Alice|King Graham|Corona|Phoenix Wright|Ema Skye|Rottytops|Pyro|Brian|Shadow|Navirou|Hidari|Porg Knight"""
SCREEN = """Aggretsuko|Bananas In Pyjamas|Batman|Blossom|Bubbles|Buttercup|Powerpuff Girl|Branch|Poppy|Clifford|
Deadpool (SirCzah)|Deadpool (Mythic)|Dipper|Gruncle Stan|Finn Merten|Princess Bubblegum|Future Trunks|Vegeta|Garfield|
Hello Kitty|Homer Simpson|Ned Flanders|Steamed Ham|Ignignokt|Meatwad|Kenny McCormick|Luffy|Rick|Rocko|Sailor Moon|
Saitama|SpongeBob SquarePants|TMNT|Power Ranger|Bob Ross|Angry Video Game Nerd|Marvin the Cat|Katsura|Stan|Pete|
Dark Panda|Ketchup|Hotdog|Goose|The Pug|Eggplant|Broccoli"""
GROUP_OF = {}
for g, names in zip(GROUPS, [ZELDA, NINTENDO, RPG, GAMES, SCREEN]):
    for n in names.replace("\n", "").split("|"):
        GROUP_OF.setdefault(n.strip(), g)

# arme de depart d'apres le nom, sinon tiree au hasard (les armes de depart des personnages actuels)
WEAPONS = ["sword", "sword", "sword", "bombs", "magic_powder", "fire_rod", "bow", "boomerang", "ice_rod", "hookshot",
           "hammer", "cane_of_somaria", "byrna", "bees"]
THEME = [
    (r"mage|wizz|magus|kefka|terra|rydia|witch|wiitch|celes|marisa|garnet|maiden", "fire_rod"),
    (r"ice|frost|kholdstare|cirno|penguin|vaporeon|lapras|squirtle|winter", "ice_rod"),
    (r"bomb|bullet|pyro|trogdor|charizard|dragon|bowser", "bombs"),
    (r"samus|megaman|robo|archer|jason|tasbot|astronaut|metroid|ness|paula", "bow"),
    (r"blacksmith|hammer|plague|knight|solaire|hardhat|goron", "hammer"),
    (r"boo|ghost|shuppet|navi|fairy|ori|rumia", "magic_powder"),
    (r"\bbee|bug|beetle", "bees"),
    (r"chest|key|tile|medallion|bottle|controller|soda|cursor", "cane_of_somaria"),
]
GOOD = [("Might", [0.1, 0.15, 0.2]), ("Luck", [0.15, 0.2, 0.25]), ("MoveSpeed", [0.05, 0.08, 0.1]),
        ("Area", [0.1, 0.15]), ("Amount", [1.0]), ("Speed", [0.15]), ("Magnet", [0.2, 0.3]),
        ("Cooldown", [-0.08, -0.1]), ("Duration", [0.15]), ("Armor", [1.0, 2.0]), ("Growth", [0.1]),
        ("DashCooldown", [-0.15])]
BAD = [("Might", [-0.05, -0.1]), ("Luck", [-0.1]), ("MoveSpeed", [-0.05, -0.08]), ("Area", [-0.1]),
       ("Growth", [-0.1]), ("Cooldown", [0.1]), ("DashCooldown", [0.2]), ("Speed", [-0.1])]
DESC = ["{n} ({g}), venu des sprites du Randomizer de A Link to the Past.",
        "{n} ({g}), from the A Link to the Past Randomizer sprites.",
        "{n} ({g}), aus den Sprites des A Link to the Past Randomizer.",
        "{n} ({g}), de los sprites del Randomizer de A Link to the Past.",
        "{n} ({g}), dagli sprite del Randomizer di A Link to the Past.",
        "{n} ({g}), dos sprites do Randomizer de A Link to the Past.",
        "{n} ({g})。 ゼルダの でんせつ ランダマイザーの スプライト。"]
# nom de l'univers dans la description (memes textes que group.* de lang_v065.py)
GROUP_TEXT = {
    "zelda": ["Zelda", "Zelda", "Zelda", "Zelda", "Zelda", "Zelda", "ゼルダ"],
    "nintendo": ["Nintendo", "Nintendo", "Nintendo", "Nintendo", "Nintendo", "Nintendo", "ニンテンドー"],
    "rpg": ["RPG", "RPG", "RPG", "RPG", "RPG", "RPG", "RPG"],
    "games": ["jeux vidéo", "video games", "Videospiele", "videojuegos", "videogiochi", "videogames", "ゲーム"],
    "screen": ["séries et films", "TV and film", "Serien und Filme", "series y cine", "serie e film", "séries e filmes", "アニメと えいが"],
    "community": ["communauté", "community", "Community", "comunidad", "comunità", "comunidade", "コミュニティ"],
}
PRICES = [1000, 1500, 2000, 3000, 4000, 5000, 6500, 8000, 10000, 12500, 15000, 20000]
HUES = [(0, "red"), (30, "orange"), (55, "yellow"), (120, "green"), (180, "cyan"), (225, "blue"), (280, "purple"),
        (325, "pink"), (360, "red")]


def h(*parts):
    return int(hashlib.sha1("/".join(map(str, parts)).encode()).hexdigest(), 16)


def pick(seq, *parts):
    return seq[h(*parts) % len(seq)]


def shown(s):
    """Texte affiche en jeu : la police n'a ni _ ni &."""
    return s.replace("_", " ").replace("&", "+").strip()


def ident(slug):
    return "zspr_" + re.sub(r"[^a-z0-9]+", "_", slug.lower()).strip("_")


def hue_name(col):
    import colorsys
    hh, s, v = colorsys.rgb_to_hsv(*(c / 255 for c in col))
    if s < 0.25 or v < 0.2:
        return "white" if v > 0.75 else "black" if v < 0.3 else "gray"
    deg = hh * 360
    if 15 <= deg < 45 and v < 0.6:
        return "brown"
    return min(HUES, key=lambda t: abs(t[0] - deg))[1]


def color_name(cols, weights):
    """Nom de couleur (cle skin.*) : la teinte qui couvre le plus de pixels (chaque couleur comptee pour
    son nombre de pixels dans les poses)."""
    score = {}
    for col, w in zip(cols, weights):
        score[hue_name(col)] = score.get(hue_name(col), 0) + w
    return max(score, key=score.get) if score and max(score.values()) else None


def unique(frames):
    """Images distinctes (indices) et, pour chaque image d'origine, son rang dans la liste distincte."""
    keys, out, idx = {}, [], []
    for im in frames:
        k = (im.size, im.tobytes())
        if k not in keys:
            keys[k] = len(out)
            out.append(im)
        idx.append(keys[k])
    return out, idx


def render(z):
    """{skin: {anim: [Image]}} : cottes distinctes (palettes 0 a 2) et lapin (palette 3)."""
    rows = zspr.indices(z["gfx"])
    pals = zspr.palettes(z["pal"])
    walk = {a: p for a, p in zspr_render.POSES.items() if not a.startswith("bunny_")}
    bunny = {a[6:]: p for a, p in zspr_render.POSES.items() if a.startswith("bunny_")}
    skins = []
    seen = set()
    for k in range(3):
        fr = {a: [zspr_render.frame(rows, pals[k], p) for p in ps] for a, ps in walk.items()}
        key = b"".join(im.tobytes() for ims in fr.values() for im in ims)
        if key in seen:
            continue
        seen.add(key)
        skins.append((k, fr))
    fb = {a: [zspr_render.frame(rows, pals[3], p) for p in ps] for a, ps in bunny.items()}
    if not any(im.getbbox() for ims in fb.values() for im in ims):
        fb = None
    # pixels de chaque indice de couleur dans les poses de marche
    counts = [0] * 16
    for ps in walk.values():
        for p in ps:
            im = zspr_render.frame(rows, [(i, i, i) for i in range(16)], p)
            for px in im.getdata():
                if px[3]:
                    counts[px[0]] += 1
    return pals, skins, fb, counts


def skin_names(pals, skins, counts):
    """Cle de couleur de chaque cotte : couleurs qui changent d'une palette a l'autre, ponderees par
    leur nombre de pixels dans les poses (la couleur la plus visible l'emporte)."""
    ks = [k for k, _ in skins]
    if len(ks) == 1:
        return ["p1"]
    diff = [i for i in range(1, 16) if len({pals[k][i] for k in ks}) > 1]
    names = []
    for n, k in enumerate(ks):
        c = color_name([pals[k][i] for i in diff], [counts[i] for i in diff])
        names.append(c if c and c not in names else f"p{n + 1}")
    return names


def sheet(skins, fb):
    """Planche et rectangles : {(rangee, anim): [(x, y, l, h)]}."""
    rows = [fr for _, fr in skins] + ([fb] if fb else [])
    plans = []
    width = 0
    for fr in rows:
        order = [im for ims in fr.values() for im in ims]
        uniq, idx = unique(order)
        plans.append((fr, uniq, idx))
        width = max(width, len(uniq) * CW)
    img = Image.new("RGBA", (width, CH * len(rows)), (0, 0, 0, 0))
    rects = {}
    for r, (fr, uniq, idx) in enumerate(plans):
        boxes = []
        for c, im in enumerate(uniq):
            x, y = c * CW, r * CH + CH - im.height
            img.alpha_composite(im, (x, y))
            boxes.append((x, y, im.width, im.height))
        k = 0
        for a, ims in fr.items():
            rects[(r, a)] = [boxes[idx[k + j]] for j in range(len(ims))]
            k += len(ims)
    return img, rects


def anim_lines(rects, r, bunny):
    out = []
    for a in ("idle_down", "idle_side", "idle_up", "walk_down", "walk_side", "walk_up"):
        fr = list(rects[(r, a)])
        if a.startswith("walk") and not bunny:
            # comme Link : 7 images de marche puis l'image au repos
            fr.append(rects[(r, "idle" + a[4:])][0])
        fps = "" if a.startswith("idle") else (", fps: 8" if bunny else ", fps: 16")
        s = ", ".join(f"({x}, {y}, {w}, {hh})" for x, y, w, hh in fr)
        out.append(f'                    "{a}": (frames: [{s}]{fps}, pivot: (0, -1), align: Bottom),')
    return out


def stats_for(cid, name):
    weapon = next((w for pat, w in THEME if re.search(pat, name.lower())), None) or pick(WEAPONS, cid, "w")
    goods = sorted(range(len(GOOD)), key=lambda i: h(cid, "g", i))[:2]
    used = {GOOD[i][0] for i in goods}
    bads = [i for i in sorted(range(len(BAD)), key=lambda i: h(cid, "b", i)) if BAD[i][0] not in used][:1]
    stats = [(GOOD[i][0], pick(GOOD[i][1], cid, "gv", i)) for i in goods] + [(BAD[i][0], pick(BAD[i][1], cid, "bv", i)) for i in bads]
    armor = any(s == "Armor" for s, _ in stats)
    hp = pick([70.0, 80.0] if armor else [40.0, 50.0, 60.0, 60.0, 70.0], cid, "hp")
    speed = pick([80.0, 82.0, 84.0, 86.0, 86.0, 88.0, 90.0, 92.0], cid, "sp")
    magnet = pick([30.0, 34.0, 34.0, 36.0, 38.0, 40.0], cid, "mg")
    dash = pick(["(speed: 330.0, duration: 0.15, cooldown: 1.8)", "(speed: 320.0, duration: 0.15, cooldown: 2.0)",
                 "(speed: 340.0, duration: 0.15, cooldown: 1.5)", "(speed: 320.0, duration: 0.15, cooldown: 1.9)"], cid, "d")
    return weapon, stats, hp, speed, magnet, dash


def fmt(v):
    return f"{v:.2f}".rstrip("0").rstrip(".") if v != int(v) else f"{v:.1f}"


def ron_list(text, key):
    return re.findall(r'^    "(\w+)": ?\(', text, re.M) if key == "enemies" else []


def unlocks(ids, names):
    """Un tiers en boutique (prix croissants), le reste par exploits ; un sprite d'ennemi : le vaincre."""
    enemies = ron_list((ROOT / "data" / "enemies.ron").read_text(encoding="utf-8"), "enemies")
    stages = sorted(p.stem for p in (ROOT / "data" / "maps").glob("*.ron") if p.stem != "arena_test")
    out = {}
    rest = []
    for cid in ids:
        key = re.sub(r"[^a-z]+", "_", names[cid].lower()).strip("_")
        match = next((e for e in enemies if e == key or e == key.rstrip("s") or (key.startswith(e) and len(e) > 3)), None)
        if match:
            out[cid] = f'Slain("{match}")'
        else:
            rest.append(cid)
    rest.sort(key=lambda c: h(c, "u"))
    shop = rest[: len(rest) // 3]
    for k, cid in enumerate(sorted(shop, key=lambda c: h(c, "p"))):
        out[cid] = f"Shop({PRICES[k * len(PRICES) // len(shop)]})"
    feats = rest[len(rest) // 3:]
    kinds = ["kills", "bosses", "level", "slain", "win"]
    per = {k: [c for j, c in enumerate(feats) if kinds[j % len(kinds)] == k] for k in kinds}
    n = len(per["kills"])
    for j, cid in enumerate(per["kills"]):
        v = 500 * (400 ** (j / max(n - 1, 1)))
        out[cid] = f"Kills({int(round(v, -2 if v < 10000 else -3))})"
    n = len(per["bosses"])
    for j, cid in enumerate(per["bosses"]):
        out[cid] = f"Bosses({max(2, int(round(2 * (150 ** (j / max(n - 1, 1))))))})"
    for j, cid in enumerate(per["level"]):
        out[cid] = f"Level({12 + j * 48 // max(len(per['level']) - 1, 1)})"
    pool = [e for e in enemies if e not in ("bobomb", "boo")]
    for j, cid in enumerate(per["slain"]):
        out[cid] = f'Slain("{pool[j % len(pool)]}")'
    for j, cid in enumerate(per["win"]):
        out[cid] = f'Win("{stages[j % len(stages)]}")'
    return out


def main():
    folder, index = Path(sys.argv[1]), Path(sys.argv[2])
    meta = {s["slug"]: s for s in json.loads(index.read_text(encoding="utf-8")) if "slug" in s}
    OUT_PNG.mkdir(parents=True, exist_ok=True)
    manifest = ["// 0.6.5 : personnages du A Link to the Past Randomizer. Genere par research/tools/gen_zspr_chars.py,",
                "// ne pas modifier.", "(", "    sheets: ["]
    chars, names, authors, skins_of, groups = {}, {}, {}, {}, {}
    taken = set()
    for f in sorted(folder.glob("*.zspr")):
        if f.read_bytes()[:4] != b"ZSPR" or f.stem in SKIP:
            continue
        info = meta.get(f.stem, {})
        z = zspr.read(f)
        name = (info.get("name") or z["name"]).strip()
        cid = REPLACE.get(name) or ident(info.get("short_slug") or f.stem)
        if cid in taken:
            continue
        taken.add(cid)
        pals, skins, fb, counts = render(z)
        img, rects = sheet(skins, fb)
        img.save(OUT_PNG / f"{cid}.png")
        keys = skin_names(pals, skins, counts)
        sk = []
        manifest += [f"        // {name} ({info.get('author') or z['author']})", "        (",
                     f'            source: "zspr/{cid}.png",', "            sprites: {"]
        for r, key in enumerate(keys):
            sprite = cid if r == 0 else f"{cid}_{key}"
            sk.append((sprite, key))
            manifest += [f'                "{sprite}": {{'] + anim_lines(rects, r, False) + ["                },"]
        if fb:
            sprite = f"{cid}_bunny"
            # forme du Monde des Tenebres (un lapin pour Link, propre a chaque sprite sinon)
            sk.append((sprite, "dark"))
            manifest += [f'                "{sprite}": {{'] + anim_lines(rects, len(keys), True) + ["                },"]
        manifest += ["            },", "        ),"]
        names[cid], authors[cid], skins_of[cid] = name, (info.get("author") or z["author"]).strip(), sk
        groups[cid] = GROUP_OF.get(name, "community")
        chars[cid] = name
    manifest += ["    ],", ")", ""]
    (ROOT / "data" / "sprites" / "zspr.ron").write_text("\n".join(manifest), encoding="utf-8")

    new = sorted((c for c in chars if c.startswith("zspr_")), key=lambda c: (GROUPS.index(groups[c]), names[c].lower()))
    unl = unlocks(new, names)
    lines = ["// 0.6.5 : personnages du A Link to the Past Randomizer (sprites .zspr, poses de Link).",
             "// Genere par research/tools/gen_zspr_chars.py, ne pas modifier.", "{"]
    for k, cid in enumerate(new):
        weapon, stats, hp, speed, magnet, dash = stats_for(cid, names[cid])
        st = ", ".join(f"({s}, {fmt(v)})" for s, v in stats)
        sk = ", ".join(f'("{s}", "{c}")' for s, c in skins_of[cid])
        lines += [f'    "{cid}": (',
                  f"        name: {json.dumps(names[cid], ensure_ascii=False)},",
                  f'        sprite: "{cid}",',
                  f"        max_hp: {fmt(hp)}, speed: {fmt(speed)}, magnet: {fmt(magnet)},",
                  f"        dash: {dash},",
                  f'        weapon: "{weapon}", weapon_sprite: "sword1",',
                  f"        stats: [{st}],",
                  f"        order: {1000 + k},",
                  f"        unlock: Some({unl[cid]}),",
                  f"        skins: [{sk}],",
                  f'        group: "{groups[cid]}",',
                  f"        credit: {json.dumps(shown(authors[cid]), ensure_ascii=False)},",
                  "    ),"]
    lines += ["}", ""]
    (ROOT / "data" / "zspr_characters.ron").write_text("\n".join(lines), encoding="utf-8")

    # personnages remplaces : nouveau sprite, couleurs des palettes, auteur
    for name, cid in REPLACE.items():
        if cid not in skins_of:
            print(f"absent : {name}")
            continue
        p = ROOT / "data" / "characters" / f"{cid}.ron"
        t = p.read_text(encoding="utf-8")
        t = re.sub(r'\n    sprite: "[^"]*",', f'\n    sprite: "{cid}",', t)
        t = re.sub(r"\n    side_faces_left: true,", "", t)
        t = re.sub(r"\n    // 0\.6\.4 : tenues[^\n]*", "", t)
        t = re.sub(r"\n    skins: \[[^\n]*\],", "", t)
        t = re.sub(r"\n    credit: [^\n]*,", "", t)
        sk = ", ".join(f'("{s}", "{c}")' for s, c in skins_of[cid])
        t = t.rstrip().rstrip(")").rstrip()
        t += f"\n    // 0.6.5 : sprite du Randomizer ({name}), couleurs de ses palettes : gen_zspr_chars.py\n"
        t += f"    skins: [{sk}],\n    credit: {json.dumps(shown(authors[cid]), ensure_ascii=False)},\n)\n"
        p.write_text(t, encoding="utf-8")

    # textes : noms (identiques dans toutes les langues) et description avec l'auteur
    rows = ["# 0.6.5 : noms et descriptions des personnages du Randomizer. Genere par gen_zspr_chars.py.", "ROWS = {"]
    for cid in new:
        n = json.dumps(shown(names[cid]), ensure_ascii=False)
        rows.append(f'    "char.{cid}.name": [{", ".join([n] * 7)}],')
        d = [json.dumps(t.format(n=shown(names[cid]), g=g), ensure_ascii=False) for t, g in zip(DESC, GROUP_TEXT[groups[cid]])]
        rows.append(f'    "char.{cid}.desc": [{", ".join(d)}],')
    rows += ["}", ""]
    (ROOT / "research" / "tools" / "lang_zspr.py").write_text("\n".join(rows), encoding="utf-8")

    # credits
    cp = ROOT / "CREDITS.md"
    t = cp.read_text(encoding="utf-8")
    by = {}
    for cid in sorted(chars, key=lambda c: names[c].lower()):
        by.setdefault(authors[cid], []).append(names[cid])
    body = ["<!-- zspr:start -->"] + [f"- {a} : {', '.join(v)}" for a, v in sorted(by.items(), key=lambda x: x[0].lower())] + ["<!-- zspr:end -->"]
    block = "\n".join(body)
    if "<!-- zspr:start -->" in t:
        t = re.sub(r"<!-- zspr:start -->.*?<!-- zspr:end -->", lambda _: block, t, flags=re.S)
    else:
        t = t.rstrip() + "\n\n" + block + "\n"
    cp.write_text(t, encoding="utf-8")
    kinds = {}
    for v in unl.values():
        kinds[v.split("(")[0]] = kinds.get(v.split("(")[0], 0) + 1
    print(f"{len(chars)} personnages ({len(new)} nouveaux), deblocages {kinds}, univers "
          f"{ {g: sum(1 for c in new if groups[c] == g) for g in GROUPS} }")


if __name__ == "__main__":
    main()

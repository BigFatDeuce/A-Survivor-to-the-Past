"""Manuel du jeu, sur le modele du livret d'A Link to the Past (Super Nintendo) : couverture, legende,
chapitres numerotes a grands titres, encadres a bandeau noir, objets avec leur icone, bestiaire.

Les listes (armes, objets, evolutions, personnages, stages, boutique, reliques, Sceaux, bestiaire) sont
lues dans les donnees du jeu et ses traductions : le manuel suit chaque version (a regenerer a chaque
publication). Les textes sont dans manual_text.py. Les captures sont prises par le jeu lui-meme, dans un
dossier temporaire (jamais avec les joueurs ni les reglages de ce dossier), son coupe.

Usage : python research/tools/make_manual.py [--lang fr,en] [--no-shots]
Sortie : dist/manual/A-Survivor-to-the-Past-<Manuel|Manual>-<lang>.pdf (pages HTML de travail a cote).
Demande : le jeu compile en release (cargo build --release -p game) et Microsoft Edge ou Google Chrome.
"""
import argparse
import html
import os
import re
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

from PIL import Image

sys.path.insert(0, str(Path(__file__).resolve().parent))
import make_readme as rd  # noqa: E402
import manual_text as mt  # noqa: E402

ROOT = rd.ROOT
OUT = ROOT / "dist" / "manual"
EXE = ROOT / "target" / "release" / "a_survivor_to_the_past.exe"
BROWSERS = [
    Path(os.environ.get("ProgramFiles(x86)", r"C:\Program Files (x86)")) / "Microsoft/Edge/Application/msedge.exe",
    Path(os.environ.get("ProgramFiles", r"C:\Program Files")) / "Microsoft/Edge/Application/msedge.exe",
    Path(os.environ.get("ProgramFiles", r"C:\Program Files")) / "Google/Chrome/Application/chrome.exe",
]
# agrandissement des images de pixels dans le PDF (nettes a tous les zooms)
PIX = 8

# ---------------------------------------------------------------------------------------------
# Donnees du jeu


def read(rel):
    return (ROOT / rel).read_text(encoding="utf-8")


def lang_table(code):
    out = {}
    for m in re.finditer(r'^\s*"([^"]+)": "((?:[^"\\]|\\.)*)",\s*$', read(f"data/lang/{code}.ron"), re.M):
        out[m.group(1)] = m.group(2).replace('\\"', '"').replace("\\\\", "\\")
    return out


def blocks(text):
    """Entrees de premier niveau d'une table RON : (identifiant, texte du bloc)."""
    ids = list(re.finditer(r'^    "(\w+)": \(', text, re.M))
    return [(m.group(1), text[m.end(): ids[k + 1].start() if k + 1 < len(ids) else len(text)]) for k, m in enumerate(ids)]


def field(block, name, default=None):
    m = re.search(rf'\b{name}:\s*"?([\w.\-]+)"?', block)
    return m.group(1) if m else default


def weapons():
    out = []
    for wid, b in blocks(read("data/weapons.ron")):
        e = re.search(r'evolution: Some\(\(passive: "(\w+)", into: "(\w+)"\)\)', b)
        out.append(dict(id=wid, icon=field(b, "icon"), evolved="evolved: true" in b, evo=e.groups() if e else None))
    return out


def passives():
    return [dict(id=pid, icon=field(b, "icon")) for pid, b in blocks(read("data/passives.ron"))]


def characters():
    out = []
    for f in sorted((ROOT / "data/characters").glob("*.ron")):
        t = f.read_text(encoding="utf-8")
        u = re.search(r"unlock:\s*Some\((\w+)\(\"?([\w]+)\"?\)\)", t)
        out.append(dict(id=f.stem, sprite=field(t, "sprite"), weapon=field(t, "weapon"),
                        order=int(field(t, "order", "99")), unlock=u.groups() if u else None))
    return sorted(out, key=lambda c: c["order"])


def enemies():
    out = []
    for eid, b in blocks(read("data/enemies.ron")):
        if "bestiary: false" in b:
            continue
        out.append(dict(id=eid, sprite=field(b, "sprite"), boss="boss: true" in b,
                        hp=float(field(b, "hp", "0")), damage=float(field(b, "damage", "0")), speed=float(field(b, "speed", "0"))))
    return out


def stages():
    out = []
    for f in (ROOT / "data/maps").glob("*.ron"):
        t = f.read_text(encoding="utf-8")
        order = int(field(t, "order", "0"))
        if order <= 0:
            continue
        mid = re.search(r'\bboss: Some\(\(at: [\d.]+, enemy: "(\w+)"\)\)', t)
        fin = re.search(r'final_boss: Some\(\(at: [\d.]+, enemy: "(\w+)"\)\)', t)
        out.append(dict(id=f.stem, order=order, mid=mid.group(1) if mid else None, final=fin.group(1) if fin else None))
    return sorted(out, key=lambda s: s["order"])


def meta():
    t = read("data/meta.ron")

    def section(name):
        m = re.search(rf"^\s*{name}: \[(.*?)^\s*\],", t, re.M | re.S)
        rows = []
        for line in (m.group(1) if m else "").splitlines():
            line = line.split("//")[0]
            if "id:" in line:
                st = re.search(r"stats: \[(.*?)\]", line)
                rows.append(dict(id=field(line, "id"), icon=field(line, "icon"), stage=field(line, "stage"),
                                 encounter="encounter: true" in line, max=field(line, "max"), bonus=field(line, "bonus"),
                                 cost=field(line, "cost"),
                                 stats=[(a, float(b)) for a, b in re.findall(r"\((\w+), (-?[\d.]+)\)", st.group(1))] if st else []))
        return rows

    return dict(shop=section("shop"), blessings=section("blessings"), relics=section("relics"),
                seals=section("seals"), difficulties=section("difficulties"))


STAT_KEYS = {"MaxHp": "stat.max_hp", "Armor": "stat.armor", "Amount": "stat.amount", "Regen": "stat.regen",
             "Luck": "stat.luck", "Growth": "stat.growth", "Area": "stat.area", "Cooldown": "stat.cooldown",
             "Duration": "stat.duration", "Speed": "stat.speed", "MoveSpeed": "stat.move_speed", "Magnet": "stat.magnet",
             "DashCooldown": "stat.dash_cd", "Might": "stat.might"}


def stat_text(L, s, v):
    """Comme meta::stat_text dans le jeu : « +10 % Puissance »."""
    if s == "MaxHp":
        val = f"+{v / 10:g}"
    elif s in ("Armor", "Amount"):
        val = f"+{v:g}"
    elif s == "Regen":
        val = f"+{v:.1f}"
    else:
        val = f"{round(v * 100):+d} %"
    return f"{val} {L.get(STAT_KEYS.get(s, 'stat.might'), s)}"


# ---------------------------------------------------------------------------------------------
# Atlas du jeu : sprites et police


class Atlas:
    def __init__(self):
        text = read("assets/gen/atlas.ron")
        self.img = Image.open(ROOT / "assets/gen/atlas.png").convert("RGBA")
        self.sprites = {}
        sprite = None
        for line in text.splitlines():
            m = re.match(r'^        "([^"]+)": \{$', line)
            if m:
                sprite = m.group(1)
                self.sprites[sprite] = {}
                continue
            m = re.match(r'^            "([^"]+)": \(frames: \[(.*?)\]', line)
            if m and sprite:
                fr = [tuple(int(v) for v in f) for f in
                      re.findall(r"\(x: (\d+), y: (\d+), w: (\d+), h: (\d+), px: (-?\d+), py: (-?\d+)\)", m.group(2))]
                self.sprites[sprite][m.group(1)] = fr
        fonts = text[text.index("    fonts: {"):]
        main = fonts[fonts.index('"main": ('):]
        gl = main[main.index("glyphs: {"): main.index("},", main.index("glyphs: {"))]
        self.glyphs = {}
        for m in re.finditer(r"'(\\'|\\\\|[^'])': \(x: (\d+), y: (\d+), w: (\d+), h: (\d+), oy: (-?\d+)\)", gl):
            ch = m.group(1).replace("\\'", "'").replace("\\\\", "\\")
            self.glyphs[ch] = tuple(int(v) for v in m.groups()[1:])
        num = lambda k, d: int(re.search(rf"{k}: (-?\d+)", main).group(1)) if re.search(rf"{k}: (-?\d+)", main) else d  # noqa: E731
        self.line_h, self.space, self.spacing = num("line_height", 14), num("space", 5), num("spacing", -1)

    def frame(self, sprite, anim, i=0):
        fr = self.sprites.get(sprite, {}).get(anim)
        if not fr:
            return None
        x, y, w, h, _, _ = fr[min(i, len(fr) - 1)]
        return self.img.crop((x, y, x + w, y + h))

    def pick(self, sprite, prefs=("walk_down", "idle_down", "walk", "idle", "fly", "0", "down", "hop")):
        anims = self.sprites.get(sprite, {})
        for p in prefs:
            if p in anims:
                return self.frame(sprite, p)
        for a in sorted(anims):
            if "down" in a:
                return self.frame(sprite, a)
        return self.frame(sprite, sorted(anims)[0]) if anims else None

    def text(self, s, color=(255, 255, 255), dark=(40, 16, 16)):
        """Texte dans la police du jeu : remplissage en `color`, contour sombre."""
        base = {"é": "é"}  # (les glyphes accentues sont dans l'atlas)
        w = 0
        for ch in s:
            ch = base.get(ch, ch)
            w += self.space if ch == " " else (self.glyphs[ch][2] + self.spacing if ch in self.glyphs else self.space)
        out = Image.new("RGBA", (max(1, w + 2), self.line_h + 6), (0, 0, 0, 0))
        x = 0
        for ch in s:
            g = self.glyphs.get(ch)
            if ch == " " or not g:
                x += self.space
                continue
            gx, gy, gw, gh, oy = g
            im = self.img.crop((gx, gy, gx + gw, gy + gh))
            px = im.load()
            for j in range(gh):
                for i in range(gw):
                    r, gg, b, a = px[i, j]
                    if a:
                        lum = (r + gg + b) / 765
                        px[i, j] = tuple(int(dark[k] + (color[k] - dark[k]) * lum) for k in range(3)) + (a,)
            out.alpha_composite(im, (x, 3 + oy))
            x += gw + self.spacing
        return out.crop(out.getbbox() or (0, 0, 1, 1))


# ---------------------------------------------------------------------------------------------
# Captures d'ecran (le jeu dans un dossier temporaire)

SHOTS = [
    ("title", ["--menu", "title"], 2.2),
    ("players", ["--menu", "players"], 1.2),
    ("hud", ["--demo", "--seed", "7", "--time", "250", "--give", "boomerang,bow,flippers,fire_rod,pegasus_boots"], 7.0),
    ("levelup", ["--seed", "5", "--give", "boomerang,bow,hookshot", "--menu", "levelup"], 1.5),
    ("thief", ["--demo", "--seed", "9", "--time", "100", "--give", "thief"], 1.6),
    ("boss", ["--demo", "--seed", "4", "--map", "ice_lake", "--time", "537", "--give", "fire_rod,bombos"], 9.0),
    ("chars", ["--menu", "chars"], 1.5),
    ("stages", ["--menu", "stages"], 1.5),
    ("shop", ["--menu", "shop"], 1.5),
    ("scores", ["--menu", "scores"], 1.5),
]

# joueurs d'exemple (noms et scores fictifs) pour les ecrans Joueurs et Classements
PLAYERS = [("LINK", 0, 61230, 2950, 588.0, True), ("IMPA", 1, 48770, 2410, 571.0, True),
           ("MOBLIN", 6, 23410, 1690, 412.0, False), ("AGAHNIM", 19, 15980, 1120, 330.0, False)]


def seed_players(root):
    for k, (name, avatar, score, kills, time, won) in enumerate(PLAYERS):
        runs = []
        for j in range(5 if k == 0 else 1):
            s = int(score * (1 - j * 0.13))
            runs.append(f'(score: {s}, time: {time - j * 40:.1f}, kills: {int(kills * (1 - j * 0.13))}, level: {38 - j * 4}, '
                        f'bosses: {2 if won and j < 2 else 1}, won: {"true" if won and j < 2 else "false"}, character: "link", '
                        f'difficulty: 1, mult: 1.0, version: "{rd.VERSION}", published: {"true" if j == 0 else "false"})')
        text = (f'(version: 2, name: "{name}", avatar: {avatar}, rupees: {4200 - k * 900}, '
                f'stages: {{"hyrule_field": (wins: {2 if won else 0}, runs: {9 - k}, best_time: 560.0)}}, '
                f'totals: (runs: {20 - k * 4}, wins: {3 - k if won else 0}, kills: {38000 - k * 7000}, time: {9000.0 - k * 1500}), '
                f'scores: {{"hyrule_field": [{", ".join(runs)}]}})\n')
        (root / f"profile_{k + 1}.ron").write_text(text, encoding="utf-8")
    (root / "profiles.ron").write_text("(active: 0)\n", encoding="utf-8")


def capture(lang, dest):
    import _winapi

    dest.mkdir(parents=True, exist_ok=True)
    tmp = Path(tempfile.mkdtemp(prefix="astp_manual_"))
    links = [tmp / "data", tmp / "assets"]
    try:
        shutil.copy(EXE, tmp / EXE.name)
        for link in links:
            _winapi.CreateJunction(str(ROOT / link.name), str(link))
        (tmp / "settings.ron").write_text(
            f'(lang: "{lang}", filter: Retro, master: 0.0, music: 0.0, sfx: 0.0, fullscreen: false, updates: Off, bindings: {{}})\n',
            encoding="utf-8")
        seed_players(tmp)
        empty = tmp / "no_updates"
        empty.mkdir()
        for name, args, at in SHOTS:
            out = dest / f"{name}.png"
            cmd = [str(tmp / EXE.name), "--lang", lang, "--window", "1280x720", "--update-source", str(empty), *args,
                   "--shot", str(out), str(at), "--seconds", str(at + 0.8)]
            subprocess.run(cmd, cwd=tmp, timeout=180, capture_output=True)
            print("  capture", lang, name, "ok" if out.exists() else "ABSENTE")
    finally:
        for link in links:
            if link.exists() or os.path.islink(link):
                os.rmdir(link)  # retire la jonction seule, jamais son contenu
        shutil.rmtree(tmp, ignore_errors=True)


# ---------------------------------------------------------------------------------------------
# Pages


CSS = """
@page { size: 118mm 171.5mm; margin: 0; }
* { box-sizing: border-box; -webkit-print-color-adjust: exact; print-color-adjust: exact; }
html, body { margin: 0; padding: 0; background: #fff; }
body { font: 7.4pt/1.36 "Segoe UI", Arial, sans-serif; color: #1f1b12; }
img.px { image-rendering: pixelated; }
#flow { position: absolute; left: -200mm; top: 0; width: 102mm; }
.page { width: 118mm; height: 171.5mm; position: relative; overflow: hidden; break-after: page;
        background: radial-gradient(ellipse at 50% 30%, #f8f1da 0%, #efe3bf 70%, #e6d6a8 100%); }
.page::before { content: ""; position: absolute; inset: 3.2mm; border: 0.35mm solid #b49a5c; border-radius: 1.5mm; pointer-events: none; }
.page .body { position: absolute; left: 8mm; right: 8mm; top: 8mm; bottom: 12.5mm; overflow: hidden; }
.pageno { position: absolute; bottom: 4.4mm; left: 50%; transform: translateX(-50%); background: #1f5a33; color: #fff;
          font: bold 6.6pt "Segoe UI", sans-serif; min-width: 7mm; text-align: center; padding: .4mm 1.6mm; border-radius: 1mm; }
p { margin: 0 0 1.6mm; text-align: justify; hyphens: auto; }
.chapter { margin: 0 0 3mm; padding-bottom: 1.6mm; border-bottom: .5mm solid #c8242b; }
.chapter img { display: block; height: 8.6mm; }
.bar { background: #17130c; color: #fff; font-weight: 700; letter-spacing: .04em; text-transform: uppercase;
       padding: .8mm 2mm; margin: 2.6mm 0 1.6mm; font-size: 7pt; border-radius: .6mm; }
.bar::before { content: ""; display: inline-block; width: 2.1mm; height: 2.1mm; background: #d8282e; margin-right: 1.6mm;
               vertical-align: -.15mm; }
.item { display: flex; gap: 2.2mm; margin: 1.2mm 0 1.8mm; align-items: flex-start; }
.ic { flex: 0 0 10mm; min-height: 9mm; display: flex; align-items: center; justify-content: center; }
.lbl { display: inline-block; background: #c8242b; color: #fff; font-weight: 700; padding: .1mm 2mm .2mm; border-radius: 3mm;
       margin: 0 0 .7mm; font-size: 7.2pt; }
.lbl.gold { background: #8a6a12; }
.lbl.blue { background: #2a4f9a; }
.txt p { margin: 0 0 .8mm; text-align: left; }
.muted { color: #6a5c3d; font-size: 6.6pt; }
figure { margin: 1mm 0 2.4mm; }
figure img { width: 92%; margin: 0 auto; height: auto; display: block; border: .45mm solid #17130c; border-radius: .6mm; }
figcaption { font-size: 6.4pt; color: #5d5038; text-align: center; margin-top: .6mm; font-style: italic; }
table { width: 100%; border-collapse: collapse; font-size: 6.6pt; margin: 1mm 0 2mm; }
th { background: #17130c; color: #fff; text-align: left; font-weight: 700; }
td, th { border: .2mm solid #a08955; padding: .6mm 1mm; vertical-align: middle; }
tr:nth-child(even) td { background: rgba(255, 255, 255, .4); }
td img { vertical-align: middle; margin-right: 1mm; }
.grid2 { display: grid; grid-template-columns: 1fr 1fr; gap: .8mm 3mm; }
.cell { display: flex; gap: 1.8mm; align-items: center; padding: .8mm 1mm; background: rgba(255, 255, 255, .35);
        border: .2mm solid #cdb77c; border-radius: .8mm; break-inside: avoid; }
.cell .ic { flex: 0 0 11mm; min-height: 10mm; }
.cell b { display: block; font-size: 7pt; }
.cell span { font-size: 6.3pt; color: #4f442d; line-height: 1.25; }
.quote { font-style: italic; text-align: center; color: #7a2a1a; margin: 3mm 5mm; font-size: 8pt; }
.toc { list-style: none; padding: 0; margin: 2mm 0; font-size: 8.4pt; }
.toc li { display: flex; align-items: baseline; margin: 1.6mm 0; }
.toc li .dots { flex: 1; border-bottom: .3mm dotted #8a7444; margin: 0 1.5mm; }
.toc li b { color: #c8242b; }
.sprites { display: flex; justify-content: center; align-items: flex-end; gap: 3mm; margin: 3mm 0; }
.tip { display: flex; gap: 2mm; margin: 0 0 2mm; } .tip::before { content: "▶"; color: #c8242b; font-size: 7pt; }
/* couverture et dos */
.page.full { background: #0c1410; }
.page.full::before { display: none; }
.page.full .body { inset: 0; }
.cover { position: absolute; inset: 0; background: radial-gradient(ellipse at 50% 38%, #1e3b2a 0%, #0b1510 62%, #050907 100%);
         color: #f1e4b5; text-align: center; }
.cover .frame { position: absolute; inset: 4mm; border: .6mm solid #c9a23c; border-radius: 2mm; }
.cover .frame2 { position: absolute; inset: 5.4mm; border: .25mm solid #6f5a22; border-radius: 1.6mm; }
.cover .logo { position: absolute; left: 0; right: 0; top: 16mm; }
.cover .logo img { width: 62mm; }
.cover .name { position: absolute; left: 0; right: 0; top: 76mm; }
.cover .name img { height: 9mm; display: block; margin: 0 auto 1.2mm; }
.cover .band { position: absolute; left: 12mm; right: 12mm; top: 99mm; background: #c8242b; color: #fff; font-weight: 800;
               letter-spacing: .12em; padding: 1.2mm 0; font-size: 8pt; border-radius: .6mm; }
.cover .ver { position: absolute; left: 0; right: 0; top: 108mm; font-size: 7.4pt; color: #d9c58a; }
.cover .row { position: absolute; left: 0; right: 0; bottom: 22mm; display: flex; justify-content: center; align-items: flex-end; gap: 3.2mm; }
.cover .note { position: absolute; left: 10mm; right: 10mm; bottom: 9mm; font-size: 6.4pt; color: #b9aa7a; }
.back { position: absolute; inset: 0; background: radial-gradient(ellipse at 50% 40%, #1e3b2a 0%, #08100b 100%);
        color: #e9ddb2; padding: 16mm 12mm; text-align: center; font-size: 7.4pt; }
.back img.t { height: 8mm; margin-bottom: 5mm; }
.back p { text-align: center; margin: 0 0 1.8mm; }
.back .cr { position: absolute; left: 12mm; right: 12mm; bottom: 12mm; font-size: 6.2pt; color: #a99a6c; }
.back .tri { margin: 8mm 0; }
"""

PAGINATE = r"""
(function () {
  const book = document.getElementById('book');
  const flow = document.getElementById('flow');
  const items = Array.from(flow.children);
  let page = null, body = null, grid = null, n = 0;
  function newPage(cls) {
    page = document.createElement('section');
    page.className = 'page' + (cls ? ' ' + cls : '');
    body = document.createElement('div');
    body.className = 'body';
    page.appendChild(body);
    book.appendChild(page);
    grid = null;
    n++;
    page.dataset.n = n;
  }
  const over = () => body.scrollHeight > body.clientHeight + 1;
  for (const b of items) {
    if (b.classList.contains('fullpage')) { newPage('full'); body.appendChild(b); page = null; continue; }
    if (!page || b.classList.contains('newpage')) newPage();
    if (b.classList.contains('cell')) {
      if (!grid) { grid = document.createElement('div'); grid.className = 'grid2'; body.appendChild(grid); }
      grid.appendChild(b);
      if (over()) {
        grid.removeChild(b);
        let prev = null;
        if (!grid.children.length) {
          body.removeChild(grid);
          prev = body.lastElementChild;
          if (prev && prev.classList.contains('keep') && body.children.length > 1) body.removeChild(prev); else prev = null;
        }
        newPage();
        if (prev) body.appendChild(prev);
        grid = document.createElement('div'); grid.className = 'grid2'; body.appendChild(grid); grid.appendChild(b);
      }
      continue;
    }
    grid = null;
    body.appendChild(b);
    if (over() && body.children.length > 1) {
      body.removeChild(b);
      const prev = body.lastElementChild;
      const keep = prev && prev.classList.contains('keep') && body.children.length > 1;
      if (keep) body.removeChild(prev);
      newPage();
      if (keep) body.appendChild(prev);
      body.appendChild(b);
    }
  }
  // numeros de page (ni la couverture ni le dos) et sommaire
  for (const p of document.querySelectorAll('.page')) {
    if (p.classList.contains('full')) continue;
    const s = document.createElement('div'); s.className = 'pageno'; s.textContent = p.dataset.n; p.appendChild(s);
  }
  for (const li of document.querySelectorAll('[data-toc]')) {
    const t = document.getElementById(li.dataset.toc);
    const p = t && t.closest('.page');
    li.querySelector('.pg').textContent = p ? p.dataset.n : '';
  }
  for (const b of document.querySelectorAll('.page .body')) {
    if (b.scrollHeight > b.clientHeight + 1) document.title += ' [DEBORDEMENT p.' + b.parentNode.dataset.n + ']';
  }
})();
"""


def md(s):
    s = html.escape(s, quote=False)
    s = re.sub(r"\*\*(.+?)\*\*", r"<b>\1</b>", s)
    return re.sub(r"\*(.+?)\*", r"<i>\1</i>", s)


class Book:
    def __init__(self, lang, atlas, img_dir):
        self.lang, self.atlas, self.img_dir = lang, atlas, img_dir
        self.t = mt.T[lang]
        self.L = lang_table(lang)
        self.parts = []
        self.toc = []
        self.saved = {}

    def get(self, key):
        return self.L.get(key, key)

    # images
    def save(self, name, img, scale=PIX):
        if img is None:
            return None
        if name not in self.saved:
            big = img.resize((img.width * scale, img.height * scale), Image.NEAREST)
            big.save(self.img_dir / f"{name}.png")
            self.saved[name] = (img.width, img.height)
        return name

    def px(self, name, img, size_mm_per_px=0.55, max_mm=None):
        """Image de pixels : chaque pixel du jeu fait `size_mm_per_px` mm (reduite si plus grande que max_mm)."""
        if not self.save(name, img):
            return ""
        w, h = self.saved[name]
        k = size_mm_per_px
        if max_mm and max(w, h) * k > max_mm:
            k = max_mm / max(w, h)
        return f'<img class="px" src="img/{name}.png" style="width:{w * k:.2f}mm;height:{h * k:.2f}mm" alt="">'

    def sprite(self, sprite, prefs=None, max_mm=9.5, k=0.55):
        img = self.atlas.pick(sprite, prefs) if prefs else self.atlas.pick(sprite)
        return self.px(f"s_{sprite}", img, k, max_mm)

    def icon(self, icon, max_mm=8.5):
        return self.px(f"i_{icon}", self.atlas.frame(icon, "0") or self.atlas.pick(icon), 0.5, max_mm)

    def title_img(self, name, text, color=(216, 40, 46), height_mm=8.6):
        img = self.atlas.text(text, color)
        self.save(f"t_{name}", img)
        w, h = self.saved[f"t_{name}"]
        return f'<img class="px" src="img/t_{name}.png" style="height:{height_mm:.2f}mm;width:{w * height_mm / h:.2f}mm" alt="{html.escape(text)}">'

    # blocs
    def add(self, h, cls=""):
        self.parts.append(f'<div class="{cls}">{h}</div>' if cls else f"<div>{h}</div>")

    def chapter(self, key, title):
        self.toc.append((key, title))
        self.add(f'<div class="chapter" id="ch_{key}">{self.title_img(key, title)}</div>', "newpage")

    def bar(self, title, first=""):
        # un bandeau reste avec ce qui le suit (jamais seul en bas de page)
        self.add(f'<div class="bar">{html.escape(title)}</div>{first}', "keep")

    def para(self, s):
        self.add(f"<p>{md(s)}</p>")

    def item(self, ic, label, text, extra="", cls=""):
        lbl = f'<span class="lbl {cls}">{html.escape(label)}</span>' if label else ""
        self.add(f'<div class="item"><div class="ic">{ic}</div><div class="txt">{lbl}<p>{md(text)}</p>{extra}</div></div>')

    def cell(self, ic, title, text):
        self.parts.append(f'<div class="cell"><div class="ic">{ic}</div><div><b>{html.escape(title)}</b><span>{md(text)}</span></div></div>')

    def figure(self, shot, caption=""):
        f = self.img_dir.parent / "shots" / f"{shot}.png"
        if not f.exists():
            return
        shutil.copy(f, self.img_dir / f"shot_{shot}.png")
        w, h = Image.open(f).size
        cap = f"<figcaption>{md(caption)}</figcaption>" if caption else ""
        # taille connue avant le chargement : la mise en page mesure juste
        self.add(f'<figure><img src="img/shot_{shot}.png" width="{w}" height="{h}" alt="">{cap}</figure>')


def build(lang, atlas):
    d = OUT / lang
    img_dir = d / "img"
    img_dir.mkdir(parents=True, exist_ok=True)
    b = Book(lang, atlas, img_dir)
    t, L = b.t, b.L
    v = rd.VERSION
    W, P, C, E, S, M = weapons(), passives(), characters(), enemies(), stages(), meta()
    wname = {w["id"]: b.get(f"weapon.{w['id']}.name") for w in W}
    pname = {p["id"]: b.get(f"passive.{p['id']}.name") for p in P}
    wicon = {w["id"]: w["icon"] for w in W}
    picon = {p["id"]: p["icon"] for p in P}
    ename = lambda e: b.get(f"enemy.{e}")  # noqa: E731
    sname = lambda s: b.get(f"map.{s}.name")  # noqa: E731

    # couverture
    logo = b.px("logo", atlas.frame("title_logo", "0"), 1.0, 62)
    name1 = b.title_img("cover1", "A Survivor", (240, 200, 90), 9)
    name2 = b.title_img("cover2", "to the Past", (240, 200, 90), 9)
    row = "".join(b.sprite(s, max_mm=14, k=0.8) for s in ("soldier_green", "moblin", "link", "lynel", "stalfos"))
    b.add(f'<div class="cover"><div class="frame"></div><div class="frame2"></div><div class="logo">{logo}</div>'
          f'<div class="name">{name1}{name2}</div><div class="band">{t["booklet"]}</div>'
          f'<div class="ver">{t["version"].format(v=v)}</div><div class="row">{row}</div>'
          f'<div class="note">{t["cover_note"]}</div></div>', "fullpage")

    # avant de jouer
    b.add(f'<div class="chapter">{b.title_img("notice", t["notice_h"])}</div>', "newpage")
    for s in t["notice"]:
        b.para(s.format(v=v))
    b.bar(t["needs_h"])
    for s in t["needs"]:
        b.para(s)

    # sommaire (rempli apres la mise en page)
    toc_at = len(b.parts)
    b.parts.append("")

    # legende
    b.chapter("legend", t["legend_h"])
    for s in t["legend"]:
        b.para(s)
    b.add(f'<div class="sprites">{b.sprite("link", max_mm=13, k=0.75)}{b.sprite("agahnim", max_mm=16, k=0.75)}'
          f'{b.sprite("ganon", max_mm=24, k=0.6)}</div><div class="quote">{md(t["legend_quote"])}</div>')

    # 1. avant de commencer
    b.chapter("start", t["ch_start"])
    b.bar(t["install_h"])
    for s in t["install"]:
        b.para(s)
    b.bar(t["title_h"])
    b.figure("title")
    b.para(t["title_txt"])
    for key, desc in t["menu"]:
        label = b.get(key).replace("{s}", "LINK")
        b.item("", label, desc)
    b.bar(t["players_h"])
    b.figure("players")
    for s in t["players"]:
        b.para(s)
    b.bar(t["name_h"])
    for s in t["name"]:
        b.para(s)

    # 2. commandes
    b.chapter("controls", t["ch_controls"])
    b.para(t["controls_intro"])
    rows = "".join(f"<tr><td><b>{html.escape(a)}</b></td><td>{html.escape(k)}</td><td>{html.escape(m)}</td><td>{html.escape(p)}</td></tr>"
                   for a, k, m, p in t["controls"])
    head = "".join(f"<th>{html.escape(c)}</th>" for c in t["controls_cols"])
    b.add(f"<table><tr>{head}</tr>{rows}</table>")
    b.para(t["controls_note"])
    b.para(t["touch_note"])

    # 3. ecran de jeu
    b.chapter("screen", t["ch_screen"])
    b.figure("hud")
    b.para(t["screen_intro"])
    for name, desc in t["hud"]:
        b.item("", name, desc)

    # 4. deroulement
    b.chapter("run", t["ch_run"])
    for s in t["run"]:
        b.para(s)
    b.figure("boss")
    b.bar(t["levelup_h"])
    b.figure("levelup")
    for s in t["levelup"]:
        b.para(s)
    b.bar(t["chest_h"])
    for s in t["chest"]:
        b.para(s)
    b.bar(t["drops_h"])
    drop_icons = {"rupee": "rupee_red", "heart": "heart_small", "fairy": "fairy", "magnet": "pickup_net",
                  "medallion": "pickup_medallion", "bomb": "bobomb"}
    for key, desc in t["drops"]:
        spr = drop_icons[key]
        b.item(b.sprite(spr, max_mm=8) if spr in atlas.sprites else "", "", desc)
    b.para(t["drops_note"])
    b.bar(t["shots_h"])
    b.para(t["shots"])
    b.bar(t["encounters_h"])
    b.para(t["champion"])
    b.figure("thief")
    b.item(b.sprite("boo", max_mm=9), ename("boo"), t["thief"])

    # 5. armes et objets
    b.chapter("gear", t["ch_gear"])
    b.para(t["gear_intro"])
    b.bar(t["weapons_h"])
    for w in W:
        if not w["evolved"]:
            b.item(b.icon(w["icon"]), wname[w["id"]], b.get(f"weapon.{w['id']}.desc"))
    b.bar(t["passives_h"])
    for p in P:
        b.cell(b.icon(p["icon"]), pname[p["id"]], b.get(f"passive.{p['id']}.desc"))
    b.bar(t["evos_h"])
    b.para(t["evos_intro"])
    ic = lambda icon: b.px(f"i_{icon}", atlas.frame(icon, "0") or atlas.pick(icon), 0.32, 5.2)  # noqa: E731
    rows = ""
    for w in W:
        if w["evo"]:
            p, into = w["evo"]
            rows += (f"<tr><td>{ic(w['icon'])}{html.escape(wname[w['id']])}</td><td>{ic(picon[p])}{html.escape(pname[p])}</td>"
                     f"<td>{ic(wicon[into])}<b>{html.escape(wname[into])}</b></td></tr>")
    head = "".join(f"<th>{html.escape(c)}</th>" for c in t["evos_cols"])
    b.add(f"<table><tr>{head}</tr>{rows}</table>")
    for w in W:
        if w["evolved"]:
            b.cell(b.icon(w["icon"]), wname[w["id"]], b.get(f"weapon.{w['id']}.desc"))

    # 6. personnages
    b.chapter("chars", t["ch_chars"])
    b.para(t["chars_intro"])
    b.figure("chars")
    for c in C:
        u = c["unlock"]
        if u is None:
            hint = t["unlocked"]
        else:
            kind, arg = u
            key = {"Win": "ui.unlock_win", "Kills": "ui.unlock_kills", "Bosses": "ui.unlock_bosses",
                   "Level": "ui.unlock_level", "Slain": "ui.unlock_slain"}[kind]
            val = sname(arg) if kind == "Win" else (ename(arg) if kind == "Slain" else arg)
            hint = b.get(key).replace("{s}", val).replace("{n}", val)
        start = f'<p class="muted">{html.escape(wname.get(c["weapon"], ""))} - {md(hint)}</p>'
        b.item(b.sprite(c["sprite"], ("idle_down", "walk_down"), max_mm=11, k=0.6), b.get(f"char.{c['id']}.name"),
               b.get(f"char.{c['id']}.desc"), start, "blue")

    # 7. stages
    b.chapter("stages", t["ch_stages"])
    b.para(t["stages_intro"])
    b.figure("stages")
    relic_of = {r["stage"]: r for r in M["relics"] if r["stage"]}
    for s in S:
        bits = []
        for lbl, e in ((t["mid_boss"], s["mid"]), (t["final_boss"], s["final"])):
            if e:
                bits.append(f'<td>{b.sprite(e, max_mm=9, k=0.35)}</td><td><span class="muted">{html.escape(lbl)}</span><br><b>{html.escape(ename(e))}</b></td>')
        r = relic_of.get(s["id"])
        if r:
            bits.append(f'<td>{b.icon(r["icon"], 6)}</td><td><span class="muted">{html.escape(t["relic"])}</span><br>'
                        f'<b>{html.escape(b.get("relic." + r["id"] + ".name"))}</b></td>')
        tbl = f'<table style="margin:.6mm 0 0"><tr>{"".join(bits)}</tr></table>'
        b.item("", f'{s["order"]}. {sname(s["id"])}', b.get(f"map.{s['id']}.desc"), tbl, "gold")

    # 8. entre les parties
    b.chapter("meta", t["ch_meta"])
    b.para(t["meta_intro"])
    b.figure("shop")
    b.bar(t["shop_h"])
    b.para(t["shop_intro"])
    for it in M["shop"]:
        b.cell(b.icon(it["icon"]), b.get(f"shop.{it['id']}.name"), b.get(f"shop.{it['id']}.desc"))
    b.bar(t["fairy_h"])
    b.para(t["fairy_intro"])
    for it in M["blessings"]:
        b.cell(b.icon(it["icon"]), b.get(f"blessing.{it['id']}.name"), b.get(f"blessing.{it['id']}.desc"))
    b.bar(t["relics_h"])
    b.para(t["relics_intro"])
    for it in M["relics"]:
        desc = ", ".join(stat_text(L, s, x) for s, x in it["stats"])
        b.cell(b.icon(it["icon"]), b.get(f"relic.{it['id']}.name"), desc)
    b.bar(t["seals_h"])
    b.para(t["seals_intro"])
    for it in M["seals"]:
        bonus = f" (+{int(float(it['bonus']) * 100)} %)" if it["bonus"] else ""
        b.cell(b.icon(it["icon"]), b.get(f"seal.{it['id']}.name") + bonus, b.get(f"seal.{it['id']}.desc"))
    b.bar(t["diff_h"])
    b.para(t["diff_intro"])
    for it in M["difficulties"]:
        b.item("", b.get(f"difficulty.{it['id']}.name"), b.get(f"difficulty.{it['id']}.desc"))
    b.bar(t["profile_h"])
    b.para(t["profile"])

    # 9. score et classements
    b.chapter("scores", t["ch_scores"])
    b.bar(t["score_h"])
    for s in t["score"]:
        b.para(s)
    b.bar(t["boards_h"])
    b.figure("scores")
    for key, desc in t["boards"]:
        b.item("", b.get(key), desc)
    b.bar(t["publish_h"])
    for s in t["publish"]:
        b.para(s)

    # 10. bestiaire
    b.chapter("bestiary", t["ch_bestiary"])
    b.para(t["bestiary_intro"])
    for e in sorted(E, key=lambda e: (e["boss"], e["hp"])):
        stats = f'{t["stat_hp"]} {e["hp"]:g} - {t["stat_dmg"]} {e["damage"]:g} - {t["stat_speed"]} {e["speed"]:g}'
        title = ename(e["id"]) + (f' ({t["boss_tag"]})' if e["boss"] else "")
        b.cell(b.sprite(e["sprite"], max_mm=11, k=0.45), title, stats)

    # 11. conseils
    b.chapter("tips", t["ch_tips"])
    for s in t["tips"]:
        b.add(f'<div class="tip"><div>{md(s)}</div></div>')
    b.add(f'<div class="sprites">{b.sprite("link", max_mm=13, k=0.75)}{b.sprite("fairy", max_mm=9, k=0.75)}</div>')

    # dos
    back_title = b.title_img("back", t["back_h"], (240, 200, 90), 8)
    lines = "".join(f"<p>{md(s.format(site=rd.SITE.rstrip('/'), discord=rd.DISCORD, repo=rd.REPO))}</p>" for s in t["back"])
    b.add(f'<div class="back">{back_title}{lines}<div class="tri">{b.sprite("link", max_mm=16, k=1)}{b.sprite("fairy", max_mm=10, k=1)}</div>'
          f'<div class="cr">{md(t["back_credits"])}</div></div>', "fullpage")

    # sommaire
    items = "".join(f'<li data-toc="ch_{k}"><b>{html.escape(name)}</b><span class="dots"></span><span class="pg"></span></li>'
                    for k, name in b.toc)
    b.parts[toc_at] = (f'<div class="newpage"><div class="chapter">{b.title_img("toc", t["toc_h"])}</div>'
                       f'<ul class="toc">{items}</ul></div>')

    page = (f'<!doctype html><html lang="{lang}"><head><meta charset="utf-8"><title>A Survivor to the Past - {t["file"]}</title>'
            f"<style>{CSS}</style></head><body><div id=\"book\"></div><div id=\"flow\">{''.join(b.parts)}</div>"
            f"<script>{PAGINATE}</script></body></html>")
    f = d / "manual.html"
    f.write_text(page, encoding="utf-8")
    return f


def to_pdf(src, pdf):
    exe = next((p for p in BROWSERS if p.exists()), None)
    if not exe:
        sys.exit("Microsoft Edge ou Google Chrome introuvable")
    pdf.parent.mkdir(parents=True, exist_ok=True)
    if pdf.exists():
        pdf.unlink()
    profile = Path(tempfile.mkdtemp(prefix="astp_pdf_"))
    try:
        subprocess.run([str(exe), "--headless", "--disable-gpu", "--no-pdf-header-footer", f"--user-data-dir={profile}",
                        "--virtual-time-budget=10000", f"--print-to-pdf={pdf}", src.resolve().as_uri()],
                       capture_output=True, timeout=180)
    finally:
        shutil.rmtree(profile, ignore_errors=True)
    if not pdf.exists():
        sys.exit(f"PDF non cree : {pdf}")


def check_overflow(src):
    """La mise en page signale les pages qui debordent dans le titre du document (verifie a l'impression)."""
    exe = next((p for p in BROWSERS if p.exists()), None)
    profile = Path(tempfile.mkdtemp(prefix="astp_dom_"))
    try:
        r = subprocess.run([str(exe), "--headless", "--disable-gpu", f"--user-data-dir={profile}", "--virtual-time-budget=10000",
                            "--dump-dom", src.resolve().as_uri()], capture_output=True, timeout=180)
    finally:
        shutil.rmtree(profile, ignore_errors=True)
    dom = r.stdout.decode("utf-8", "replace")
    m = re.search(r"<title>(.*?)</title>", dom)
    pages = dom.count('class="page')
    return (m.group(1) if m else ""), pages


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--lang", default="fr,en")
    ap.add_argument("--no-shots", action="store_true", help="garder les captures deja prises")
    a = ap.parse_args()
    atlas = Atlas()
    for lang in a.lang.split(","):
        shots = OUT / lang / "shots"
        if not a.no_shots:
            if not EXE.exists():
                sys.exit("jeu absent : cargo build --release -p game")
            if shots.exists():
                shutil.rmtree(shots)
            capture(lang, shots)
        img = OUT / lang / "img"
        if img.exists():
            shutil.rmtree(img)
        src = build(lang, atlas)
        title, pages = check_overflow(src)
        if "DEBORDEMENT" in title:
            print("ATTENTION :", title)
        pdf = OUT / f"A-Survivor-to-the-Past-{mt.T[lang]['file']}-{lang}.pdf"
        to_pdf(src, pdf)
        print(f"ok : {pdf.relative_to(ROOT)} ({pages} pages)")


if __name__ == "__main__":
    main()

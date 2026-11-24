"""Site de presentation (Cloudflare Pages) : site/ genere a partir des textes du README.

Page statique, 7 langues (anglais dans le HTML, les autres via i18n.js), aucun traceur, aucune
ressource externe. Usage : python research/tools/make_site.py
"""
import html
import json
import re
import shutil
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import make_readme as rd  # noqa: E402

ROOT = rd.ROOT
OUT = ROOT / "site"

UI = {
    "en": dict(download="Download for Windows", soon=f"Alpha {rd.VERSION} for Windows — Linux, macOS, Android and browser later",
               discord="Join the Discord", github="Source code", lang="Language", nav_what="The game",
               nav_shots="Screenshots", nav_evo="Evolutions", nav_road="Roadmap", horde="A horde at 3:30"),
    "fr": dict(download="Télécharger pour Windows", soon=f"Alpha {rd.VERSION} pour Windows — Linux, macOS, Android et navigateur plus tard",
               discord="Rejoindre le Discord", github="Code source", lang="Langue", nav_what="Le jeu",
               nav_shots="Captures", nav_evo="Évolutions", nav_road="Feuille de route", horde="Une horde à 3:30"),
    "de": dict(download="Für Windows herunterladen", soon=f"Alpha {rd.VERSION} für Windows — Linux, macOS, Android und Browser folgen",
               discord="Zum Discord", github="Quellcode", lang="Sprache", nav_what="Das Spiel",
               nav_shots="Bilder", nav_evo="Entwicklungen", nav_road="Fahrplan", horde="Eine Horde bei 3:30"),
    "es": dict(download="Descargar para Windows", soon=f"Alfa {rd.VERSION} para Windows — Linux, macOS, Android y navegador más adelante",
               discord="Únete al Discord", github="Código fuente", lang="Idioma", nav_what="El juego",
               nav_shots="Capturas", nav_evo="Evoluciones", nav_road="Hoja de ruta", horde="Una horda a los 3:30"),
    "it": dict(download="Scarica per Windows", soon=f"Alfa {rd.VERSION} per Windows — Linux, macOS, Android e browser più avanti",
               discord="Entra nel Discord", github="Codice sorgente", lang="Lingua", nav_what="Il gioco",
               nav_shots="Immagini", nav_evo="Evoluzioni", nav_road="Tabella di marcia", horde="Un'orda a 3:30"),
    "pt": dict(download="Baixar para Windows", soon=f"Alfa {rd.VERSION} para Windows — Linux, macOS, Android e navegador depois",
               discord="Entrar no Discord", github="Código-fonte", lang="Idioma", nav_what="O jogo",
               nav_shots="Imagens", nav_evo="Evoluções", nav_road="Próximos passos", horde="Uma horda aos 3:30"),
    "ja": dict(download="Windows 版をダウンロード", soon=f"Windows 版アルファ {rd.VERSION} — Linux・macOS・Android・ブラウザ版は今後",
               discord="Discord に参加", github="ソースコード", lang="言語", nav_what="ゲーム",
               nav_shots="スクリーンショット", nav_evo="進化", nav_road="今後の予定", horde="3:30 の大群"),
}

SHOTS = ["title", "levelup", "evolution", "boss", "options", "about"]


def md(s):
    """Markdown minimal du README -> HTML (gras, italique, liens, code)."""
    s = html.escape(s, quote=False)
    s = re.sub(r"\[([^\]]+)\]\(([^)]+)\)", lambda m: f'<a href="{m.group(2)}" rel="noopener">{m.group(1)}</a>', s)
    s = re.sub(r"\*\*(.+?)\*\*", r"<strong>\1</strong>", s)
    s = re.sub(r"\*(.+?)\*", r"<em>\1</em>", s)
    s = re.sub(r"`(.+?)`", r"<code>\1</code>", s)
    # le lien vers LICENSE pointe vers le depot
    s = s.replace('href="LICENSE"', f'href="{rd.REPO}/blob/main/LICENSE"')
    return s


def paras(s):
    return "".join(f"<p>{md(p)}</p>" for p in s.split("\n\n"))


def strings(code):
    t, u = rd.T[code], UI[code]
    name = rd.lang_names(code)
    cols = t["evo_cols"]
    evo = (f"<thead><tr><th>{md(cols[0])}</th><th>{md(cols[1])}</th><th>{md(cols[2])}</th></tr></thead><tbody>"
           + "".join(f"<tr><td>{html.escape(name('weapon', w))}</td><td>{html.escape(name('passive', p))}</td>"
                     f"<td><strong>{html.escape(name('weapon', e))}</strong></td></tr>" for w, p, e in rd.EVOS)
           + "</tbody>")
    ccols = t["ctrl_cols"]
    ctrl = (f"<thead><tr>{''.join(f'<th>{md(c)}</th>' for c in ccols)}</tr></thead><tbody>"
            + "".join(f"<tr><td>{md(a)}</td><td>{md(k)}</td><td>{md(p)}</td></tr>" for a, k, p in t["ctrl"])
            + "</tbody>")
    play = t["play"].split("\n\n")[0]
    s = {
        "tagline": t["tagline"],
        "what_h": md(t["what_h"]), "what": paras(t["what"]),
        "feat_h": md(t["feat_h"]), "feats": "".join(f"<li>{md(f)}</li>" for f in t["feats"]),
        "evo_h": md(t["evo_h"]), "evo": evo,
        "shots_h": md(t["shots_h"]), "filters_cap": md(t["filters_cap"]),
        "play_h": md(t["play_h"]), "play": paras(play),
        "ctrl_h": md(t["ctrl_h"]), "ctrl": ctrl,
        "road_h": md(t["road_h"]), "road": "".join(f"<li>{md(r)}</li>" for r in t["road"]),
        "comm_h": md(t["comm_h"]), "comm": paras(t["comm"].split("\n\n")[0]),
        "legal_h": md(t["legal_h"]), "legal": paras(t["legal"]),
        "horde": md(u["horde"]),
    }
    for k, v in u.items():
        if k != "horde":
            s["ui_" + k] = md(v)
    for k in SHOTS:
        s["cap_" + k] = md(t["caps"][k])
    return s


PAGE = """<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>A Survivor to the Past</title>
<meta name="description" content="{desc}">
<meta name="theme-color" content="#14143a">
<meta name="referrer" content="no-referrer">
<meta property="og:type" content="website">
<meta property="og:title" content="A Survivor to the Past">
<meta property="og:description" content="{desc}">
<meta property="og:url" content="{site}">
<meta property="og:image" content="{site}img/horde.png">
<meta name="twitter:card" content="summary_large_image">
<link rel="icon" href="favicon.ico" sizes="16x16 32x32 48x48">
<link rel="icon" type="image/png" href="img/icon_192.png" sizes="192x192">
<link rel="apple-touch-icon" href="img/icon_192.png">
<link rel="stylesheet" href="style.css">
</head>
<body>
<header class="top">
  <a class="brand" href="#"><img src="img/icon_64.png" width="32" height="32" alt=""><span>A Survivor to the Past</span></a>
  <nav>
    <a href="#game" data-i="ui_nav_what">{ui_nav_what}</a>
    <a href="#shots" data-i="ui_nav_shots">{ui_nav_shots}</a>
    <a href="#evo" data-i="ui_nav_evo">{ui_nav_evo}</a>
    <a href="#road" data-i="ui_nav_road">{ui_nav_road}</a>
  </nav>
  <label class="lang"><span class="sr" data-i="ui_lang">{ui_lang}</span>
    <select id="lang">{lang_options}</select>
  </label>
</header>

<main>
<section class="hero">
  <img class="shot pix" src="img/title.png" width="1280" height="720" alt="A Survivor to the Past">
  <p class="tagline" data-i="tagline">{tagline}</p>
  <div class="cta">
    <a class="btn gold" href="{releases}/latest" rel="noopener" data-i="ui_download">{ui_download}</a>
    <a class="btn discord" href="{discord}" rel="noopener" data-i="ui_discord">{ui_discord}</a>
    <a class="btn" href="{repo}" rel="noopener" data-i="ui_github">{ui_github}</a>
  </div>
  <p class="soon" data-i="ui_soon">{ui_soon}</p>
</section>

<section id="game" class="panel">
  <h2 data-i="what_h">{what_h}</h2>
  <div data-i="what">{what}</div>
  <figure><img class="shot pix" src="img/horde.png" width="1280" height="720" alt="" loading="lazy">
    <figcaption data-i="horde">{horde}</figcaption></figure>
  <h2 data-i="feat_h">{feat_h}</h2>
  <ul class="feats" data-i="feats">{feats}</ul>
</section>

<section id="shots" class="panel">
  <h2 data-i="shots_h">{shots_h}</h2>
  <div class="grid">{gallery}</div>
  <figure><img class="shot" src="img/filters.png" width="1280" height="1080" alt="" loading="lazy">
    <figcaption data-i="filters_cap">{filters_cap}</figcaption></figure>
</section>

<section id="evo" class="panel">
  <h2 data-i="evo_h">{evo_h}</h2>
  <div class="scroll"><table data-i="evo">{evo}</table></div>
  <h2 data-i="ctrl_h">{ctrl_h}</h2>
  <div class="scroll"><table data-i="ctrl">{ctrl}</table></div>
</section>

<section id="road" class="panel">
  <h2 data-i="play_h">{play_h}</h2>
  <div data-i="play">{play}</div>
  <h2 data-i="road_h">{road_h}</h2>
  <ul data-i="road">{road}</ul>
  <h2 data-i="comm_h">{comm_h}</h2>
  <div data-i="comm">{comm}</div>
  <p class="cta"><a class="btn discord" href="{discord}" rel="noopener" data-i="ui_discord">{ui_discord}</a></p>
</section>
</main>

<footer class="legal">
  <h2 data-i="legal_h">{legal_h}</h2>
  <div data-i="legal">{legal}</div>
  <p class="small">A Survivor to the Past {version} · <a href="{repo}" rel="noopener">GitHub</a> · <a href="{discord}" rel="noopener">Discord</a></p>
</footer>
<script src="i18n.js"></script>
<script src="app.js"></script>
</body>
</html>
"""

CSS = """:root {
  --bg: #0e0e2a; --bg2: #16164a; --panel: #1c1c58; --ink: #eef0ff; --dim: #aab0e0;
  --gold: #f8d040; --gold2: #b08010; --frame: #d8c070; --line: #2c2c7a; --discord: #5865f2;
  color-scheme: dark;
}
* { box-sizing: border-box; }
html { scroll-behavior: smooth; }
body {
  margin: 0; background: var(--bg); color: var(--ink);
  font: 16px/1.6 system-ui, -apple-system, "Segoe UI", Roboto, "Noto Sans", "Hiragino Sans", "Yu Gothic", sans-serif;
  background-image: radial-gradient(ellipse at top, #22226a 0%, var(--bg) 60%);
  background-attachment: fixed;
}
a { color: var(--gold); }
a:hover { color: #fff3b0; }
.sr { position: absolute; width: 1px; height: 1px; overflow: hidden; clip: rect(0 0 0 0); }
.top {
  position: sticky; top: 0; z-index: 10; display: flex; align-items: center; gap: 16px;
  padding: 8px 16px; background: rgba(14, 14, 42, .92); border-bottom: 2px solid var(--line);
  backdrop-filter: blur(6px);
}
.brand { display: flex; align-items: center; gap: 10px; color: var(--ink); text-decoration: none; font-weight: 700; white-space: nowrap; }
.brand img, .pix { image-rendering: pixelated; }
.top nav { display: flex; gap: 14px; flex: 1; justify-content: center; flex-wrap: wrap; }
.top nav a { color: var(--dim); text-decoration: none; font-size: 15px; }
.top nav a:hover { color: var(--gold); }
.lang select {
  background: var(--panel); color: var(--ink); border: 2px solid var(--frame); border-radius: 4px;
  padding: 4px 8px; font: inherit; font-size: 14px;
}
main { max-width: 1040px; margin: 0 auto; padding: 24px 16px 8px; }
section { scroll-margin-top: 72px; }
.hero { text-align: center; }
.shot { display: block; width: 100%; height: auto; border: 3px solid var(--frame); border-radius: 6px;
  box-shadow: 0 0 0 3px #000, 0 12px 40px rgba(0, 0, 0, .5); background: #000; }
.tagline { font-size: 20px; margin: 22px auto 8px; max-width: 760px; }
.tagline b { color: var(--gold); }
.cta { display: flex; gap: 12px; justify-content: center; flex-wrap: wrap; margin: 18px 0 6px; }
.btn {
  display: inline-block; padding: 10px 20px; border-radius: 6px; text-decoration: none; font-weight: 700;
  color: var(--ink); background: var(--panel); border: 2px solid var(--frame); box-shadow: 0 3px 0 #000;
}
.btn:hover { transform: translateY(-1px); color: #fff; }
.btn.gold { background: linear-gradient(#f8d850, #d8a020); color: #2a1a00; border-color: #fff0a0; }
.btn.discord { background: var(--discord); border-color: #aab4ff; }
.soon { color: var(--dim); font-size: 14px; margin: 6px 0 0; }
.panel {
  margin: 36px 0; padding: 24px 28px; background: linear-gradient(var(--bg2), var(--panel));
  border: 3px solid var(--frame); border-radius: 8px; box-shadow: 0 0 0 3px #000, inset 0 0 0 2px #000;
}
h2 { color: var(--gold); margin: 26px 0 10px; font-size: 24px; text-shadow: 2px 2px 0 #000; }
.panel > h2:first-child { margin-top: 0; }
figure { margin: 22px 0 6px; }
figcaption { text-align: center; color: var(--dim); font-size: 14px; margin-top: 8px; }
.feats li, .panel ul li { margin: 6px 0; }
.feats strong, .panel strong { color: #fff3b0; }
.grid { display: grid; grid-template-columns: repeat(2, 1fr); gap: 18px; }
.grid figure { margin: 0; }
.scroll { overflow-x: auto; }
table { width: 100%; border-collapse: collapse; margin: 6px 0 4px; font-size: 15px; }
th, td { text-align: left; padding: 7px 10px; border-bottom: 1px solid var(--line); }
th { color: var(--gold); font-weight: 700; }
code { background: #000; padding: 1px 6px; border-radius: 4px; font-size: 14px; }
.legal { max-width: 1040px; margin: 10px auto 0; padding: 18px 16px 40px; color: var(--dim); font-size: 14px; border-top: 2px solid var(--line); }
.legal h2 { font-size: 18px; }
.legal strong { color: var(--ink); }
.small { font-size: 13px; margin-top: 18px; }
@media (max-width: 700px) {
  .top { flex-wrap: wrap; gap: 8px; }
  .top { justify-content: space-between; }
  .top nav { order: 3; flex: 1 1 100%; justify-content: flex-start; gap: 12px; }
  .grid { grid-template-columns: 1fr; }
  .panel { padding: 18px 16px; }
  .tagline { font-size: 18px; }
}
"""

APP = """(function () {
  var L = window.I18N || {};
  var sel = document.getElementById('lang');
  function pick() {
    try { var s = localStorage.getItem('lang'); if (s && L[s]) return s; } catch (e) {}
    var want = (navigator.languages || [navigator.language || 'en']);
    for (var i = 0; i < want.length; i++) {
      var c = String(want[i]).slice(0, 2).toLowerCase();
      if (L[c]) return c;
    }
    return 'en';
  }
  function apply(code) {
    var d = L[code]; if (!d) return;
    document.documentElement.lang = code;
    var els = document.querySelectorAll('[data-i]');
    for (var i = 0; i < els.length; i++) {
      var k = els[i].getAttribute('data-i');
      if (d[k] !== undefined) els[i].innerHTML = d[k];
    }
    var caps = document.querySelectorAll('img[data-alt]');
    for (var j = 0; j < caps.length; j++) {
      var a = d[caps[j].getAttribute('data-alt')];
      if (a !== undefined) caps[j].alt = a.replace(/<[^>]*>/g, '');
    }
    sel.value = code;
  }
  sel.addEventListener('change', function () {
    try { localStorage.setItem('lang', sel.value); } catch (e) {}
    apply(sel.value);
  });
  apply(pick());
})();
"""

HEADERS = """/*
  Content-Security-Policy: default-src 'none'; img-src 'self'; style-src 'self'; script-src 'self'; base-uri 'none'; form-action 'none'; frame-ancestors 'none'
  X-Content-Type-Options: nosniff
  X-Frame-Options: DENY
  Referrer-Policy: no-referrer
  Permissions-Policy: camera=(), microphone=(), geolocation=()
  Cross-Origin-Opener-Policy: same-origin
  Strict-Transport-Security: max-age=31536000

/img/*
  Cache-Control: public, max-age=604800
"""


def main():
    if OUT.exists():
        shutil.rmtree(OUT)
    (OUT / "img").mkdir(parents=True)
    for f in ["horde", "filters", *SHOTS]:
        shutil.copy(ROOT / f"media/{f}.png", OUT / f"img/{f}.png")
    for s in (64, 192):
        shutil.copy(ROOT / f"assets/icons/icon_{s}.png", OUT / f"img/icon_{s}.png")
    shutil.copy(ROOT / "assets/icons/favicon.ico", OUT / "favicon.ico")
    all_s = {c: strings(c) for c, _ in rd.LANGS}
    en = all_s["en"]
    gallery = "".join(
        f'<figure><img class="shot" src="img/{k}.png" width="1280" height="720" alt="{html.escape(rd.T["en"]["caps"][k])}" '
        f'data-alt="cap_{k}" loading="lazy"><figcaption data-i="cap_{k}">{en["cap_" + k]}</figcaption></figure>'
        for k in SHOTS)
    opts = "".join(f'<option value="{c}">{n}</option>' for c, n in rd.LANGS)
    desc = re.sub(r"<[^>]+>", "", rd.T["en"]["tagline"])
    page = PAGE.format(desc=html.escape(desc), site=rd.SITE, releases=f"{rd.REPO}/releases", discord=rd.DISCORD,
                       repo=rd.REPO, version=rd.VERSION, lang_options=opts, gallery=gallery, **en)
    (OUT / "index.html").write_text(page, encoding="utf-8", newline="\n")
    (OUT / "style.css").write_text(CSS, encoding="utf-8", newline="\n")
    (OUT / "app.js").write_text(APP, encoding="utf-8", newline="\n")
    (OUT / "i18n.js").write_text("window.I18N = " + json.dumps(all_s, ensure_ascii=False, indent=0) + ";\n",
                                 encoding="utf-8", newline="\n")
    (OUT / "_headers").write_text(HEADERS, encoding="utf-8", newline="\n")
    (OUT / "robots.txt").write_text("User-agent: *\nAllow: /\n", encoding="utf-8", newline="\n")
    print("ok :", OUT)


if __name__ == "__main__":
    main()

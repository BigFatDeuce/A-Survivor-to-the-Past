"""Version web jouable : dist/web/ (copiee dans site/play/ par make_site.py).

Contenu :
  game.js + game_bg.wasm   le jeu (wasm-bindgen, cible web)
  game.pak                 data/ (sans data/sprites), assets/gen/, assets/audio/sfx/ en un seul fichier
  assets/audio/music/      les musiques utilisees (lues en streaming par le navigateur)
  index.html, play.js, play.css   page de lancement (telechargement, puis clic pour jouer)

Usage : python research/tools/make_web.py [--no-build]
Necessite la cible wasm32-unknown-unknown et wasm-bindgen-cli de la meme version que la crate.
"""
import argparse
import json
import re
import shutil
import struct
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "dist" / "web"
VERSION = re.search(r'version\s*=\s*"([^"]+)"', (ROOT / "Cargo.toml").read_text(encoding="utf-8")).group(1)
WASM = ROOT / "target/wasm32-unknown-unknown/release/a_survivor_to_the_past.wasm"


def pak_files():
    files = []
    for p in sorted((ROOT / "data").rglob("*")):
        rel = p.relative_to(ROOT).as_posix()
        if p.is_file() and not rel.startswith("data/sprites/") and p.suffix == ".ron":
            files.append(rel)
    meta = (ROOT / "assets/gen/atlas.ron").read_text(encoding="utf-8")
    image = re.search(r'image:\s*"([^"]+)"', meta).group(1)
    files += ["assets/gen/atlas.ron", f"assets/gen/{image}"]
    for p in sorted((ROOT / "assets/audio/sfx").iterdir()):
        if p.suffix in (".ogg", ".wav"):
            files.append(p.relative_to(ROOT).as_posix())
    return files


def write_pak(files, dest):
    with open(dest, "wb") as f:
        f.write(b"ASTP" + struct.pack("<II", 1, len(files)))
        for rel in files:
            data = (ROOT / rel).read_bytes()
            name = rel.encode("utf-8")
            f.write(struct.pack("<H", len(name)) + name + struct.pack("<I", len(data)) + data)
    return dest.stat().st_size


# Ecran de chargement : sprites tires de l'atlas du jeu (A Link to the Past), assembles en une bande.
LOADER = [
    ("link", "walk_side"), ("link", "idle_side"), ("link", "idle_down"),
    ("soldier_green", "walk_side"), ("octorok", "walk_side"), ("moblin", "walk_side"),
    ("bobomb", "walk"), ("bobomb", "red"), ("ganon", "walk_side"), ("rupee_green", "0"),
]


def build_loader(dest):
    """Ecrit loader.png (bande d'images) et renvoie {"sprite.anim": {"fps", "frames": [[x, y, w, h, px, py]]}}."""
    from PIL import Image
    text = (ROOT / "assets/gen/atlas.ron").read_text(encoding="utf-8")
    atlas = Image.open(ROOT / "assets/gen" / re.search(r'image:\s*"([^"]+)"', text).group(1)).convert("RGBA")
    out, crops, x = {}, [], 0
    for sprite, anim in LOADER:
        block = re.search(r'\n        "%s": \{(.*?)\n        \},' % re.escape(sprite), text, re.S).group(1)
        m = re.search(r'"%s": \(frames: \[(.*?)\](?:, fps: ([\d.]+))?' % re.escape(anim), block)
        frames = []
        for f in re.findall(r'\(x: (\d+), y: (\d+), w: (\d+), h: (\d+), px: (-?\d+), py: (-?\d+)\)', m.group(1)):
            fx, fy, w, h, px, py = map(int, f)
            crops.append((atlas.crop((fx, fy, fx + w, fy + h)), x))
            frames.append([x, 0, w, h, px, py])
            x += w + 1
        out[f"{sprite}.{anim}"] = {"fps": float(m.group(2) or 8), "frames": frames}
    strip = Image.new("RGBA", (x, max(c.height for c, _ in crops)), (0, 0, 0, 0))
    for c, at in crops:
        strip.paste(c, (at, 0))
    strip.save(dest, optimize=True)
    return out


def music_files():
    text = (ROOT / "data/audio.ron").read_text(encoding="utf-8")
    return sorted(set(re.findall(r'file:\s*"([^"]+)"', text)))


PAGE = """<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>A Survivor to the Past</title>
<meta name="description" content="Play A Survivor to the Past in your browser.">
<link rel="icon" href="../favicon.ico">
<link rel="stylesheet" href="play.css?v={v}">
</head>
<body>
<canvas id="game" tabindex="0"></canvas>
<div id="overlay">
  <img src="../img/icon_192.png" width="96" height="96" alt="">
  <h1>A Survivor to the Past</h1>
  <canvas id="scene" width="220" height="60" aria-hidden="true"></canvas>
  <p id="status">Loading...</p>
  <div id="bar"><div id="fill"></div></div>
  <p id="tip"></p>
  <button id="start" hidden>Play</button>
  <p id="note"></p>
  <p class="small"><a href="../">a-survivor-to-the-past</a> · v{v}</p>
</div>
<script type="module" src="play.js?v={v}"></script>
</body>
</html>
"""

CSS = """html, body { margin: 0; height: 100%; background: #000; overflow: hidden; }
canvas#game { display: block; width: 100vw; height: 100vh; outline: none; touch-action: none; }
#overlay { position: fixed; inset: 0; display: flex; flex-direction: column; align-items: center; justify-content: center;
  gap: 12px; background: radial-gradient(#1c1838, #07060f); color: #e8e4ff; font: 16px/1.4 system-ui, sans-serif;
  text-align: center; padding: 16px; box-sizing: border-box; }
#overlay[hidden] { display: none; }
#overlay img { image-rendering: pixelated; }
h1 { margin: 0; font-size: 28px; color: #f8d858; letter-spacing: .5px; }
#scene { width: min(540px, 92vw); aspect-ratio: 11 / 3; image-rendering: pixelated; }
#tip { min-height: 1.4em; margin: 0; font-style: italic; color: #b8b0e8; max-width: 520px; }
#bar { width: min(360px, 80vw); height: 10px; border: 2px solid #f8d858; border-radius: 3px; }
#fill { width: 0; height: 100%; background: #f8d858; transition: width .15s; }
#start { font: 700 20px system-ui, sans-serif; color: #10102a; background: #f8d858; border: 0; border-radius: 6px;
  padding: 12px 36px; cursor: pointer; }
#start:hover, #start:focus { background: #fff0a0; outline: none; }
#note { max-width: 520px; opacity: .85; margin: 0; }
.small { font-size: 13px; opacity: .6; margin: 0; }
.small a { color: inherit; }
.error { color: #ff8a80; white-space: pre-wrap; }
"""

JS = """// Page de lancement : telecharge le paquet du jeu, puis demarre au clic (le son demande un clic).
import init from './game.js?v={v}';

const T = {
  en: { load: 'Loading...', ready: 'Ready!', play: 'Play', err: 'The game could not start.',
        note: 'Keyboard, mouse or gamepad. Best on a computer, in Chrome, Edge or Firefox. Your progress is saved in this browser.' },
  fr: { load: 'Chargement...', ready: 'Prêt !', play: 'Jouer', err: "Le jeu n'a pas pu démarrer.",
        note: 'Clavier, souris ou manette. Idéal sur ordinateur, dans Chrome, Edge ou Firefox. Votre progression est enregistrée dans ce navigateur.' },
  de: { load: 'Wird geladen...', ready: 'Bereit!', play: 'Spielen', err: 'Das Spiel konnte nicht starten.',
        note: 'Tastatur, Maus oder Controller. Am besten am Computer, in Chrome, Edge oder Firefox. Dein Fortschritt wird in diesem Browser gespeichert.' },
  es: { load: 'Cargando...', ready: '¡Listo!', play: 'Jugar', err: 'El juego no pudo iniciarse.',
        note: 'Teclado, ratón o mando. Mejor en ordenador, en Chrome, Edge o Firefox. Tu progreso se guarda en este navegador.' },
  it: { load: 'Caricamento...', ready: 'Pronto!', play: 'Gioca', err: 'Il gioco non è riuscito ad avviarsi.',
        note: 'Tastiera, mouse o controller. Meglio su computer, con Chrome, Edge o Firefox. I progressi sono salvati in questo browser.' },
  pt: { load: 'Carregando...', ready: 'Pronto!', play: 'Jogar', err: 'O jogo não conseguiu iniciar.',
        note: 'Teclado, mouse ou controle. Melhor no computador, no Chrome, Edge ou Firefox. Seu progresso fica salvo neste navegador.' },
  ja: { load: 'よみこみちゅう...', ready: 'じゅんび OK!', play: 'プレイ', err: 'ゲームを はじめられませんでした。',
        note: 'キーボード、マウス、コントローラー。パソコンの Chrome、Edge、Firefox がおすすめ。しんこうは このブラウザに ほぞんされます。' },
};
const TIPS = {"en": "No Nintendo lawyers approved this game!", "fr": "Aucun avocat de Nintendo n'a approuvé ce jeu !", "de": "Kein Nintendo-Anwalt hat dieses Spiel genehmigt!", "es": "¡Ningún abogado de Nintendo ha aprobado este juego!", "it": "Nessun avvocato di Nintendo ha approvato questo gioco!", "pt": "Nenhum advogado da Nintendo aprovou este jogo!", "ja": "このゲームは 任天堂の べんごしに みとめられていません！"};
const params = new URLSearchParams(location.search);
const code = (params.get('lang') || navigator.language || 'en').slice(0, 2).toLowerCase();
const t = T[code] || T.en;
document.documentElement.lang = T[code] ? code : 'en';

const $ = (id) => document.getElementById(id);
const status = $('status'), fill = $('fill'), start = $('start'), overlay = $('overlay');
status.textContent = t.load;
start.textContent = t.play;
$('note').textContent = t.note;

function fail(msg) {
  failed = true;
  overlay.hidden = false;
  $('bar').hidden = true;
  start.hidden = true;
  status.textContent = t.err;
  status.className = 'error';
  $('note').textContent = String(msg);
  $('note').className = 'error';
}
window.astpCrash = fail;

// --- petite scene : Link court sur la barre, poursuivi par la horde ; une fois pret, il se retourne ---
$('tip').textContent = TIPS[code] || TIPS.en;

const L = {loader};
const sheet = new Image();
sheet.src = 'loader.png?v={v}';
const cv = $('scene'), g = cv.getContext('2d');
const W = cv.width, H = cv.height, GY = 50;
let prog = 0, shown = 0, ready = 0, arrive = 0, failed = false, last = performance.now(), clock = 0;
const horde = [
  { s: 'soldier_green.walk_side', off: 30, flip: false },
  { s: 'octorok.walk_side', off: 52, flip: true },
  { s: 'moblin.walk_side', off: 72, flip: false },
  { s: 'bobomb.walk', off: 94, flip: false },
  { s: 'ganon.walk_side', off: 134, flip: false },
];
horde.forEach((h, i) => { h.x = -30 - h.off; h.seed = i * 1.7; });
const dust = [];

function spr(name, t, x, y, flip) {
  const a = L[name];
  const [sx, sy, w, h, px, py] = a.frames[Math.floor(t * a.fps) % a.frames.length];
  g.save();
  g.translate(Math.round(x), Math.round(y));
  if (flip) g.scale(-1, 1);
  g.drawImage(sheet, sx, sy, w, h, -px, -py, w, h);
  g.restore();
}
function bang(x, y) {
  g.font = 'bold 12px system-ui, sans-serif';
  g.textAlign = 'center';
  g.lineWidth = 3;
  g.strokeStyle = '#10102a';
  g.strokeText('!', x, y);
  g.fillStyle = '#f8d858';
  g.fillText('!', x, y);
}
function frame(now) {
  const dt = Math.min(0.1, (now - last) / 1000);
  last = now;
  clock += dt;
  g.clearRect(0, 0, W, H);
  // sol : herbe et chemin
  g.fillStyle = '#28462c';
  g.fillRect(0, GY - 4, W, H - GY + 4);
  g.fillStyle = '#3c6a3a';
  for (let i = 0; i < W; i += 13) g.fillRect((i * 7) % W, GY + ((i * 3) % 7), 2, 2);
  if (sheet.complete && sheet.naturalWidth) {
    // vitesse plafonnee : meme si tout est deja en cache, la poursuite dure environ 3 s
    shown += Math.max(0, Math.min(prog - shown, dt * 0.33));
    if (!arrive && ready && shown >= 0.999) arrive = now;
    const lx = 14 + shown * (W - 28);
    const after = arrive ? (now - arrive) / 1000 : -1;
    // horde
    for (const h of horde) {
      let flip = h.flip, t = clock + h.seed;
      if (after < 0 || failed) {
        h.x += (lx - h.off - h.x) * Math.min(1, dt * 2.2);
      } else if (after < 0.8) {
        t = h.seed;  // figes
      } else {
        h.x -= dt * (h.s.startsWith('ganon') ? 140 : 190);
        flip = !flip;
        t = clock * 2 + h.seed;
      }
      const name = h.s === 'bobomb.walk' && Math.floor(clock * 4 + h.seed) % 6 === 0 ? 'bobomb.red' : h.s;
      const hop = after < 0 ? Math.abs(Math.sin(clock * 9 + h.seed)) * 1.5 : 0;
      spr(name, t, h.x, GY - hop, flip);
      if (after >= 0.15 && after < 1.4 && !failed) bang(h.x, GY - 30 - (h.s.startsWith('ganon') ? 14 : 0));
    }
    // poussiere derriere Link
    if (!arrive && prog - shown > 0.002 && Math.random() < dt * 14) dust.push({ x: lx - 6, y: GY - 2, t: 0 });
    for (let i = dust.length - 1; i >= 0; i--) {
      const d = dust[i];
      d.t += dt;
      if (d.t > 0.5) { dust.splice(i, 1); continue; }
      g.fillStyle = `rgba(220, 210, 180, ${0.6 * (1 - d.t * 2)})`;
      g.fillRect(Math.round(d.x - d.t * 10), Math.round(d.y - d.t * 8), 2, 2);
    }
    // Link
    if (failed) spr('link.idle_down', 0, lx, GY, false);
    else if (after < 0) spr('link.walk_side', clock, lx, GY - Math.abs(Math.sin(clock * 10)), false);
    else if (after < 1.8) spr('link.idle_side', 0, lx, GY, true);
    else {
      spr('link.idle_down', 0, lx, GY, false);
      const r = Math.min(1, (after - 1.8) * 2);
      spr('rupee_green.0', clock, lx, GY - 24 - r * 6, false);
    }
  }
  if (!overlay.hidden) requestAnimationFrame(frame);
}
requestAnimationFrame(frame);

async function download(url) {
  const r = await fetch(url);
  if (!r.ok) throw new Error(url + ' : HTTP ' + r.status);
  const total = Number(r.headers.get('content-length')) || {pak};
  const reader = r.body.getReader();
  const parts = [];
  let got = 0;
  for (;;) {
    const { done, value } = await reader.read();
    if (done) break;
    parts.push(value);
    got += value.length;
    prog = Math.min(1, got / total);
    fill.style.width = (prog * 100).toFixed(1) + '%';
  }
  const out = new Uint8Array(got);
  let at = 0;
  for (const p of parts) { out.set(p, at); at += p.length; }
  return out;
}

try {
  const wasmBytes = fetch('game_bg.wasm?v={v}').then((r) => {
    if (!r.ok) throw new Error('game_bg.wasm : HTTP ' + r.status);
    return r.arrayBuffer();
  });
  const [pak, wasm] = await Promise.all([download('game.pak?v={v}'), wasmBytes]);
  fill.style.width = '100%';
  prog = 1;
  ready = performance.now();
  status.textContent = t.ready;
  start.hidden = false;
  start.focus();
  start.addEventListener('click', async () => {
    start.disabled = true;
    window.astpPak = pak;
    try {
      await init({ module_or_path: wasm });
      overlay.hidden = true;
      $('game').focus();
    } catch (e) {
      fail(e && e.message ? e.message : e);
    }
  }, { once: true });
} catch (e) {
  fail(e && e.message ? e.message : e);
}
"""


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--no-build", action="store_true")
    a = ap.parse_args()
    if not a.no_build:
        subprocess.run(["cargo", "build", "--release", "--target", "wasm32-unknown-unknown", "-p", "game"],
                       cwd=ROOT, check=True)
    if OUT.exists():
        shutil.rmtree(OUT)
    OUT.mkdir(parents=True)
    subprocess.run(["wasm-bindgen", "--target", "web", "--no-typescript", "--out-dir", str(OUT),
                    "--out-name", "game", str(WASM)], cwd=ROOT, check=True)
    files = pak_files()
    size = write_pak(files, OUT / "game.pak")
    music = music_files()
    mdir = OUT / "assets/audio/music"
    mdir.mkdir(parents=True)
    for m in music:
        shutil.copy(ROOT / "assets/audio/music" / m, mdir / m)
    v = VERSION
    (OUT / "index.html").write_text(PAGE.replace("{v}", v), encoding="utf-8", newline="\n")
    (OUT / "play.css").write_text(CSS, encoding="utf-8", newline="\n")
    loader = build_loader(OUT / "loader.png")
    js = JS.replace("{v}", v).replace("{pak}", str(size)).replace("{loader}", json.dumps(loader, separators=(",", ":")))
    (OUT / "play.js").write_text(js, encoding="utf-8", newline="\n")
    wasm = (OUT / "game_bg.wasm").stat().st_size
    print(f"web v{v} : {len(files)} fichiers dans game.pak ({size / 1e6:.1f} Mo), {len(music)} musiques, "
          f"wasm {wasm / 1e6:.1f} Mo -> {OUT}")


if __name__ == "__main__":
    sys.exit(main())

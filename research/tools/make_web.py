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
  <p id="status">Loading...</p>
  <div id="bar"><div id="fill"></div></div>
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
  overlay.hidden = false;
  $('bar').hidden = true;
  start.hidden = true;
  status.textContent = t.err;
  status.className = 'error';
  $('note').textContent = String(msg);
  $('note').className = 'error';
}
window.astpCrash = fail;

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
    fill.style.width = Math.min(100, (got / total) * 100).toFixed(1) + '%';
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
    (OUT / "play.js").write_text(JS.replace("{v}", v).replace("{pak}", str(size)), encoding="utf-8", newline="\n")
    wasm = (OUT / "game_bg.wasm").stat().st_size
    print(f"web v{v} : {len(files)} fichiers dans game.pak ({size / 1e6:.1f} Mo), {len(music)} musiques, "
          f"wasm {wasm / 1e6:.1f} Mo -> {OUT}")


if __name__ == "__main__":
    sys.exit(main())

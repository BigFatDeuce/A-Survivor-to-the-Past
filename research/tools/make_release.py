"""Paquet Windows publie : dist/A-Survivor-to-the-Past-v<version>-windows-x64.zip

Contenu (dossier « A Survivor to the Past/ ») : l'executable, data/, assets/gen/, assets/audio/,
LICENSE et LISEZ-MOI / README. Les fichiers du joueur (settings.ron...) n'y sont jamais.
Usage : python research/tools/make_release.py [--no-build] [--out dist]
"""
import argparse
import hashlib
import re
import shutil
import subprocess
import sys
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
VERSION = re.search(r'version\s*=\s*"([^"]+)"', (ROOT / "Cargo.toml").read_text(encoding="utf-8")).group(1)
TOP = "A Survivor to the Past"
EXE = "A Survivor to the Past.exe"
REPO = "https://github.com/BigFatDeuce/A-Survivor-to-the-Past"
DISCORD = "https://discord.gg/N8f62M2eaK"
SITE = "https://a-survivor-to-the-past.holy-c.men/"

README = f"""A Survivor to the Past - v{VERSION} (alpha)
==========================================

[English]  Unofficial, free and non-commercial fan game: a survivors-like in the world of
A Link to the Past.

HOW TO PLAY
1. Extract the WHOLE archive into a folder (Documents, Desktop...), not inside "Program Files".
2. Launch "{EXE}".
   Windows may say "Windows protected your PC" because the game is not signed:
   click "More info", then "Run anyway".
Keyboard, mouse or gamepad. Controls and options are in the menus.

UPDATES
At launch the game asks GitHub whether a new version exists. In "Automatic" mode (default) it
downloads it in the background and installs it at the next launch, or right away with
"Restart to update" on the title screen. Options > Updates: Automatic / Ask me / Off.
Manual download: {REPO}/releases/latest
Your settings (settings.ron) are always kept.

If the game crashes, the file crash.log explains why: please share it on Discord.

[Français]  Fan game non officiel, gratuit et non commercial : un survivors-like dans l'univers
de A Link to the Past.

COMMENT JOUER
1. Extrayez TOUTE l'archive dans un dossier (Documents, Bureau...), pas dans « Program Files ».
2. Lancez « {EXE} ».
   Windows peut afficher « Windows a protégé votre ordinateur » car le jeu n'est pas signé :
   cliquez sur « Informations complémentaires » puis « Exécuter quand même ».
Clavier, souris ou manette. Les commandes et options sont dans les menus.

MISES À JOUR
Au lancement, le jeu demande à GitHub s'il existe une nouvelle version. En mode « Automatiques »
(par défaut), il la télécharge en arrière-plan et l'installe au lancement suivant, ou tout de
suite avec « Redémarrer et installer » sur l'écran titre. Options > Mises à jour :
Automatiques / Me demander / Non.
Téléchargement manuel : {REPO}/releases/latest
Vos réglages (settings.ron) sont toujours conservés.

En cas de plantage, le fichier crash.log en donne la raison : partagez-le sur le Discord.

LINKS / LIENS
Discord : {DISCORD}
Site / Website : {SITE}
Source : {REPO}

LEGAL / MENTIONS LÉGALES
Unofficial, free and non-commercial fan game, made for fun. SELLING IT IS FORBIDDEN.
The Legend of Zelda, A Link to the Past, The Minish Cap, their characters, graphics, music and
sounds belong to Nintendo. This project is not affiliated with, sponsored or endorsed by Nintendo.
The GPL-3.0 license (LICENSE) only covers the source code of this project.

Fan game non officiel, gratuit et non commercial, fait pour le plaisir. SA REVENTE EST INTERDITE.
The Legend of Zelda, A Link to the Past, The Minish Cap, leurs personnages, graphismes, musiques
et sons appartiennent à Nintendo. Ce projet n'est ni affilié, ni sponsorisé, ni approuvé par Nintendo.
La licence GPL-3.0 (LICENSE) ne couvre que le code source de ce projet.
"""


def sha256(p):
    h = hashlib.sha256()
    with open(p, "rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--no-build", action="store_true")
    ap.add_argument("--out", default="dist")
    ap.add_argument("--marker", help="fichier ajoute dans data/ (essais de mise a jour)")
    a = ap.parse_args()
    if not a.no_build:
        subprocess.run(["cargo", "build", "--release", "-p", "game"], cwd=ROOT, check=True)
    out = (ROOT / a.out).resolve()
    stage = out / "stage" / TOP
    if stage.parent.exists():
        shutil.rmtree(stage.parent)
    stage.mkdir(parents=True)
    shutil.copy2(ROOT / "target/release/a_survivor_to_the_past.exe", stage / EXE)
    shutil.copytree(ROOT / "data", stage / "data")
    for d in ("assets/gen", "assets/audio"):
        shutil.copytree(ROOT / d, stage / d)
    shutil.copy2(ROOT / "LICENSE", stage / "LICENSE.txt")
    (stage / "README.txt").write_text(README.replace("\n", "\r\n"), encoding="utf-8-sig", newline="")
    if a.marker:
        (stage / "data" / a.marker).write_text(VERSION, encoding="utf-8")
    name = f"A-Survivor-to-the-Past-v{VERSION}-windows-x64.zip"
    zpath = out / name
    if zpath.exists():
        zpath.unlink()
    stored = {".ogg", ".png"}
    with zipfile.ZipFile(zpath, "w") as z:
        for f in sorted(stage.rglob("*")):
            arc = f.relative_to(stage.parent).as_posix()
            if f.is_dir():
                continue
            comp = zipfile.ZIP_STORED if f.suffix.lower() in stored else zipfile.ZIP_DEFLATED
            z.write(f, arc, compress_type=comp, compresslevel=9 if comp == zipfile.ZIP_DEFLATED else None)
    shutil.rmtree(stage.parent)
    size = zpath.stat().st_size
    print(f"paquet : {zpath}")
    print(f"taille : {size / 1e6:.1f} Mo")
    print(f"sha256 : {sha256(zpath)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())

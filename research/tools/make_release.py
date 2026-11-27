"""Paquets publies : dist/A-Survivor-to-the-Past-v<version>-<plateforme>.zip

Contenu (dossier « A Survivor to the Past/ ») : l'executable, data/, assets/gen/, assets/audio/,
LICENSE, LISEZ-MOI / README et le livret d'instructions (PDF, regenere avant par make_manual.py).
Les fichiers du joueur (settings.ron...) n'y sont jamais.
Plateformes : windows-x64 (compile ici), linux-x64, linux-arm64, macos (application .app, donnees
dans Contents/MacOS pour la mise a jour automatique) et portmaster (consoles portables Linux ARM64 :
lanceur .sh + dossier du port). Les executables Linux / macOS viennent du workflow GitHub
.github/workflows/builds.yml (gh run download <id> -D dist/bin).
Usage : python research/tools/make_release.py [--no-build] [--out dist] [--platform P] [--exe chemin]
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
# a la racine du paquet : la mise a jour automatique remplace les fichiers de la racine
MANUALS = ("A-Survivor-to-the-Past-Manual-en.pdf", "A-Survivor-to-the-Past-Manuel-fr.pdf")
EXE = "A Survivor to the Past.exe"
REPO = "https://github.com/BigFatDeuce/A-Survivor-to-the-Past"
DISCORD = "https://discord.gg/N8f62M2eaK"
SITE = "https://a-survivor-to-the-past.holy-c.men/"

README = """A Survivor to the Past - v{VERSION} (alpha)
==========================================

[English]  Unofficial, free and non-commercial fan game: a survivors-like in the world of
A Link to the Past.

HOW TO PLAY
{HOW_EN}
Keyboard, mouse or gamepad. Controls and options are in the menus.
The instruction booklet is A-Survivor-to-the-Past-Manual-en.pdf.

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
{HOW_FR}
Clavier, souris ou manette. Les commandes et options sont dans les menus.
Le livret d'instructions est A-Survivor-to-the-Past-Manuel-fr.pdf.

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
Sprite rippers and artists are credited in CREDITS.txt. Sonic the Hedgehog belongs to SEGA.

Fan game non officiel, gratuit et non commercial, fait pour le plaisir. SA REVENTE EST INTERDITE.
The Legend of Zelda, A Link to the Past, The Minish Cap, leurs personnages, graphismes, musiques
et sons appartiennent à Nintendo. Ce projet n'est ni affilié, ni sponsorisé, ni approuvé par Nintendo.
La licence GPL-3.0 (LICENSE) ne couvre que le code source de ce projet.
Les auteurs des sprites sont crédités dans CREDITS.txt. Sonic the Hedgehog appartient à SEGA.
"""


def sha256(p):
    h = hashlib.sha256()
    with open(p, "rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


PORT_DIR = "asurvivortothepast"
UNIX_EXE = "a-survivor-to-the-past"
APP = "A Survivor to the Past.app"

HOW = {
    "windows-x64": (
        f"""1. Extract the WHOLE archive into a folder (Documents, Desktop...), not inside "Program Files".
2. Launch "{EXE}".
   Windows may say "Windows protected your PC" because the game is not signed:
   click "More info", then "Run anyway".""",
        f"""1. Extrayez TOUTE l'archive dans un dossier (Documents, Bureau...), pas dans « Program Files ».
2. Lancez « {EXE} ».
   Windows peut afficher « Windows a protégé votre ordinateur » car le jeu n'est pas signé :
   cliquez sur « Informations complémentaires » puis « Exécuter quand même »."""),
    "linux": (
        f"""1. Extract the WHOLE archive into a folder of your home directory.
2. Launch "{UNIX_EXE}" (double-click, or ./{UNIX_EXE} in a terminal).
   Needs Vulkan or OpenGL ES 3 graphics drivers and ALSA / PulseAudio / PipeWire sound.
   If the game does not start: WGPU_BACKEND=gl ./{UNIX_EXE}""",
        f"""1. Extrayez TOUTE l'archive dans un dossier de votre répertoire personnel.
2. Lancez « {UNIX_EXE} » (double-clic, ou ./{UNIX_EXE} dans un terminal).
   Pilotes graphiques Vulkan ou OpenGL ES 3 et son ALSA / PulseAudio / PipeWire nécessaires.
   Si le jeu ne démarre pas : WGPU_BACKEND=gl ./{UNIX_EXE}"""),
    "macos": (
        f"""1. Extract the archive and move "{APP}" into Applications.
2. The game is not signed by Apple. Open Terminal once and type:
   xattr -cr "/Applications/{APP}"
   then launch the game normally (otherwise macOS refuses it or says it is damaged).
   Runs on Apple Silicon and Intel Macs (macOS 11 or later).""",
        f"""1. Extrayez l'archive et déplacez « {APP} » dans Applications.
2. Le jeu n'est pas signé par Apple. Ouvrez le Terminal une fois et tapez :
   xattr -cr "/Applications/{APP}"
   puis lancez le jeu normalement (sinon macOS le refuse ou le dit endommagé).
   Fonctionne sur Mac Apple Silicon et Intel (macOS 11 ou plus récent)."""),
    "portmaster": (
        f"""EXPERIMENTAL: Linux ARM64 handhelds (Anbernic RG40XX H...).
1. Copy "A Survivor to the Past.sh" and the folder "{PORT_DIR}" into the ports folder
   of the SD card (ROMS/ports, /roms/ports, /mnt/mmc/ports... depending on the firmware).
2. Needs PortMaster and its Westonpack runtime (PortMaster downloads it when online).
3. Launch it from the Ports menu. Select + Start quits.
   Automatic updates are off: copy the new version over the old one (your saves stay).
   If it does not work, share {PORT_DIR}/log.txt on Discord.""",
        f"""EXPÉRIMENTAL : consoles Linux ARM64 (Anbernic RG40XX H...).
1. Copiez « A Survivor to the Past.sh » et le dossier « {PORT_DIR} » dans le dossier ports
   de la carte SD (ROMS/ports, /roms/ports, /mnt/mmc/ports... selon le firmware).
2. PortMaster et son module Westonpack sont nécessaires (PortMaster le télécharge en ligne).
3. Lancez-le depuis le menu Ports. Select + Start pour quitter.
   Pas de mise à jour automatique : copiez la nouvelle version par-dessus (sauvegardes gardées).
   Si ça ne marche pas, partagez {PORT_DIR}/log.txt sur le Discord."""),
}

# Boutons de la console -> touches du clavier du jeu (data/input.ron), gptokeyb de PortMaster.
# Positions : b = bas (valider / esquive), a = droite (retour), y = gauche (attaque), x = haut.
GPTK = """back = f11
start = esc
a = backspace
b = space
x = tab
y = j
l1 = q
r1 = e
l2 = k
r2 = j
up = up
down = down
left = left
right = right
left_analog_up = up
left_analog_down = down
left_analog_left = left
left_analog_right = right
"""

# Lanceur PortMaster : runtime Westonpack (Weston + Xwayland + EGL GLES de Crusty), wgpu en OpenGL ES.
PORT_SH = r"""#!/bin/bash
# A Survivor to the Past - PortMaster (experimental)
XDG_DATA_HOME=${XDG_DATA_HOME:-$HOME/.local/share}
if [ -d "/opt/system/Tools/PortMaster/" ]; then
  controlfolder="/opt/system/Tools/PortMaster"
elif [ -d "/opt/tools/PortMaster/" ]; then
  controlfolder="/opt/tools/PortMaster"
elif [ -d "$XDG_DATA_HOME/PortMaster/" ]; then
  controlfolder="$XDG_DATA_HOME/PortMaster"
else
  controlfolder="/roms/ports/PortMaster"
fi
source $controlfolder/control.txt
[ -f "${controlfolder}/mod_${CFW_NAME}.txt" ] && source "${controlfolder}/mod_${CFW_NAME}.txt"
get_controls

GAMEDIR="/$directory/ports/@PORT_DIR@"
cd "$GAMEDIR"
> "$GAMEDIR/log.txt" && exec > >(tee "$GAMEDIR/log.txt") 2>&1
chmod +x "$GAMEDIR/@EXE@"

weston_dir=/tmp/weston
weston_runtime="weston_pkg_0.2"
if [ ! -f "$controlfolder/libs/${weston_runtime}.squashfs" ]; then
  if [ ! -f "$controlfolder/harbourmaster" ]; then
    pm_message "This port requires the latest PortMaster to run, please go to https://portmaster.games/ for more info."
    sleep 5
    exit 1
  fi
  $ESUDO $controlfolder/harbourmaster --quiet --no-check runtime_check "${weston_runtime}.squashfs"
fi
$ESUDO mkdir -p "${weston_dir}"
if [[ "$PM_CAN_MOUNT" != "N" ]]; then
  $ESUDO umount "${weston_dir}"
fi
$ESUDO mount "$controlfolder/libs/${weston_runtime}.squashfs" "${weston_dir}"

$GPTOKEYB "@EXE@" -c "./@PORT_DIR@.gptk" &
pm_platform_helper "$GAMEDIR/@EXE@"
$ESUDO env $weston_dir/westonwrap.sh headless noop kiosk crusty_x11egl \
  WGPU_BACKEND=gl ASTP_NO_UPDATE=1 WAYLAND_DISPLAY= SDL_GAMECONTROLLERCONFIG="$sdl_controllerconfig" ./@EXE@

if [[ "$PM_CAN_MOUNT" != "N" ]]; then
  $ESUDO umount "${weston_dir}"
fi
pm_finish
"""

INFO_PLIST = f"""<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleName</key><string>A Survivor to the Past</string>
  <key>CFBundleDisplayName</key><string>A Survivor to the Past</string>
  <key>CFBundleIdentifier</key><string>men.holy-c.a-survivor-to-the-past</string>
  <key>CFBundleExecutable</key><string>{UNIX_EXE}</string>
  <key>CFBundleIconFile</key><string>icon</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleVersion</key><string>{VERSION}</string>
  <key>CFBundleShortVersionString</key><string>{VERSION}</string>
  <key>LSMinimumSystemVersion</key><string>11.0</string>
  <key>NSHighResolutionCapable</key><true/>
</dict>
</plist>
"""


def readme(platform):
    how = HOW["linux" if platform.startswith("linux") else platform]
    text = README.format(VERSION=VERSION, HOW_EN=how[0], HOW_FR=how[1], REPO=REPO, DISCORD=DISCORD, SITE=SITE)
    if platform == "windows-x64":
        return text.replace("\n", "\r\n").encode("utf-8-sig")
    return text.encode("utf-8")


def game_files(dst, exe_src, exe_name):
    """Executable + donnees + licence : la racine du jeu (celle que la mise a jour remplace)."""
    dst.mkdir(parents=True, exist_ok=True)
    shutil.copy2(exe_src, dst / exe_name)
    shutil.copytree(ROOT / "data", dst / "data")
    for d in ("assets/gen", "assets/audio"):
        shutil.copytree(ROOT / d, dst / d)
    shutil.copy2(ROOT / "LICENSE", dst / "LICENSE.txt")
    shutil.copy2(ROOT / "CREDITS.md", dst / "CREDITS.txt")


def docs(dst, platform):
    for m in MANUALS:
        shutil.copy2(ROOT / "dist" / "manual" / m, dst / m)
    (dst / "README.txt").write_bytes(readme(platform))


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--no-build", action="store_true")
    ap.add_argument("--out", default="dist")
    ap.add_argument("--marker", help="fichier ajoute dans data/ (essais de mise a jour)")
    ap.add_argument("--platform", default="windows-x64",
                    choices=["windows-x64", "linux-x64", "linux-arm64", "macos", "portmaster"])
    ap.add_argument("--exe", help="executable deja compile (defaut Linux / macOS : dist/bin/<plateforme>/a-survivor-to-the-past)")
    a = ap.parse_args()
    plat = a.platform
    for m in MANUALS:
        if not (ROOT / "dist" / "manual" / m).is_file():
            sys.exit(f"livret absent : dist/manual/{m} (lancer research/tools/make_manual.py)")
    if plat == "windows-x64":
        if not a.no_build:
            subprocess.run(["cargo", "build", "--release", "-p", "game"], cwd=ROOT, check=True)
        exe_src = Path(a.exe) if a.exe else ROOT / "target/release/a_survivor_to_the_past.exe"
    else:
        bin_plat = "linux-arm64" if plat == "portmaster" else plat
        exe_src = Path(a.exe) if a.exe else ROOT / "dist" / "bin" / bin_plat / UNIX_EXE
    if not exe_src.is_file():
        sys.exit(f"executable absent : {exe_src}")
    out = (ROOT / a.out).resolve()
    stage = out / "stage"
    if stage.exists():
        shutil.rmtree(stage)
    executables = set()
    if plat == "windows-x64":
        root = stage / TOP
        game_files(root, exe_src, EXE)
        docs(root, plat)
    elif plat.startswith("linux"):
        root = stage / TOP
        game_files(root, exe_src, UNIX_EXE)
        docs(root, plat)
        executables.add(root / UNIX_EXE)
    elif plat == "macos":
        # dossier commun « A Survivor to the Past/ » : l'application + lisez-moi et livrets a cote
        contents = stage / TOP / APP / "Contents"
        root = contents / "MacOS"
        game_files(root, exe_src, UNIX_EXE)
        executables.add(root / UNIX_EXE)
        (contents / "Info.plist").write_text(INFO_PLIST, encoding="utf-8")
        (contents / "Resources").mkdir()
        from PIL import Image
        Image.open(ROOT / "assets/icons/icon.ico").save(contents / "Resources" / "icon.icns")
        docs(stage / TOP, plat)
    else:
        # pas de dossier commun : le contenu va tel quel dans le dossier ports de la carte SD
        root = stage / PORT_DIR
        game_files(root, exe_src, UNIX_EXE)
        docs(root, plat)
        executables.add(root / UNIX_EXE)
        (root / f"{PORT_DIR}.gptk").write_text(GPTK, encoding="utf-8", newline="\n")
        sh = stage / "A Survivor to the Past.sh"
        sh.write_text(PORT_SH.replace("@PORT_DIR@", PORT_DIR).replace("@EXE@", UNIX_EXE), encoding="utf-8", newline="\n")
        executables.add(sh)
    if a.marker:
        (root / "data" / a.marker).write_text(VERSION, encoding="utf-8")
    name = f"A-Survivor-to-the-Past-v{VERSION}-{plat}.zip"
    zpath = out / name
    if zpath.exists():
        zpath.unlink()
    stored = {".ogg", ".png", ".icns"}
    with zipfile.ZipFile(zpath, "w") as z:
        for f in sorted(stage.rglob("*")):
            if f.is_dir():
                continue
            info = zipfile.ZipInfo.from_file(f, f.relative_to(stage).as_posix())
            if plat != "windows-x64":
                # droits Unix (createur « Unix ») : executables 755, le reste 644
                info.create_system = 3
                info.external_attr = (0o100755 if f in executables else 0o100644) << 16
            info.compress_type = zipfile.ZIP_STORED if f.suffix.lower() in stored else zipfile.ZIP_DEFLATED
            level = 9 if info.compress_type == zipfile.ZIP_DEFLATED else None
            z.writestr(info, f.read_bytes(), compresslevel=level)
    shutil.rmtree(stage)
    size = zpath.stat().st_size
    print(f"paquet : {zpath}")
    print(f"taille : {size / 1e6:.1f} Mo")
    print(f"sha256 : {sha256(zpath)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())

"""Version Android : construit dist/A-Survivor-to-the-Past-v<version>-android.apk.

Le jeu (crates/game/src/main.rs) est compile en bibliotheque pour NativeActivity (crates/android, sans code
Java) pour les telephones (arm64-v8a) et les emulateurs / Chromebook (x86_64). Les fichiers du jeu sont
dans assets/game.pak (meme format que la version web, musiques comprises), extraits au premier lancement.

Necessite : SDK Android (ANDROID_HOME, build-tools, plateforme), NDK, JDK (keytool, apksigner), cibles Rust
aarch64-linux-android et x86_64-linux-android.

Signature : cle de publication hors du depot (ANDROID_KEYSTORE, par defaut ~/.astp-android/release.jks),
creee au premier passage ; son mot de passe est dans le fichier voisin release.pass. Il faut la garder
(sauvegarde) : une mise a jour de l'APK doit etre signee avec la meme cle.

  python research/tools/make_apk.py            # compile, empaquette, signe
  python research/tools/make_apk.py --abi x86_64 --no-build
"""
import argparse
import os
import re
import secrets
import shutil
import subprocess
import sys
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "research" / "tools"))
import make_web  # noqa: E402  (liste des fichiers du paquet)

VERSION = re.search(r'version\s*=\s*"([^"]+)"', (ROOT / "Cargo.toml").read_text(encoding="utf-8")).group(1)
CRATE = ROOT / "crates" / "android"
PACKAGE = "com.asurvivortothepast.game"
MIN_SDK, TARGET_SDK = 26, 35
ABIS = {"arm64-v8a": "aarch64-linux-android", "x86_64": "x86_64-linux-android"}
SDK = Path(os.environ.get("ANDROID_HOME") or os.environ.get("ANDROID_SDK_ROOT") or Path.home() / "AppData" / "Local" / "Android" / "Sdk")
KEYSTORE = Path(os.environ.get("ANDROID_KEYSTORE") or Path.home() / ".astp-android" / "release.jks")
EXE = ".exe" if os.name == "nt" else ""
CMD = ".cmd" if os.name == "nt" else ""
BAT = ".bat" if os.name == "nt" else ""

MANIFEST = """<?xml version="1.0" encoding="utf-8"?>
<manifest xmlns:android="http://schemas.android.com/apk/res/android"
    package="{package}" android:versionCode="{code}" android:versionName="{version}">
    <uses-permission android:name="android.permission.INTERNET" />
    <uses-feature android:glEsVersion="0x00030000" android:required="true" />
    <uses-feature android:name="android.hardware.touchscreen" android:required="false" />
    <uses-feature android:name="android.hardware.gamepad" android:required="false" />
    <application android:label="A Survivor to the Past" android:icon="@mipmap/ic_launcher"
        android:hasCode="false" android:extractNativeLibs="true" android:isGame="true"
        android:appCategory="game" android:allowBackup="true"
        android:theme="@android:style/Theme.Black.NoTitleBar.Fullscreen">
        <activity android:name="android.app.NativeActivity" android:exported="true"
            android:screenOrientation="sensorLandscape" android:launchMode="singleTask"
            android:configChanges="mcc|mnc|locale|touchscreen|keyboard|keyboardHidden|navigation|orientation|screenLayout|uiMode|screenSize|smallestScreenSize|density|layoutDirection|fontScale|colorMode|fontWeightAdjustment|grammaticalGender">
            <meta-data android:name="android.app.lib_name" android:value="astp" />
            <intent-filter>
                <action android:name="android.intent.action.MAIN" />
                <category android:name="android.intent.category.LAUNCHER" />
            </intent-filter>
        </activity>
    </application>
</manifest>
"""

ICONS = {"mdpi": 48, "hdpi": 72, "xhdpi": 96, "xxhdpi": 144, "xxxhdpi": 192}


def latest(d: Path) -> Path:
    return sorted((p for p in d.iterdir() if p.is_dir()), key=lambda p: [int(x) if x.isdigit() else 0 for x in re.split(r"[.-]", p.name)])[-1]


def version_code(v):
    a, b, c = (int(x) for x in v.split(".")[:3])
    return a * 10000 + b * 100 + c


def build(abis):
    ndk = Path(os.environ.get("ANDROID_NDK_HOME") or latest(SDK / "ndk"))
    bin_dir = next((ndk / "toolchains" / "llvm" / "prebuilt").iterdir()) / "bin"
    env = dict(os.environ, ANDROID_NDK_HOME=str(ndk), ANDROID_NDK_ROOT=str(ndk))
    for abi in abis:
        t = ABIS[abi]
        clang = str(bin_dir / f"{t}{MIN_SDK}-clang{CMD}")
        u = t.replace("-", "_")
        env[f"CARGO_TARGET_{u.upper()}_LINKER"] = clang
        env[f"CC_{u}"] = clang
        env[f"AR_{u}"] = str(bin_dir / f"llvm-ar{EXE}")
        subprocess.run(["cargo", "build", "--release", "--target", t], cwd=CRATE, env=env, check=True)


def key():
    """Cle de publication : creee au premier passage (mot de passe aleatoire dans release.pass)."""
    pw = KEYSTORE.with_suffix(".pass")
    if not KEYSTORE.is_file():
        KEYSTORE.parent.mkdir(parents=True, exist_ok=True)
        pw.write_text(secrets.token_urlsafe(24), encoding="utf-8")
        subprocess.run(["keytool", "-genkeypair", "-keystore", str(KEYSTORE), "-alias", "astp", "-keyalg", "RSA",
                        "-keysize", "4096", "-validity", "36500", "-storepass:file", str(pw), "-keypass:file", str(pw),
                        "-dname", "CN=A Survivor to the Past"], check=True)
        print(f"nouvelle cle de publication : {KEYSTORE} (a sauvegarder avec {pw.name})")
    return pw


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--no-build", action="store_true")
    ap.add_argument("--abi", action="append", choices=list(ABIS), help="architecture (repetable ; defaut : toutes)")
    a = ap.parse_args()
    abis = a.abi or list(ABIS)
    crate_version = re.search(r'^version\s*=\s*"([^"]+)"', (CRATE / "Cargo.toml").read_text(encoding="utf-8"), re.M).group(1)
    if crate_version != VERSION:
        sys.exit(f"crates/android/Cargo.toml : version {crate_version}, l'espace de travail est en {VERSION}")
    if not a.no_build:
        build(abis)
    tools = latest(SDK / "build-tools")
    platform = SDK / "platforms" / f"android-{TARGET_SDK}" / "android.jar"
    work = ROOT / "dist" / "android"
    if work.exists():
        shutil.rmtree(work)
    res = work / "res"
    from PIL import Image
    icon = Image.open(ROOT / "assets/icons/icon_512.png").convert("RGBA")
    for dpi, px in ICONS.items():
        d = res / f"mipmap-{dpi}"
        d.mkdir(parents=True)
        icon.resize((px, px), Image.LANCZOS).save(d / "ic_launcher.png")
    (work / "AndroidManifest.xml").write_text(
        MANIFEST.format(package=PACKAGE, code=version_code(VERSION), version=VERSION), encoding="utf-8")
    flat = work / "res.zip"
    subprocess.run([str(tools / f"aapt2{EXE}"), "compile", "--dir", str(res), "-o", str(flat)], check=True)
    base = work / "base.apk"
    subprocess.run([str(tools / f"aapt2{EXE}"), "link", "-o", str(base), "-I", str(platform),
                    "--manifest", str(work / "AndroidManifest.xml"), "--min-sdk-version", str(MIN_SDK),
                    "--target-sdk-version", str(TARGET_SDK), str(flat)], check=True)
    # fichiers du jeu (comme la version web) + musiques utilisees
    files = make_web.pak_files() + [f"assets/audio/music/{m}" for m in make_web.music_files()]
    pak = work / "game.pak"
    size = make_web.write_pak(files, pak)
    unsigned = work / "unsigned.apk"
    shutil.copy(base, unsigned)
    with zipfile.ZipFile(unsigned, "a") as z:
        z.write(pak, "assets/game.pak", compress_type=zipfile.ZIP_STORED)
        for abi in abis:
            so = CRATE / "target" / ABIS[abi] / "release" / "libastp.so"
            if not so.is_file():
                sys.exit(f"bibliotheque absente : {so}")
            z.write(so, f"lib/{abi}/libastp.so", compress_type=zipfile.ZIP_DEFLATED)
    aligned = work / "aligned.apk"
    subprocess.run([str(tools / f"zipalign{EXE}"), "-f", "-p", "4", str(unsigned), str(aligned)], check=True)
    pw = key()
    out = ROOT / "dist" / f"A-Survivor-to-the-Past-v{VERSION}-android.apk"
    subprocess.run([str(tools / f"apksigner{BAT}"), "sign", "--ks", str(KEYSTORE), "--ks-key-alias", "astp",
                    "--ks-pass", f"file:{pw}", "--out", str(out), str(aligned)], check=True)
    subprocess.run([str(tools / f"apksigner{BAT}"), "verify", str(out)], check=True)
    print(f"APK v{VERSION} ({', '.join(abis)}) : {len(files)} fichiers ({size / 1e6:.1f} Mo), "
          f"{out.stat().st_size / 1e6:.1f} Mo -> {out}")


if __name__ == "__main__":
    main()

"""Musiques « pretes a boucler » : boucle sans couture, comme dans le jeu d'origine.

Dans ALttP, le pilote son (N-SPC) joue l'introduction une fois puis repart du debut de la boucle
sans aucune coupure ; le recompile (ZeldaAlttPSNESRecomp) fait pareil avec ses pistes MSU-1 : un
fichier PCM et un point de boucle en echantillons. Nos rips jouent : intro + boucle x2 + fondu,
mais le second passage n'est pas une copie exacte au sample pres (decalages de quelques ms) : un
simple saut s'entend. On fabrique donc des fichiers ou la jointure est continue par construction :

1) longueur de boucle L : correlation des attaques des notes (flux spectral, pas de 4 ms) entre t et
   t - L ; on garde le plus grand L presque aussi bon que le meilleur (une phrase repetee donne un L
   plus court), verifie sur la forme d'onde (correlation locale) ;
2) point de saut E dans le second passage (la ou les deux passages se ressemblent le plus) et son
   decalage exact l : debut de boucle S = E - l ;
3) fichier = x[0:E], ses 60 dernieres ms fondues vers x[S-60ms:S] (au saut, le signal continue
   exactement), puis 0,5 s de x[S:] (filet de securite si un lecteur depasse un peu la fin) ;
4) audio.ron : loop_start = S, loop_end = E (en secondes, au sample pres a 32 kHz).

Les originaux sont relus dans l'historique git (commit de la 0.6.0), le script peut donc etre
relance ; une repetition douteuse (morceaux inutilises par le jeu) laisse l'enregistrement
d'origine. Les ambiances (pluie, vent) sont du bruit : fondu de 0,3 s entre la fin et le debut.
Resultats : research/music_loopify.json ; les points sont a recopier dans data/audio.ron.
Usage : python music_loopify.py [id...]   (sans argument : toutes les musiques en boucle)
"""
import sys, os, re, json, subprocess, tempfile
import numpy as np

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
SRC_COMMIT = "7e28f6b"
SR = 32000
HOP = 512
BLEND = int(0.06 * SR)
TAIL = int(0.5 * SR)
AMBIENCES = {"fierce_rainstorm", "howling_wind"}


def original(file):
    blob = subprocess.run(["git", "-C", ROOT, "show", f"{SRC_COMMIT}:assets/audio/music/{file}"],
                          capture_output=True, check=True).stdout
    with tempfile.NamedTemporaryFile(suffix=".ogg", delete=False) as t:
        t.write(blob)
        path = t.name
    raw = subprocess.run(["ffmpeg", "-v", "error", "-i", path, "-map", "0:a:0", "-ac", "2", "-ar", str(SR),
                          "-f", "f32le", "-"], capture_output=True, check=True).stdout
    os.unlink(path)
    return np.frombuffer(raw, np.float32).astype(np.float64).reshape(-1, 2)


def restore(file):
    blob = subprocess.run(["git", "-C", ROOT, "show", f"{SRC_COMMIT}:assets/audio/music/{file}"],
                          capture_output=True, check=True).stdout
    open(os.path.join(ROOT, "assets/audio/music", file), "wb").write(blob)


def fade_start(m2):
    """Debut du fondu de fin de l'enregistrement : derniere fenetre de 0,25 s encore proche du
    niveau habituel du morceau."""
    x = m2.mean(axis=1)
    hop = SR // 4
    r = np.array([np.sqrt(np.mean(x[i : i + hop] ** 2)) for i in range(0, len(x) - hop, hop)])
    ref = np.median(r[: max(1, int(len(r) * 0.8))])
    k = len(r) - 1
    while k > 0 and r[k] < 0.6 * ref:
        k -= 1
    return round((k + 1) * hop / SR, 2)


def features(m):
    n = (len(m) - 1024) // HOP
    idx = np.arange(1024)[None, :] + HOP * np.arange(n)[:, None]
    spec = np.abs(np.fft.rfft(m[idx] * np.hanning(1024)[None, :], axis=1))[:, 1:385]
    f = np.log1p(spec.reshape(n, 32, 12).sum(axis=2) * 10)
    f -= f.mean(axis=1, keepdims=True)
    f /= np.linalg.norm(f, axis=1, keepdims=True) + 1e-9
    return f


def local(m, t, L, w=0.16, pre=0.06, search=0.02):
    """Meilleure correlation entre m[t-pre : t-pre+w] et la meme fenetre une boucle plus tot."""
    b = t - int(pre * SR)
    win = m[b : b + int(w * SR)]
    s = int(search * SR)
    lo = b - L - s
    if lo < 0:
        return -1.0, L
    seg = m[lo : lo + 2 * s + len(win)]
    c = np.correlate(seg, win, "valid")
    n = np.sqrt(np.convolve(seg ** 2, np.ones(len(win)), "valid")) * np.linalg.norm(win)
    cc = c / (n + 1e-9)
    i = int(np.argmax(cc))
    return float(cc[i]), b - (lo + i)


def onsets(m):
    """Enveloppe des attaques (flux spectral positif), 250 trames/s : le rythme exact du morceau."""
    hop, n_fft = 128, 1024
    n = (len(m) - n_fft) // hop
    idx = np.arange(n_fft)[None, :] + hop * np.arange(n)[:, None]
    spec = np.log1p(10 * np.abs(np.fft.rfft(m[idx] * np.hanning(n_fft)[None, :], axis=1))[:, 1:257])
    flux = np.maximum(spec[1:] - spec[:-1], 0).sum(axis=1)
    flux -= np.convolve(flux, np.ones(25) / 25, "same")
    return np.maximum(flux, 0), hop


def period_scores(m, fade):
    """Correlation des attaques entre t et t - L, pour chaque L (pas de 4 ms)."""
    env, hop = onsets(m)
    U = min(int((fade - 0.3) * SR / hop), len(env))
    out = {}
    for L in range(int(3 * SR / hop), U):
        a, b = env[L:U], env[: U - L]
        if (U - L) * hop < 0.25 * L * hop or (U - L) * hop < 3 * SR:
            continue
        a0, b0 = a - a.mean(), b - b.mean()
        out[L * hop] = float(np.dot(a0, b0) / (np.linalg.norm(a0) * np.linalg.norm(b0) + 1e-9))
    return out


def profile(m, fade, L, t0=None):
    ts = np.arange(t0 if t0 else L / SR + 0.5, fade - 0.3, 0.25)
    return ts, np.array([local(m, int(t * SR), L)[0] for t in ts])


def choose_period(m, fade, old=None):
    """Candidats : les pics de correlation des attaques (multiples de la mesure). Juge de paix : la
    forme d'onde (mediane + 10e centile de la correlation locale, apres le debut de la repetition) ;
    a egalite, le plus GRAND L. Renvoie aussi le debut de la repetition."""
    sc = period_scores(m, fade)
    top = max(sc.values())
    keys = sorted(sc)
    peaks = [L for i, L in enumerate(keys[1:-1], 1) if sc[L] >= sc[keys[i - 1]] and sc[L] >= sc[keys[i + 1]]]
    good = sorted((L for L in peaks if sc[L] >= 0.8 * top), key=lambda L: -sc[L])[:10]
    if old:
        good.append(int(old * SR))
    res = []
    for L in good:
        ts, c = profile(m, fade, L)
        # debut de la repetition : premiere fenetre suivie de 4 bonnes correlations
        i0 = next((i for i in range(len(c) - 4) if np.median(c[i : i + 4]) >= 0.6), None)
        if i0 is None:
            continue
        r = c[i0:]
        # une fausse periode (phrase repetee) a des trous reguliers : le 10e centile les voit
        sco = float(np.median(r) + np.percentile(r, 10))
        res.append((sco, L, max(0, int(ts[i0] * SR) - L), float(np.median(r))))
    if not res:
        return None
    best = max(r[0] for r in res)
    sco, L, R, med = max((r for r in res if r[0] >= best - 0.03), key=lambda r: r[1])
    return L, med, sc[L], R


def build(m, S, E, blend):
    y = m[:E].copy()
    w = 0.5 - 0.5 * np.cos(np.linspace(0, np.pi, blend))[:, None]
    y[E - blend :] = m[E - blend : E] * (1 - w) + m[S - blend : S] * w
    return np.concatenate([y, m[S : S + TAIL]])


def encode(y, path):
    with tempfile.NamedTemporaryFile(suffix=".f32", delete=False) as t:
        t.write(np.clip(y, -1, 1).astype(np.float32).tobytes())
        raw = t.name
    subprocess.run(["ffmpeg", "-v", "error", "-y", "-f", "f32le", "-ar", str(SR), "-ac", "2", "-i", raw,
                    "-c:a", "libvorbis", "-q:a", "5", path], check=True)
    os.unlink(raw)


def loopify(mid, e):
    m2 = original(e["file"])
    m = m2.mean(axis=1)
    fade = fade_start(m2)
    if mid in AMBIENCES:
        S, E = int(0.5 * SR), int((fade - 0.5) * SR)
        encode(build(m2, S, E, int(0.3 * SR)), os.path.join(ROOT, "assets/audio/music", e["file"]))
        return {"loop_start": S / SR, "loop_end": E / SR, "L": (E - S) / SR, "corr": None}
    old = (e["loop_end"] - e["loop_start"]) if e.get("loop_end") else None
    found = choose_period(m, fade, old)
    if found is None:
        return None
    L, med, run, R = found
    lo = max(R + L + int(0.5 * SR), L + BLEND + SR // 10)
    hi = int((fade - 0.4) * SR)
    pick = (-2.0, 0, L)
    for t in range(lo, hi, SR // 50):
        c, l = local(m, t, L)
        if c > pick[0]:
            pick = (c, t, l)
    c, E, l = pick
    S = E - l
    if med < 0.5 or c < 0.95:
        # repetition douteuse : on garde l'enregistrement d'origine
        restore(e["file"])
        return None
    encode(build(m2, S, E, BLEND), os.path.join(ROOT, "assets/audio/music", e["file"]))
    return {"loop_start": S / SR, "loop_end": E / SR, "L": l / SR, "corr": round(c, 4), "median": round(med, 3),
            "repeat_from": round(R / SR, 2), "onset_corr": round(run, 3)}


def entries():
    txt = open(os.path.join(ROOT, "data/audio.ron"), encoding="utf-8").read()
    out = {}
    for mm in re.finditer(r'"(\w+)": \(file: "([^"]+)"([^\n]*)\),', txt):
        mid, file, rest = mm.groups()
        if "once: true" in rest:
            continue
        g = lambda k: (lambda r: float(r.group(1)) if r else None)(re.search(k + r": (?:Some\()?([\d.]+)", rest))
        out[mid] = {"file": file, "loop_start": g("loop_start") or 0.0, "loop_end": g("loop_end")}
    return out


def main():
    ents = entries()
    ids = sys.argv[1:] or list(ents)
    out_json = os.path.join(ROOT, "research/music_loopify.json")
    res = json.load(open(out_json)) if os.path.exists(out_json) else {}
    seen = set()
    for mid in ids:
        e = ents[mid]
        if e["file"] in seen:
            continue
        seen.add(e["file"])
        r = loopify(mid, e)
        if r is None:
            print(f"{mid:32s} aucune boucle fiable : fichier d'origine garde", flush=True)
            res.pop(mid, None)
            json.dump(res, open(out_json, "w"), indent=1)
            continue
        r["file"] = e["file"]
        res[mid] = r
        print(f"{mid:32s} S {r['loop_start']:9.5f} E {r['loop_end']:9.5f} L {r['L']:7.3f} corr {r['corr']} "
              f"median {r.get('median', '')} rep {r.get('repeat_from', '')} onsets {r.get('onset_corr', '')}", flush=True)
        json.dump(res, open(out_json, "w"), indent=1)


main()

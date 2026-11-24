"""Detection automatique des points de boucle des musiques SNES (rips FLAC 32 kHz).

Les rips jouent : intro + boucle x2 + fondu. On cherche le decalage L (longueur de boucle)
pour lequel le signal "se repete" sur la plus longue duree continue avant le fondu :
1) analyse spectrale grossiere (bandes de frequence, 62,5 trames/s) -> L approximatif ;
2) affinage a l'echantillon pres par correlation sur une fenetre de 1 s ;
3) debut de boucle = point le plus tot ou x[t] ~ x[t+L] (similarite par blocs).
Usage : python music_loops.py <dossier_flac> <sortie.json>
"""
import sys, os, json, subprocess, glob
import numpy as np

SR = 32000
HOP = 512

def decode(path):
    raw = subprocess.run(["ffmpeg", "-v", "error", "-i", path, "-ac", "1", "-ar", str(SR), "-f", "f32le", "-"],
                         capture_output=True, check=True).stdout
    return np.frombuffer(raw, np.float32).astype(np.float64)

def features(x):
    n = (len(x) - 1024) // HOP
    idx = np.arange(1024)[None, :] + HOP * np.arange(n)[:, None]
    frames = x[idx] * np.hanning(1024)[None, :]
    spec = np.abs(np.fft.rfft(frames, axis=1))[:, 1:385]           # jusqu'a ~12 kHz
    bands = spec.reshape(n, 32, 12).sum(axis=2)
    f = np.log1p(bands * 10)
    f -= f.mean(axis=1, keepdims=True)
    f /= np.linalg.norm(f, axis=1, keepdims=True) + 1e-9
    return f

def block_sim(a, b):
    na, nb = np.linalg.norm(a), np.linalg.norm(b)
    if na < 1e-6 and nb < 1e-6:
        return 1.0
    return float(np.dot(a, b) / (na * nb + 1e-12))

def analyze(path):
    x = decode(path)
    dur = len(x) / SR
    rms = lambda a: float(np.sqrt(np.mean(a ** 2)) + 1e-9)
    fade = rms(x[-3 * SR:]) < 0.5 * rms(x[: max(len(x) - 3 * SR, SR)])
    if dur < 15:
        return {"duration": round(dur, 3), "kind": "jingle", "fade": bool(fade)}
    f = features(x)
    nfr = len(f)
    usable = nfr - int((10.0 if fade else 0.5) * SR / HOP)   # trames avant le fondu
    # Pour chaque decalage L : plus longue serie continue de trames ou f[t] ~ f[t+L].
    # La vraie boucle est le plus GRAND L dont la serie couvre presque toute la boucle
    # (une phrase musicale repetee donne un L plus court ; 2xL exigerait 3 passages).
    cands = []
    for L in range(int(3 * SR / HOP), usable // 2 + 1):
        sim = np.einsum("ij,ij->i", f[:usable - L], f[L:usable])
        # lissage sur ~1 s : l'echo SNES cree de petites differences ponctuelles
        k1 = 63
        smooth = np.convolve(sim, np.ones(k1) / k1, mode="same")
        ok = np.concatenate([[False], smooth > 0.88, [False]]).astype(np.int8)
        d = np.diff(ok)
        starts, ends = np.nonzero(d == 1)[0], np.nonzero(d == -1)[0]
        if len(starts) == 0:
            continue
        k = int(np.argmax(ends - starts))
        run = int(ends[k] - starts[k])
        cands.append((L, run, int(starts[k]), float(sim[starts[k]:ends[k]].mean())))
    good = [c for c in cands if c[1] >= 0.8 * c[0]]
    if good:
        L, run, run_start, quality = max(good, key=lambda c: c[0])
    else:
        L, run, run_start, quality = max(cands, key=lambda c: c[1] / c[0])
    Ls = L * HOP
    # affinage de L a l'echantillon pres
    end_ref = usable * HOP - SR
    ref = x[end_ref - SR:end_ref]
    lo, hi = end_ref - SR - Ls - HOP, end_ref - SR - Ls + HOP
    scores = [block_sim(ref, x[p:p + SR]) for p in range(lo, hi)]
    Ls = (end_ref - SR) - (lo + int(np.argmax(scores)))
    # debut de boucle : reculer tant que les blocs de 50 ms se ressemblent
    t = usable * HOP - Ls
    blk = SR // 20
    while t - blk >= 0 and block_sim(x[t - blk:t], x[t - blk + Ls:t + Ls]) > 0.97:
        t -= blk
    return {"duration": round(dur, 3), "kind": "music", "fade": bool(fade),
            "loop_start": round(t / SR, 4), "loop_end": round((t + Ls) / SR, 4),
            "loop_start_sample": int(t), "loop_len_sample": int(Ls),
            "quality": round(quality, 3), "matched_seconds": round(run * HOP / SR, 1)}

def main():
    folder, out = sys.argv[1], sys.argv[2]
    res = {}
    for fpath in sorted(glob.glob(os.path.join(folder, "*.flac"))):
        r = analyze(fpath)
        res[os.path.basename(fpath)] = r
        if r["kind"] == "music":
            print(f"{os.path.basename(fpath)[:40]:40s} dur={r['duration']:7.2f} boucle={r['loop_start']:7.3f} -> "
                  f"{r['loop_end']:7.3f} (L={r['loop_len_sample']/SR:6.2f}s) qual={r['quality']} match={r['matched_seconds']}s")
        else:
            print(f"{os.path.basename(fpath)[:40]:40s} jingle {r['duration']:.2f}s")
    json.dump(res, open(out, "w"), indent=1)

main()

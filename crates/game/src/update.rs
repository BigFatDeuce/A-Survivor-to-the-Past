//! Mises a jour du jeu depuis les versions publiees sur GitHub.
//!
//! 1. Verification (fil d'execution a part) : derniere version publiee, comparee a la notre.
//! 2. Telechargement de l'archive de la plateforme dans `.update/`, controle de la taille et de
//!    l'empreinte SHA-256 publiee par GitHub, extraction (chemins controles) dans `.update/new/`.
//! 3. Installation, jeu ferme : le nouvel executable (`--apply-update`) attend la fermeture de
//!    l'ancien, remplace l'executable et les dossiers `data/` et `assets/` (anciens gardes de cote
//!    et remis en place en cas d'erreur), puis relance le jeu.
//!
//! Les fichiers du joueur (`settings.ron`, captures, journaux) ne sont jamais touches. Seules les
//! archives publiees sur le depot officiel sont acceptees. Le jeu installe a la main (dossier de
//! developpement) ne se met pas a jour.
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

pub const REPO: &str = "BigFatDeuce/A-Survivor-to-the-Past";
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
/// Nom de l'executable dans le paquet publie.
#[cfg(windows)]
pub const PACKAGE_EXE: &str = "A Survivor to the Past.exe";
#[cfg(not(windows))]
pub const PACKAGE_EXE: &str = "a-survivor-to-the-past";
/// Fin du nom de l'archive de cette plateforme (make_release.py --platform).
#[cfg(all(windows, target_arch = "x86_64"))]
pub const ASSET_SUFFIX: &str = "-windows-x64.zip";
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
pub const ASSET_SUFFIX: &str = "-linux-x64.zip";
#[cfg(all(target_os = "linux", target_arch = "aarch64"))]
pub const ASSET_SUFFIX: &str = "-linux-arm64.zip";
#[cfg(target_os = "macos")]
pub const ASSET_SUFFIX: &str = "-macos.zip";
#[cfg(not(any(all(windows, target_arch = "x86_64"), all(target_os = "linux", any(target_arch = "x86_64", target_arch = "aarch64")), target_os = "macos")))]
pub const ASSET_SUFFIX: &str = "-no-package-yet.zip";
/// Dossiers remplaces en entier (ils ne contiennent aucun fichier du joueur).
const REPLACED_DIRS: [&str; 2] = ["data", "assets"];
/// Taille maximale acceptee pour l'archive et pour chaque fichier extrait.
const MAX_SIZE: u64 = 1 << 30;
const UPDATE_DIR: &str = ".update";

/// Reglage du joueur.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub enum UpdateMode {
    /// Verifie au lancement, telecharge seul, installe au prochain lancement (ou tout de suite).
    #[default]
    Auto,
    /// Verifie au lancement et propose la mise a jour sur l'ecran titre.
    Ask,
    /// Aucune connexion.
    Off,
}

impl UpdateMode {
    pub const ALL: [UpdateMode; 3] = [UpdateMode::Auto, UpdateMode::Ask, UpdateMode::Off];
}

#[derive(Clone, Debug)]
pub struct Release {
    pub version: String,
    url: String,
    size: u64,
    sha256: Option<String>,
}

impl Release {
    /// Version fictive (verification de l'interface).
    pub fn sample(version: &str) -> Release {
        Release { version: version.into(), url: String::new(), size: 0, sha256: None }
    }
}

// les versions et messages d'erreur servent aux journaux (Debug)
#[allow(dead_code)]
#[derive(Clone, Debug)]
pub enum Status {
    Idle,
    Checking,
    UpToDate,
    Available(Release),
    Downloading { version: String, done: u64, total: u64 },
    /// Pret a installer (version).
    Ready(String),
    Failed(String),
}

/// Ou chercher les versions : GitHub, ou un dossier local (essais : `latest.json` + archive).
#[derive(Clone, Debug)]
pub enum Source {
    GitHub,
    Local(PathBuf),
}

pub struct Updater {
    root: PathBuf,
    source: Source,
    status: Arc<Mutex<Status>>,
    /// Jeu installe depuis un paquet publie (sinon : pas de mise a jour).
    pub enabled: bool,
}

#[derive(Deserialize)]
struct Latest {
    tag_name: String,
    #[serde(default)]
    assets: Vec<Asset>,
}

#[derive(Deserialize)]
struct Asset {
    name: String,
    browser_download_url: String,
    #[serde(default)]
    size: u64,
    #[serde(default)]
    digest: Option<String>,
}

/// "v1.2.3" -> (1, 2, 3) ; suffixes (-beta...) ignores.
fn parse_version(s: &str) -> Option<(u64, u64, u64)> {
    let s = s.trim().trim_start_matches('v');
    let core = s.split(['-', '+']).next()?;
    let mut it = core.split('.').map(|p| p.parse::<u64>());
    let v = (it.next()?.ok()?, it.next().unwrap_or(Ok(0)).ok()?, it.next().unwrap_or(Ok(0)).ok()?);
    Some(v)
}

pub fn is_newer(candidate: &str, current: &str) -> bool {
    match (parse_version(candidate), parse_version(current)) {
        (Some(a), Some(b)) => a > b,
        _ => false,
    }
}

fn agent(timeout: Duration) -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_global(Some(timeout))
        .user_agent(concat!("A-Survivor-to-the-Past/", env!("CARGO_PKG_VERSION")))
        .build()
        .into()
}

fn set(status: &Arc<Mutex<Status>>, s: Status) {
    if let Ok(mut g) = status.lock() {
        *g = s;
    }
}

impl Updater {
    pub fn new(root: &Path, source: Option<PathBuf>) -> Updater {
        let source = match source {
            Some(p) => Source::Local(p),
            None => Source::GitHub,
        };
        // ASTP_NO_UPDATE : paquets dont le lanceur doit rester maitre (PortMaster)
        let enabled = matches!(source, Source::Local(_)) || (is_packaged(root) && std::env::var_os("ASTP_NO_UPDATE").is_none());
        let mut status = Status::Idle;
        if let Ok(v) = fs::read_to_string(root.join(UPDATE_DIR).join("ready")) {
            status = Status::Ready(v.trim().to_string());
        }
        Updater { root: root.to_path_buf(), source, status: Arc::new(Mutex::new(status)), enabled }
    }

    pub fn status(&self) -> Status {
        self.status.lock().map(|g| g.clone()).unwrap_or(Status::Idle)
    }

    /// Pour la verification de l'interface.
    pub fn force_status(&self, s: Status) {
        set(&self.status, s);
    }

    /// Lance la verification en arriere-plan (sans effet si deja faite ou impossible).
    pub fn check(&self) {
        if !self.enabled || !matches!(self.status(), Status::Idle) {
            return;
        }
        set(&self.status, Status::Checking);
        let (status, source) = (self.status.clone(), self.source.clone());
        std::thread::spawn(move || match fetch_latest(&source) {
            Ok(Some(r)) => {
                log::info!("mise a jour disponible : v{}", r.version);
                set(&status, Status::Available(r));
            }
            Ok(None) => {
                log::info!("jeu a jour (v{VERSION})");
                set(&status, Status::UpToDate);
            }
            Err(e) => {
                // hors ligne, GitHub indisponible... : rien a signaler au joueur
                log::warn!("verification des mises a jour impossible : {e}");
                set(&status, Status::UpToDate);
            }
        });
    }

    /// Telecharge et prepare la mise a jour en arriere-plan.
    pub fn download(&self, rel: Release) {
        if !self.enabled || matches!(self.status(), Status::Downloading { .. } | Status::Ready(_)) {
            return;
        }
        set(&self.status, Status::Downloading { version: rel.version.clone(), done: 0, total: rel.size });
        let (status, source, root) = (self.status.clone(), self.source.clone(), self.root.clone());
        std::thread::spawn(move || {
            let up = root.join(UPDATE_DIR);
            match prepare(&source, &rel, &up, &status) {
                Ok(()) => {
                    log::info!("mise a jour v{} prete", rel.version);
                    set(&status, Status::Ready(rel.version.clone()));
                }
                Err(e) => {
                    log::warn!("mise a jour v{} : {e}", rel.version);
                    let _ = fs::remove_dir_all(&up);
                    set(&status, Status::Failed(e));
                }
            }
        });
    }
}

/// Le jeu tourne-t-il depuis un paquet publie (executable du paquet, a la racine du jeu) ?
fn is_packaged(root: &Path) -> bool {
    let Ok(exe) = std::env::current_exe() else {
        return false;
    };
    let name_ok = exe.file_name().is_some_and(|n| n == PACKAGE_EXE);
    let dir_ok = match (exe.parent().map(fs::canonicalize), fs::canonicalize(root)) {
        (Some(Ok(a)), Ok(b)) => a == b,
        _ => false,
    };
    name_ok && dir_ok
}

fn fetch_latest(source: &Source) -> Result<Option<Release>, String> {
    let latest: Latest = match source {
        Source::GitHub => {
            let url = format!("https://api.github.com/repos/{REPO}/releases/latest");
            let res = agent(Duration::from_secs(15))
                .get(&url)
                .header("Accept", "application/vnd.github+json")
                .call();
            match res {
                Ok(mut r) => r.body_mut().read_json().map_err(|e| e.to_string())?,
                // aucune version publiee
                Err(ureq::Error::StatusCode(404)) => return Ok(None),
                Err(e) => return Err(e.to_string()),
            }
        }
        Source::Local(dir) => {
            let t = fs::read_to_string(dir.join("latest.json")).map_err(|e| e.to_string())?;
            serde_json::from_str(&t).map_err(|e| e.to_string())?
        }
    };
    if !is_newer(&latest.tag_name, VERSION) {
        return Ok(None);
    }
    let Some(a) = latest.assets.into_iter().find(|a| a.name.ends_with(ASSET_SUFFIX)) else {
        return Ok(None);
    };
    // seules les archives du depot officiel sont acceptees
    let ok = match source {
        Source::GitHub => a.browser_download_url.starts_with(&format!("https://github.com/{REPO}/releases/download/")),
        Source::Local(_) => !a.browser_download_url.contains(['/', '\\']) && !a.browser_download_url.contains(".."),
    };
    if !ok {
        return Err(format!("adresse d'archive refusee : {}", a.browser_download_url));
    }
    let sha256 = a.digest.and_then(|d| d.strip_prefix("sha256:").map(|h| h.to_ascii_lowercase()));
    let version = latest.tag_name.trim_start_matches('v').to_string();
    Ok(Some(Release { version, url: a.browser_download_url, size: a.size, sha256 }))
}

/// Telechargement + controle + extraction dans `up/new`, puis marqueur `up/ready`.
fn prepare(source: &Source, rel: &Release, up: &Path, status: &Arc<Mutex<Status>>) -> Result<(), String> {
    let _ = fs::remove_dir_all(up);
    fs::create_dir_all(up).map_err(|e| format!("{}: {e}", up.display()))?;
    let zip_path = up.join("package.zip");
    let mut reader: Box<dyn Read> = match source {
        Source::GitHub => {
            let r = agent(Duration::from_secs(1800)).get(&rel.url).call().map_err(|e| e.to_string())?;
            Box::new(r.into_body().into_reader())
        }
        Source::Local(dir) => Box::new(fs::File::open(dir.join(&rel.url)).map_err(|e| e.to_string())?),
    };
    let mut out = fs::File::create(&zip_path).map_err(|e| e.to_string())?;
    let mut hash = Sha256::new();
    let mut buf = vec![0u8; 64 * 1024];
    let mut done = 0u64;
    let mut last = Instant::now();
    loop {
        let n = reader.read(&mut buf).map_err(|e| format!("telechargement : {e}"))?;
        if n == 0 {
            break;
        }
        done += n as u64;
        if done > MAX_SIZE {
            return Err("archive trop grosse".into());
        }
        hash.update(&buf[..n]);
        out.write_all(&buf[..n]).map_err(|e| e.to_string())?;
        if last.elapsed() > Duration::from_millis(100) {
            last = Instant::now();
            set(status, Status::Downloading { version: rel.version.clone(), done, total: rel.size });
        }
    }
    out.flush().map_err(|e| e.to_string())?;
    drop(out);
    if rel.size > 0 && done != rel.size {
        return Err(format!("taille inattendue ({done} au lieu de {})", rel.size));
    }
    let got: String = hash.finalize().iter().map(|b| format!("{b:02x}")).collect();
    if let Some(want) = &rel.sha256 {
        if &got != want {
            return Err("empreinte SHA-256 differente : archive refusee".into());
        }
    }
    let new = up.join("new");
    extract(&zip_path, &new)?;
    let _ = fs::remove_file(&zip_path);
    if !new.join(PACKAGE_EXE).is_file() || !new.join("data").is_dir() {
        return Err("archive incomplete".into());
    }
    fs::write(up.join("ready"), &rel.version).map_err(|e| e.to_string())?;
    Ok(())
}

/// Extraction controlee : chemins relatifs uniquement (pas de « .. » ni de racine), dossier de
/// premier niveau commun retire, taille de chaque fichier limitee.
fn extract(zip_path: &Path, dest: &Path) -> Result<(), String> {
    let f = fs::File::open(zip_path).map_err(|e| e.to_string())?;
    let mut z = zip::ZipArchive::new(f).map_err(|e| format!("archive illisible : {e}"))?;
    let mut names = Vec::with_capacity(z.len());
    for i in 0..z.len() {
        let e = z.by_index(i).map_err(|e| e.to_string())?;
        let label = e.name().map(|n| n.to_string()).unwrap_or_default();
        let p = e.enclosed_name().ok_or_else(|| format!("chemin refuse dans l'archive : {label}"))?;
        if p.components().any(|c| !matches!(c, Component::Normal(_))) {
            return Err(format!("chemin refuse dans l'archive : {label}"));
        }
        names.push((p, e.is_dir()));
    }
    // dossier de premier niveau commun (« A Survivor to the Past/... »)
    let first = |p: &Path| p.components().next().map(|c| c.as_os_str().to_owned());
    let prefix = names.first().and_then(|(p, _)| first(p));
    let strip = prefix.is_some()
        && names.iter().all(|(p, d)| first(p) == prefix && (*d || p.components().count() > 1));
    fs::create_dir_all(dest).map_err(|e| e.to_string())?;
    for (i, (p, is_dir)) in names.into_iter().enumerate() {
        let rel: PathBuf = if strip { p.components().skip(1).collect() } else { p };
        // macOS : la racine du jeu est « .app/Contents/MacOS » ; le reste (Info.plist, lisez-moi
        // a cote de l'application) garde la version installee
        #[cfg(target_os = "macos")]
        let rel: PathBuf = {
            let c: Vec<_> = rel.components().collect();
            match c.windows(2).position(|w| w[0].as_os_str() == "Contents" && w[1].as_os_str() == "MacOS") {
                Some(i) => c[i + 2..].iter().collect(),
                None => continue,
            }
        };
        if rel.as_os_str().is_empty() {
            continue;
        }
        let target = dest.join(&rel);
        if is_dir {
            fs::create_dir_all(&target).map_err(|e| e.to_string())?;
            continue;
        }
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let mut e = z.by_index(i).map_err(|e| e.to_string())?;
        let mut out = fs::File::create(&target).map_err(|e| format!("{}: {e}", target.display()))?;
        let n = std::io::copy(&mut (&mut e).take(MAX_SIZE + 1), &mut out).map_err(|e| e.to_string())?;
        if n > MAX_SIZE {
            return Err("fichier trop gros dans l'archive".into());
        }
        // droit d'execution (executable du jeu sous Linux / macOS)
        #[cfg(unix)]
        if let Some(mode) = e.unix_mode() {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&target, fs::Permissions::from_mode(mode & 0o755 | 0o600)).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

/// Au lancement : si une mise a jour est prete, lance son installateur. Renvoie vrai s'il faut
/// quitter tout de suite (l'installateur relancera le jeu).
pub fn launch_pending(root: &Path) -> bool {
    let up = root.join(UPDATE_DIR);
    let exe = up.join("new").join(PACKAGE_EXE);
    if !up.join("ready").is_file() || !exe.is_file() {
        return false;
    }
    match std::process::Command::new(&exe).arg("--apply-update").arg(root).current_dir(up.join("new")).spawn() {
        Ok(_) => true,
        Err(e) => {
            log::warn!("installateur de mise a jour : {e}");
            let _ = fs::remove_dir_all(&up);
            false
        }
    }
}

/// Apres une installation : retire le dossier de travail (en arriere-plan : l'installateur
/// est peut-etre encore en train de se fermer).
pub fn cleanup(root: &Path) {
    let up = root.join(UPDATE_DIR);
    if !up.exists() || up.join("ready").exists() {
        return;
    }
    std::thread::spawn(move || {
        for _ in 0..50 {
            if fs::remove_dir_all(&up).is_ok() || !up.exists() {
                return;
            }
            std::thread::sleep(Duration::from_millis(200));
        }
    });
}

/// Attend que l'ancien executable soit libre (jeu ferme).
fn wait_unlocked(exe: &Path, max: Duration) -> Result<(), String> {
    let t = Instant::now();
    loop {
        if !exe.exists() || fs::OpenOptions::new().write(true).open(exe).is_ok() {
            return Ok(());
        }
        if t.elapsed() > max {
            return Err("le jeu ne s'est pas ferme".into());
        }
        std::thread::sleep(Duration::from_millis(200));
    }
}

fn remove_any(p: &Path) -> std::io::Result<()> {
    if p.is_dir() {
        fs::remove_dir_all(p)
    } else if p.exists() {
        fs::remove_file(p)
    } else {
        Ok(())
    }
}

/// Mode installateur (`--apply-update <racine>`), lance depuis `.update/new/`.
/// Ecrit `update.log` a la racine et relance le jeu dans tous les cas.
pub fn apply(root: &Path) {
    let up = root.join(UPDATE_DIR);
    let new = up.join("new");
    let old = up.join("old");
    let target = root.join(PACKAGE_EXE);
    // le marqueur part d'abord : pas de boucle si quelque chose echoue
    let _ = fs::remove_file(up.join("ready"));
    // (element de la racine, sauvegarde eventuelle) dans l'ordre des changements
    let mut done: Vec<(PathBuf, Option<PathBuf>)> = Vec::new();
    let result = (|| -> Result<(), String> {
        wait_unlocked(&target, Duration::from_secs(30))?;
        let _ = fs::remove_dir_all(&old);
        fs::create_dir_all(&old).map_err(|e| e.to_string())?;
        let swap = |name: &str, src: &Path, copy: bool, done: &mut Vec<(PathBuf, Option<PathBuf>)>| -> Result<(), String> {
            let dst = root.join(name);
            let bak = old.join(name);
            let had = dst.exists();
            if had {
                fs::rename(&dst, &bak).map_err(|e| format!("{name} : {e}"))?;
            }
            done.push((dst.clone(), had.then_some(bak)));
            if copy {
                fs::copy(src, &dst).map(|_| ()).map_err(|e| format!("{name} : {e}"))
            } else {
                fs::rename(src, &dst).map_err(|e| format!("{name} : {e}"))
            }
        };
        // l'executable (copie : c'est lui qui tourne en ce moment)
        swap(PACKAGE_EXE, &new.join(PACKAGE_EXE), true, &mut done)?;
        for d in REPLACED_DIRS {
            let src = new.join(d);
            if src.is_dir() {
                swap(d, &src, false, &mut done)?;
            }
        }
        // autres fichiers du paquet (licence, lisez-moi)
        for e in fs::read_dir(&new).map_err(|e| e.to_string())?.flatten() {
            let name = e.file_name();
            let name = name.to_string_lossy();
            if e.path().is_file() && name != PACKAGE_EXE {
                swap(&name, &e.path(), true, &mut done)?;
            }
        }
        Ok(())
    })();
    let mut log = format!("mise a jour vers la v{VERSION}\n");
    match &result {
        Ok(()) => log += "ok\n",
        Err(e) => {
            log += &format!("echec : {e}\nretour a la version precedente\n");
            for (dst, bak) in done.into_iter().rev() {
                let _ = remove_any(&dst);
                if let Some(b) = bak {
                    if let Err(e) = fs::rename(&b, &dst) {
                        log += &format!("restauration de {} impossible : {e}\n", dst.display());
                    }
                }
            }
        }
    }
    let _ = fs::write(root.join("update.log"), log);
    let _ = std::process::Command::new(&target).current_dir(root).spawn();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versions() {
        assert!(is_newer("v0.3.0", "0.2.0"));
        assert!(is_newer("0.2.1", "0.2.0"));
        assert!(is_newer("v1.0", "0.9.9"));
        assert!(!is_newer("v0.2.0", "0.2.0"));
        assert!(!is_newer("v0.1.9", "0.2.0"));
        assert!(!is_newer("nimportequoi", "0.2.0"));
        assert!(is_newer("v0.3.0-beta", "0.2.0"));
    }

    fn zip_with(dir: &Path, entries: &[&str]) -> PathBuf {
        let path = dir.join("p.zip");
        let mut z = zip::ZipWriter::new(fs::File::create(&path).unwrap());
        let opt = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
        for e in entries {
            z.start_file(*e, opt).unwrap();
            z.write_all(b"x").unwrap();
        }
        z.finish().unwrap();
        path
    }

    /// Une archive qui tente d'ecrire hors du dossier de mise a jour est refusee en entier ;
    /// une archive normale perd son dossier de premier niveau.
    #[test]
    fn extract_refuses_escaping_paths() {
        let dir = std::env::temp_dir().join(format!("astp_extract_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        for bad in ["../evil.txt", "A/../../evil.txt"] {
            let z = zip_with(&dir, &["A/data/ok.txt", bad]);
            let dest = dir.join("new");
            assert!(extract(&z, &dest).is_err(), "{bad} accepte");
            assert!(!dir.join("evil.txt").exists());
            let _ = fs::remove_dir_all(&dest);
        }
        // chemin absolu : ramene dans le dossier de destination, jamais ecrit ailleurs
        // macOS : seuls les fichiers de « .app/Contents/MacOS » sont repris
        let m = if cfg!(target_os = "macos") { "A.app/Contents/MacOS/" } else { "" };
        let z = zip_with(&dir, &[&format!("/{m}abs.txt")]);
        let dest = dir.join("new");
        extract(&z, &dest).unwrap();
        assert!(dest.join("abs.txt").is_file() && !dir.join("abs.txt").exists());
        let _ = fs::remove_dir_all(&dest);
        let z = zip_with(&dir, &[&format!("A Survivor/{m}data/ok.txt"), &format!("A Survivor/{m}game.exe")]);
        let dest = dir.join("new");
        extract(&z, &dest).unwrap();
        assert!(dest.join("data/ok.txt").is_file() && dest.join("game.exe").is_file());
        let _ = fs::remove_dir_all(&dir);
    }

    /// Linux / macOS : l'executable extrait garde son droit d'execution.
    #[cfg(unix)]
    #[test]
    fn extract_keeps_exec_bit() {
        use std::os::unix::fs::PermissionsExt;
        let dir = std::env::temp_dir().join(format!("astp_exec_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("p.zip");
        let mut z = zip::ZipWriter::new(fs::File::create(&path).unwrap());
        let opt = zip::write::SimpleFileOptions::default().unix_permissions(0o755);
        let root = if cfg!(target_os = "macos") { "A.app/Contents/MacOS/" } else { "A/" };
        z.start_file(format!("{root}{PACKAGE_EXE}"), opt).unwrap();
        z.write_all(b"x").unwrap();
        z.start_file(format!("{root}data/a.ron"), opt.unix_permissions(0o644)).unwrap();
        z.write_all(b"x").unwrap();
        z.finish().unwrap();
        let dest = dir.join("new");
        extract(&path, &dest).unwrap();
        let mode = |p: &Path| fs::metadata(p).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode(&dest.join(PACKAGE_EXE)), 0o755);
        assert_eq!(mode(&dest.join("data/a.ron")), 0o644);
        let _ = fs::remove_dir_all(&dir);
    }
}

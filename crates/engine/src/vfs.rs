//! Fichiers du jeu en lecture seule : le disque sur ordinateur ; dans le navigateur, un paquet
//! unique (`game.pak`) telecharge avant le lancement et installe par `install`.
//!
//! Format du paquet (petit-boutiste) : "ASTP" (4 octets), version u32 = 1, nombre d'entrees u32,
//! puis pour chaque entree : longueur du chemin u16, chemin UTF-8 (separateur /), taille u32, octets.
use std::path::Path;

/// Chemin normalise : separateur /, sans prefixe ./ ni / initial.
pub fn key(path: &Path) -> String {
    let s = path.to_string_lossy().replace('\\', "/");
    let mut s = s.as_str();
    while let Some(r) = s.strip_prefix("./").or_else(|| s.strip_prefix('/')) {
        s = r;
    }
    s.to_string()
}

pub fn read_string(path: &Path) -> Option<String> {
    String::from_utf8(read(path)?).ok()
}

/// Lit un paquet ; renvoie les entrees (chemin, debut, fin) dans `data`.
pub fn parse(data: &[u8]) -> Result<Vec<(String, usize, usize)>, String> {
    struct R<'a> {
        d: &'a [u8],
        at: usize,
    }
    impl<'a> R<'a> {
        fn take(&mut self, n: usize) -> Result<&'a [u8], String> {
            let s = self.d.get(self.at..self.at + n).ok_or("paquet tronque")?;
            self.at += n;
            Ok(s)
        }
        fn u32(&mut self) -> Result<u32, String> {
            let b = self.take(4)?;
            Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
        }
    }
    let mut r = R { d: data, at: 0 };
    if r.take(4)? != b"ASTP" {
        return Err("ce n'est pas un paquet du jeu".into());
    }
    let version = r.u32()?;
    if version != 1 {
        return Err(format!("version de paquet inconnue : {version}"));
    }
    let n = r.u32()? as usize;
    let mut out = Vec::with_capacity(n.min(65536));
    for _ in 0..n {
        let l = r.take(2)?;
        let len = u16::from_le_bytes([l[0], l[1]]) as usize;
        let name = String::from_utf8(r.take(len)?.to_vec()).map_err(|_| "chemin invalide")?;
        let size = r.u32()? as usize;
        let start = r.at;
        r.take(size)?;
        out.push((name, start, start + size));
    }
    Ok(out)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn read(path: &Path) -> Option<Vec<u8>> {
    std::fs::read(path).ok()
}

/// Noms des fichiers d'un dossier (tries).
#[cfg(not(target_arch = "wasm32"))]
pub fn list(dir: &Path) -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir(dir)
        .map(|rd| rd.flatten().filter(|e| e.path().is_file()).map(|e| e.file_name().to_string_lossy().to_string()).collect())
        .unwrap_or_default();
    v.sort();
    v
}

#[cfg(target_arch = "wasm32")]
mod web {
    use std::collections::HashMap;
    use std::sync::OnceLock;

    pub struct Pak {
        pub data: Vec<u8>,
        pub index: HashMap<String, (usize, usize)>,
    }

    pub static PAK: OnceLock<Pak> = OnceLock::new();
}

/// Installe le paquet telecharge par la page (une seule fois, avant le lancement du jeu).
#[cfg(target_arch = "wasm32")]
pub fn install(data: Vec<u8>) -> Result<usize, String> {
    let entries = parse(&data)?;
    let n = entries.len();
    let index = entries.into_iter().map(|(k, a, b)| (k, (a, b))).collect();
    web::PAK.set(web::Pak { data, index }).map_err(|_| "paquet deja installe".to_string())?;
    Ok(n)
}

#[cfg(target_arch = "wasm32")]
pub fn read(path: &Path) -> Option<Vec<u8>> {
    let p = web::PAK.get()?;
    let &(a, b) = p.index.get(&key(path))?;
    Some(p.data[a..b].to_vec())
}

#[cfg(target_arch = "wasm32")]
pub fn list(dir: &Path) -> Vec<String> {
    let Some(p) = web::PAK.get() else { return Vec::new() };
    let mut prefix = key(dir);
    if !prefix.is_empty() && !prefix.ends_with('/') {
        prefix.push('/');
    }
    let mut v: Vec<String> = p
        .index
        .keys()
        .filter_map(|k| k.strip_prefix(&prefix))
        .filter(|rest| !rest.contains('/'))
        .map(str::to_string)
        .collect();
    v.sort();
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_pack() {
        let mut d = b"ASTP".to_vec();
        d.extend(1u32.to_le_bytes());
        d.extend(1u32.to_le_bytes());
        d.extend(5u16.to_le_bytes());
        d.extend(b"a/b.x");
        d.extend(3u32.to_le_bytes());
        d.extend(b"xyz");
        let e = parse(&d).unwrap();
        assert_eq!(e, vec![("a/b.x".to_string(), 23, 26)]);
        assert_eq!(&d[23..26], b"xyz");
        assert!(parse(&d[..25]).is_err());
        assert_eq!(key(Path::new("./data\\maps/x.ron")), "data/maps/x.ron");
    }
}

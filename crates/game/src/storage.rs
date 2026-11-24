//! Fichiers du joueur (reglages, profil) : a cote du jeu sur ordinateur, dans le stockage du
//! navigateur pour la version web. Ecriture atomique sur disque (fichier temporaire puis
//! renommage) : un plantage pendant l'enregistrement ne corrompt jamais une sauvegarde.
use std::path::Path;

#[cfg(not(target_arch = "wasm32"))]
pub fn read(root: &Path, name: &str) -> Option<String> {
    std::fs::read_to_string(root.join(name)).ok()
}

#[cfg(not(target_arch = "wasm32"))]
pub fn write(root: &Path, name: &str, text: &str) -> Result<(), String> {
    let path = root.join(name);
    let tmp = root.join(format!("{name}.tmp"));
    std::fs::write(&tmp, text).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, &path).map_err(|e| {
        let _ = std::fs::remove_file(&tmp);
        e.to_string()
    })
}

#[cfg(target_arch = "wasm32")]
fn store() -> Option<web_sys::Storage> {
    web_sys::window()?.local_storage().ok()?
}

#[cfg(target_arch = "wasm32")]
fn key(name: &str) -> String {
    format!("a-survivor-to-the-past/{name}")
}

#[cfg(target_arch = "wasm32")]
pub fn read(_root: &Path, name: &str) -> Option<String> {
    store()?.get_item(&key(name)).ok()?
}

#[cfg(target_arch = "wasm32")]
pub fn write(_root: &Path, name: &str, text: &str) -> Result<(), String> {
    let s = store().ok_or("stockage du navigateur indisponible")?;
    s.set_item(&key(name), text).map_err(|_| "stockage du navigateur plein ou bloque".to_string())
}

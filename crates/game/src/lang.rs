//! Textes du jeu par langue (`data/lang/<code>.ron`). L'anglais sert de reference :
//! toute cle absente d'une langue est prise en anglais, puis affichee telle quelle.
use std::collections::HashMap;
use std::path::Path;

/// Langues proposees, dans l'ordre du menu.
pub const LANGS: [&str; 7] = ["en", "fr", "de", "es", "it", "pt", "ja"];

pub struct Lang {
    texts: HashMap<String, String>,
    fallback: HashMap<String, String>,
}

fn load_file(root: &Path, code: &str) -> HashMap<String, String> {
    let p = root.join("data/lang").join(format!("{code}.ron"));
    match std::fs::read_to_string(&p) {
        Ok(t) => ron::from_str(&t).unwrap_or_else(|e| {
            log::warn!("{} : {e}", p.display());
            HashMap::new()
        }),
        Err(e) => {
            log::warn!("{} : {e}", p.display());
            HashMap::new()
        }
    }
}

impl Lang {
    pub fn load(root: &Path, code: &str) -> Lang {
        let code = if LANGS.contains(&code) { code } else { "en" };
        let fallback = load_file(root, "en");
        let texts = if code == "en" { HashMap::new() } else { load_file(root, code) };
        if code != "en" {
            let mut missing: Vec<&String> = fallback.keys().filter(|k| !texts.contains_key(*k)).collect();
            missing.sort();
            if !missing.is_empty() {
                log::warn!("langue {code} : {} texte(s) manquant(s), pris en anglais : {:?}", missing.len(), missing);
            }
        }
        Lang { texts, fallback }
    }

    /// Texte d'une cle (anglais si absente de la langue, « ? » si absente partout).
    pub fn get(&self, key: &str) -> &str {
        match self.texts.get(key).or_else(|| self.fallback.get(key)) {
            Some(s) => s.as_str(),
            None => {
                log::debug!("texte absent : {key}");
                "?"
            }
        }
    }

    /// Nom affiche d'une langue (dans cette langue).
    pub fn name_of(root: &Path, code: &str) -> String {
        load_file(root, code).remove("lang.name").unwrap_or_else(|| code.to_string())
    }
}

//! Chargement du contenu (`data/`) : regles, personnages, armes, objets passifs, ennemis,
//! cartes, audio et touches.
use engine::input::Bindings;
use serde::Deserialize;
use sim::defs::{CharacterDef, Content, EnemyDef, MapDef, PassiveDef, RulesDef, WeaponDef};
use std::collections::BTreeMap;
use std::path::Path;

fn read<T: for<'de> Deserialize<'de>>(path: &Path) -> T {
    let text = engine::vfs::read_string(path).unwrap_or_else(|| panic!("{} introuvable", path.display()));
    ron::from_str(&text).unwrap_or_else(|e| panic!("{} : {e}", path.display()))
}

/// Tous les fichiers `.ron` d'un dossier, indexes par nom de fichier.
fn read_dir<T: for<'de> Deserialize<'de>>(dir: &Path) -> BTreeMap<String, T> {
    let mut out = BTreeMap::new();
    for f in engine::vfs::list(dir) {
        let p = dir.join(&f);
        if p.extension().is_some_and(|x| x == "ron") {
            let id = p.file_stem().unwrap().to_string_lossy().to_string();
            out.insert(id, read(&p));
        }
    }
    out
}

pub fn load_content(root: &Path) -> Content {
    let data = root.join("data");
    let rules: RulesDef = read(&data.join("rules.ron"));
    let mut characters: BTreeMap<String, CharacterDef> = read_dir(&data.join("characters"));
    // 0.6.5 : personnages du Randomizer, generes dans un seul fichier
    if engine::vfs::read_string(&data.join("zspr_characters.ron")).is_some() {
        characters.extend(read::<BTreeMap<String, CharacterDef>>(&data.join("zspr_characters.ron")));
    }
    for (id, c) in characters.iter_mut() {
        c.id = id.clone();
    }
    let (weapon_ids, weapons) = split(read::<BTreeMap<String, WeaponDef>>(&data.join("weapons.ron")));
    let (passive_ids, passives) = split(read::<BTreeMap<String, PassiveDef>>(&data.join("passives.ron")));
    let (enemy_ids, enemies) = split(read::<BTreeMap<String, EnemyDef>>(&data.join("enemies.ron")));
    let maps: BTreeMap<String, MapDef> = read_dir(&data.join("maps"));
    let content = Content { rules, characters, weapons, weapon_ids, passives, passive_ids, enemies, enemy_ids, maps };
    check(&content);
    log::info!(
        "contenu : {} personnage(s), {} arme(s), {} objet(s) passif(s), {} ennemi(s), {} carte(s)",
        content.characters.len(),
        content.weapons.len(),
        content.passives.len(),
        content.enemies.len(),
        content.maps.len()
    );
    content
}

fn split<T>(m: BTreeMap<String, T>) -> (Vec<String>, Vec<T>) {
    m.into_iter().unzip()
}

/// Verifie les references croisees (evolutions, invocations, vagues) et le signale au journal.
fn check(c: &Content) {
    for (id, w) in c.weapon_ids.iter().zip(&c.weapons) {
        for e in w.recipes() {
            if c.passive_index(&e.passive).is_none() {
                log::warn!("arme {id} : objet d'evolution inconnu {}", e.passive);
            }
            if c.weapon_index(&e.into).is_none() {
                log::warn!("arme {id} : evolution inconnue {}", e.into);
            }
        }
    }
    for (id, e) in c.enemy_ids.iter().zip(&c.enemies) {
        if let sim::defs::Behavior::Boss { actions } = &e.behavior {
            for a in actions {
                if let sim::defs::BossAction::Summon { enemy, .. } = a {
                    if c.enemy_index(enemy).is_none() {
                        log::warn!("boss {id} : renfort inconnu {enemy}");
                    }
                }
            }
        }
    }
    for (mid, m) in &c.maps {
        for w in &m.waves {
            if c.enemy_index(&w.enemy).is_none() {
                log::warn!("carte {mid} : ennemi inconnu {}", w.enemy);
            }
        }
    }
}

/// Touches par defaut (`data/input.ron`).
pub fn load_bindings(root: &Path) -> Bindings {
    read(&root.join("data/input.ron"))
}

fn one() -> f32 {
    1.0
}

#[derive(Deserialize, Clone, Debug)]
pub struct MusicEntry {
    pub file: String,
    #[serde(default)]
    pub loop_start: f64,
    #[serde(default)]
    pub loop_end: Option<f64>,
    #[serde(default)]
    pub once: bool,
    #[serde(default = "one")]
    pub volume: f32,
}

#[derive(Deserialize, Clone, Copy, Debug)]
pub struct SfxRuleDef {
    pub max_voices: usize,
    pub min_interval: f32,
    pub volume: f32,
    #[serde(default)]
    pub pitch_jitter: f32,
}

#[derive(Deserialize, Clone, Debug, Default)]
pub struct AudioConfig {
    pub music: BTreeMap<String, MusicEntry>,
    /// Musique de l'ecran titre et des menus hors partie.
    #[serde(default = "default_menu_music")]
    pub menu_music: String,
    /// Ambiance sonore de l'ecran titre (une entree de `music`).
    #[serde(default)]
    pub menu_ambience: Option<String>,
    #[serde(default)]
    pub sfx: BTreeMap<String, SfxRuleDef>,
    #[serde(default)]
    pub events: BTreeMap<String, Vec<String>>,
}

fn default_menu_music() -> String {
    "title_link_to_the_past".into()
}

impl AudioConfig {
    pub fn load(root: &Path) -> AudioConfig {
        read(&root.join("data/audio.ron"))
    }

    pub fn music_def(&self, id: &str) -> Option<engine::audio::MusicDef> {
        self.music.get(id).map(|m| engine::audio::MusicDef {
            file: m.file.clone(),
            loop_start: m.loop_start,
            loop_end: m.loop_end,
            looping: !m.once,
            volume: m.volume,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::AudioConfig;
    use std::path::Path;

    /// Chaque musique de stage et de boss existe, et chaque boucle tient dans son fichier.
    #[test]
    fn stage_music_loops() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let cfg = AudioConfig::load(&root);
        for (id, m) in &cfg.music {
            let path = root.join("assets/audio/music").join(&m.file);
            assert!(path.is_file(), "{id} : {}", m.file);
            if let Some(end) = m.loop_end {
                assert!(end > m.loop_start + 3.0, "{id} : boucle trop courte");
            }
        }
        for f in std::fs::read_dir(root.join("data/maps")).unwrap().flatten() {
            let s = std::fs::read_to_string(f.path()).unwrap();
            for key in ["music", "boss_music", "final_music"] {
                let pat = format!("\n    {key}: \"");
                if let Some(i) = s.find(&pat) {
                    let rest = &s[i + pat.len()..];
                    let id = &rest[..rest.find('"').unwrap()];
                    assert!(cfg.music.contains_key(id), "{:?} : {key} {id}", f.path());
                }
            }
        }
    }
}

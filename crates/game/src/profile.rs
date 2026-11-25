//! Profils des joueurs : 5 emplacements (`profile_1.ron` a `profile_5.ron`), le joueur actif dans
//! `profiles.ron`. Chaque profil garde son nom, son avatar, sa progression entre les parties (stages
//! gagnes, rubis en banque, boutique, reliques, difficulte, bestiaire, armurerie, statistiques) et ses
//! meilleurs scores par stage. Les reglages restent dans `settings.ron`.
use crate::storage;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

/// Ancien fichier unique (avant la 0.6), repris dans le premier emplacement.
const LEGACY: &str = "profile.ron";
const INDEX: &str = "profiles.ron";
const VERSION: u32 = 2;
/// Nombre d'emplacements de joueurs.
pub const SLOTS: usize = 5;
/// Meilleures parties gardees par stage.
pub const KEEP_SCORES: usize = 5;
/// Nom par defaut (celui du heros d'A Link to the Past).
pub const DEFAULT_NAME: &str = "LINK";

fn slot_file(slot: usize) -> String {
    format!("profile_{}.ron", slot + 1)
}

/// Une partie notee (meilleurs scores par stage).
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct ScoreRecord {
    pub score: u64,
    /// Temps de partie (s).
    #[serde(default)]
    pub time: f32,
    #[serde(default)]
    pub kills: u32,
    #[serde(default)]
    pub level: u32,
    #[serde(default)]
    pub bosses: u32,
    #[serde(default)]
    pub won: bool,
    #[serde(default)]
    pub character: String,
    #[serde(default)]
    pub difficulty: usize,
    /// Multiplicateur de score (difficulte et Sceaux).
    #[serde(default)]
    pub mult: f32,
    /// Version du jeu de la partie.
    #[serde(default)]
    pub version: String,
    /// Deja publie dans le classement en ligne.
    #[serde(default)]
    pub published: bool,
}

/// Index des profils : emplacement du joueur actif.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
struct Index {
    #[serde(default)]
    active: usize,
}

/// Records d'un stage.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct StageRecord {
    /// Victoires (porte franchie).
    #[serde(default)]
    pub wins: u32,
    /// Parties jouees.
    #[serde(default)]
    pub runs: u32,
    /// Meilleur temps de victoire (s), 0 = jamais gagne.
    #[serde(default)]
    pub best_time: f32,
    /// Plus haute difficulte gagnee (indice, -1 = aucune).
    #[serde(default = "minus_one")]
    pub best_difficulty: i32,
}

fn minus_one() -> i32 {
    -1
}

/// Statistiques cumulees.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Totals {
    #[serde(default)]
    pub runs: u32,
    #[serde(default)]
    pub wins: u32,
    #[serde(default)]
    pub deaths: u32,
    #[serde(default)]
    pub kills: u64,
    #[serde(default)]
    pub bosses: u32,
    #[serde(default)]
    pub elites: u32,
    /// Temps de jeu (s).
    #[serde(default)]
    pub time: f64,
    /// Rubis ramasses en partie.
    #[serde(default)]
    pub rupees: u64,
    /// Rubis gagnes pour la banque.
    #[serde(default)]
    pub banked: u64,
    #[serde(default)]
    pub best_level: u32,
    #[serde(default)]
    pub chests: u32,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Profile {
    #[serde(default)]
    pub version: u32,
    #[serde(default)]
    pub stages: BTreeMap<String, StageRecord>,
    /// Rubis en banque (Boutique de Kakariko, Grande Fee).
    #[serde(default)]
    pub rupees: u64,
    /// Ameliorations permanentes achetees : identifiant -> niveau.
    #[serde(default)]
    pub shop: BTreeMap<String, u32>,
    /// Reliques trouvees, et celles equipees pour la prochaine partie.
    #[serde(default)]
    pub relics: BTreeSet<String>,
    #[serde(default)]
    pub equipped: Vec<String>,
    /// Benedictions de la Grande Fee debloquees.
    #[serde(default)]
    pub blessings: BTreeSet<String>,
    /// Difficulte choisie (indice dans `difficulty.ron`).
    #[serde(default = "d_difficulty")]
    pub difficulty: usize,
    /// Sceaux de Ganon actifs.
    #[serde(default)]
    pub seals: BTreeSet<String>,
    /// Bestiaire : ennemis vaincus par type.
    #[serde(default)]
    pub bestiary: BTreeMap<String, u64>,
    /// Armurerie : armes et objets choisis (nombre de parties), evolutions decouvertes.
    #[serde(default)]
    pub armory: BTreeMap<String, u32>,
    #[serde(default)]
    pub evolutions: BTreeSet<String>,
    #[serde(default)]
    pub totals: Totals,
    /// Dernier personnage choisi.
    #[serde(default)]
    pub character: String,
    /// Couleur choisie pour les personnages qui en proposent plusieurs (indice dans `skins`).
    #[serde(default)]
    pub skins: BTreeMap<String, usize>,
    /// Nom du joueur et avatar (indice dans `players::AVATARS`).
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub avatar: usize,
    /// Pseudo utilise pour le classement en ligne (le nom par defaut).
    #[serde(default)]
    pub pseudo: String,
    /// Meilleures parties par stage (du meilleur score au moins bon).
    #[serde(default)]
    pub scores: BTreeMap<String, Vec<ScoreRecord>>,
    /// Emplacement du profil (non enregistre : c'est le nom du fichier).
    #[serde(skip)]
    pub slot: usize,
}

fn d_difficulty() -> usize {
    1
}

impl Profile {
    /// Profil du joueur actif (reprend l'ancien `profile.ron` au premier lancement de la 0.6).
    pub fn load(root: &Path) -> Profile {
        let active = active_slot(root);
        if active == 0 && storage::read(root, &slot_file(0)).is_none() {
            if let Some(t) = storage::read(root, LEGACY) {
                if let Ok(mut p) = ron::from_str::<Profile>(&t) {
                    p.slot = 0;
                    p.version = VERSION;
                    if p.name.is_empty() {
                        p.name = DEFAULT_NAME.to_string();
                    }
                    p.save(root);
                    // l'ancien fichier reste en copie de securite sous un autre nom
                    let _ = storage::write(root, "profile.old.ron", &t);
                    storage::remove(root, LEGACY);
                    return p;
                }
            }
        }
        Self::load_slot(root, active).unwrap_or_else(|| Profile::fresh(active, DEFAULT_NAME, 0))
    }

    /// Nouveau profil vide dans un emplacement.
    pub fn fresh(slot: usize, name: &str, avatar: usize) -> Profile {
        let mut p = Profile::new();
        p.slot = slot;
        p.name = name.to_string();
        p.avatar = avatar;
        p
    }

    /// Profil d'un emplacement, s'il existe.
    pub fn load_slot(root: &Path, slot: usize) -> Option<Profile> {
        let name = slot_file(slot);
        let t = storage::read(root, &name)?;
        let mut p = match ron::from_str::<Profile>(&t) {
            Ok(p) => p,
            Err(e) => {
                log::error!("{name} illisible ({e}) : copie de secours");
                let _ = storage::write(root, &format!("profile_{}.broken.ron", slot + 1), &t);
                return None;
            }
        };
        p.version = VERSION;
        p.slot = slot;
        if p.name.is_empty() {
            p.name = DEFAULT_NAME.to_string();
        }
        Some(p)
    }

    pub fn new() -> Profile {
        Profile { version: VERSION, difficulty: d_difficulty(), ..Default::default() }
    }

    /// Ajoute une partie aux meilleurs scores du stage ; renvoie sa place (0 = record) si elle est gardee.
    pub fn add_score(&mut self, map: &str, rec: ScoreRecord) -> Option<usize> {
        let list = self.scores.entry(map.to_string()).or_default();
        let k = list.iter().position(|r| rec.score > r.score).unwrap_or(list.len());
        if k >= KEEP_SCORES {
            return None;
        }
        list.insert(k, rec);
        list.truncate(KEEP_SCORES);
        Some(k)
    }

    /// Meilleur score sur un stage.
    pub fn best_score(&self, map: &str) -> Option<&ScoreRecord> {
        self.scores.get(map).and_then(|l| l.first())
    }

    pub fn to_text(&self) -> String {
        let cfg = ron::ser::PrettyConfig::new().depth_limit(3);
        let body = ron::ser::to_string_pretty(self, cfg).unwrap_or_default();
        format!("// Profil du joueur : progression, boutique, bestiaire. Ne pas modifier pendant une partie.\n{body}\n")
    }

    pub fn save(&self, root: &Path) {
        if let Err(e) = storage::write(root, &slot_file(self.slot), &self.to_text()) {
            log::warn!("enregistrement du profil impossible : {e}");
        }
    }

    pub fn won(&self, stage: &str) -> bool {
        self.stages.get(stage).is_some_and(|r| r.wins > 0)
    }

    pub fn shop_level(&self, id: &str) -> u32 {
        self.shop.get(id).copied().unwrap_or(0)
    }

    /// Code d'export : le profil entier, en texte copiable (base64).
    pub fn export_code(&self) -> String {
        format!("ASTP1:{}", b64::encode(self.to_text().as_bytes()))
    }

    /// Lit un code d'export ; None s'il est invalide.
    pub fn import_code(code: &str) -> Option<Profile> {
        let body = code.trim().strip_prefix("ASTP1:")?;
        let bytes = b64::decode(body)?;
        let text = String::from_utf8(bytes).ok()?;
        let mut p: Profile = ron::from_str(&text).ok()?;
        p.version = VERSION;
        if p.name.is_empty() {
            p.name = DEFAULT_NAME.to_string();
        }
        Some(p)
    }
}

/// Emplacement du joueur actif.
pub fn active_slot(root: &Path) -> usize {
    storage::read(root, INDEX).and_then(|t| ron::from_str::<Index>(&t).ok()).map(|i| i.active).unwrap_or(0).min(SLOTS - 1)
}

pub fn set_active(root: &Path, slot: usize) {
    let body = ron::to_string(&Index { active: slot }).unwrap_or_default();
    let text = format!("// Joueur actif (emplacement 0 a {}).\n{body}\n", SLOTS - 1);
    if let Err(e) = storage::write(root, INDEX, &text) {
        log::warn!("enregistrement du joueur actif impossible : {e}");
    }
}

/// Efface un emplacement.
pub fn delete_slot(root: &Path, slot: usize) {
    storage::remove(root, &slot_file(slot));
}

/// Les 5 emplacements (None = libre).
pub fn all_slots(root: &Path) -> Vec<Option<Profile>> {
    (0..SLOTS).map(|k| Profile::load_slot(root, k)).collect()
}

/// Score d'une partie : ennemis x10 + secondes x10 + boss x1000 + victoire 5000, puis le multiplicateur
/// (difficulte et Sceaux de Ganon).
pub fn score(kills: u32, time: f32, bosses: u32, won: bool, mult: f32) -> u64 {
    let base = kills as u64 * 10 + (time.max(0.0) as u64) * 10 + bosses as u64 * 1000 + if won { 5000 } else { 0 };
    (base as f64 * mult.max(0.0) as f64).round() as u64
}

/// Base64 standard (sans dependance).
mod b64 {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

    pub fn encode(data: &[u8]) -> String {
        let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
        for c in data.chunks(3) {
            let n = (c[0] as u32) << 16 | (*c.get(1).unwrap_or(&0) as u32) << 8 | *c.get(2).unwrap_or(&0) as u32;
            out.push(T[(n >> 18) as usize & 63] as char);
            out.push(T[(n >> 12) as usize & 63] as char);
            out.push(if c.len() > 1 { T[(n >> 6) as usize & 63] as char } else { '=' });
            out.push(if c.len() > 2 { T[n as usize & 63] as char } else { '=' });
        }
        out
    }

    pub fn decode(s: &str) -> Option<Vec<u8>> {
        let mut out = Vec::with_capacity(s.len() / 4 * 3);
        let mut buf = 0u32;
        let mut bits = 0;
        for ch in s.bytes() {
            if ch == b'=' {
                break;
            }
            if ch.is_ascii_whitespace() {
                continue;
            }
            let v = T.iter().position(|&t| t == ch)? as u32;
            buf = buf << 6 | v;
            bits += 6;
            if bits >= 8 {
                bits -= 8;
                out.push((buf >> bits) as u8);
                buf &= (1 << bits) - 1;
            }
        }
        Some(out)
    }

    #[cfg(test)]
    mod tests {
        #[test]
        fn roundtrip() {
            for s in ["", "a", "ab", "abc", "abcd", "Profil é ✓"] {
                assert_eq!(super::decode(&super::encode(s.as_bytes())).unwrap(), s.as_bytes());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn score_formula() {
        assert_eq!(score(100, 600.9, 2, true, 1.0), 1000 + 6000 + 2000 + 5000);
        assert_eq!(score(100, 600.0, 2, true, 1.5), 21000);
        assert_eq!(score(0, 0.0, 0, false, 2.0), 0);
    }

    #[test]
    fn best_scores_kept_in_order() {
        let mut p = Profile::new();
        let rec = |s| ScoreRecord { score: s, ..Default::default() };
        assert_eq!(p.add_score("a", rec(10)), Some(0));
        assert_eq!(p.add_score("a", rec(30)), Some(0));
        assert_eq!(p.add_score("a", rec(20)), Some(1));
        for s in [5, 6, 7] {
            p.add_score("a", rec(s));
        }
        assert_eq!(p.add_score("a", rec(1)), None);
        let l: Vec<u64> = p.scores["a"].iter().map(|r| r.score).collect();
        assert_eq!(l, vec![30, 20, 10, 7, 6]);
        assert_eq!(p.best_score("a").map(|r| r.score), Some(30));
        assert!(p.best_score("b").is_none());
    }

    /// Ancien profil unique repris dans le premier emplacement, puis plusieurs joueurs.
    #[test]
    fn slots_and_legacy_profile() {
        let dir = std::env::temp_dir().join(format!("astp_profiles_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let mut old = Profile::new();
        old.rupees = 1234;
        std::fs::write(dir.join(LEGACY), old.to_text()).unwrap();
        let p = Profile::load(&dir);
        assert_eq!((p.slot, p.rupees, p.name.as_str()), (0, 1234, DEFAULT_NAME));
        assert!(!dir.join(LEGACY).exists());
        assert!(dir.join("profile_1.ron").exists() && dir.join("profile.old.ron").exists());
        let q = Profile::fresh(3, "ZELDA", 1);
        q.save(&dir);
        set_active(&dir, 3);
        let q2 = Profile::load(&dir);
        assert_eq!((q2.slot, q2.name.as_str(), q2.avatar), (3, "ZELDA", 1));
        let slots = all_slots(&dir);
        assert!(slots[0].is_some() && slots[3].is_some() && slots[1].is_none());
        delete_slot(&dir, 3);
        assert!(Profile::load_slot(&dir, 3).is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }
}

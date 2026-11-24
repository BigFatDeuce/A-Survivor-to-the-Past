//! Profil du joueur (`profile.ron`) : progression entre les parties.
//! Stages gagnes et records, rubis en banque et achats de la boutique, reliques, difficulte,
//! bestiaire, armurerie et statistiques. Les reglages restent dans `settings.ron`.
use crate::storage;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

const FILE: &str = "profile.ron";
const VERSION: u32 = 1;

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
}

fn d_difficulty() -> usize {
    1
}

impl Profile {
    pub fn load(root: &Path) -> Profile {
        let mut p = match storage::read(root, FILE) {
            Some(t) => ron::from_str(&t).unwrap_or_else(|e| {
                log::error!("profile.ron illisible ({e}) : copie de secours puis nouveau profil");
                let _ = storage::write(root, "profile.broken.ron", &t);
                Profile::new()
            }),
            None => Profile::new(),
        };
        p.version = VERSION;
        p
    }

    pub fn new() -> Profile {
        Profile { version: VERSION, difficulty: d_difficulty(), ..Default::default() }
    }

    pub fn to_text(&self) -> String {
        let cfg = ron::ser::PrettyConfig::new().depth_limit(3);
        let body = ron::ser::to_string_pretty(self, cfg).unwrap_or_default();
        format!("// Profil du joueur : progression, boutique, bestiaire. Ne pas modifier pendant une partie.\n{body}\n")
    }

    pub fn save(&self, root: &Path) {
        if let Err(e) = storage::write(root, FILE, &self.to_text()) {
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
        Some(p)
    }
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

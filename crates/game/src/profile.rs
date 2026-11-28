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
const VERSION: u32 = 5;
/// Nombre d'emplacements de joueurs.
pub const SLOTS: usize = 5;
/// Meilleures parties gardees par stage.
pub const KEEP_SCORES: usize = 5;
/// « Mes parties » : une partie plus courte n'est pas gardee (retour 0.6.1).
pub const MIN_RUN_TIME: f32 = 30.0;
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
    /// Poids de vie des ennemis (`Meta::kill_weight`) ; absent avant la 0.9 (poids 1).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kw: Option<f32>,
    /// Version du jeu de la partie.
    #[serde(default)]
    pub version: String,
    /// Deja publie dans le classement en ligne.
    #[serde(default)]
    pub published: bool,
    /// Coop : pseudos de l'equipe dans l'ordre de la partie (vide : partie solo) ; `character` liste alors
    /// les personnages, separes par des virgules.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub team: Vec<String>,
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
    /// Personnages debloques mais pas encore joues : nom en vert dans le choix des personnages.
    #[serde(default)]
    pub new_chars: BTreeSet<String>,
    /// Armes debloquees (celles qui demandent de vaincre un mini-boss).
    #[serde(default)]
    pub weapons: BTreeSet<String>,
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
    /// 0.9 : meilleures parties de coop, par stage et par nombre de joueurs (`coop_key`).
    #[serde(default)]
    pub coop_scores: BTreeMap<String, Vec<ScoreRecord>>,
    /// 0.6.3 : tenues achetees (sprites), musiques du juke-box et celle de l'ecran titre (vide : par
    /// defaut), offrandes a la Grande Fee (nombre de titres), figurines du bestiaire (ennemis).
    #[serde(default)]
    pub wardrobe: BTreeSet<String>,
    /// 0.6.5 : personnages achetes a la boutique.
    #[serde(default)]
    pub chars_bought: BTreeSet<String>,
    #[serde(default)]
    pub jukebox: BTreeSet<String>,
    #[serde(default)]
    pub title_music: String,
    #[serde(default)]
    pub title: u32,
    #[serde(default)]
    pub figurines: BTreeSet<String>,
    /// Conseils de la fee (apres deux defaites) deja donnes.
    #[serde(default)]
    pub fairy_tip: bool,
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
                    p.migrate();
                    if p.name.is_empty() {
                        p.name = DEFAULT_NAME.to_string();
                    }
                    // l'ancien fichier n'est retire qu'une fois le nouveau et sa copie de securite ecrits
                    let saved = p.try_save(root).and_then(|_| storage::write(root, "profile.old.ron", &t));
                    match saved {
                        Ok(()) => storage::remove(root, LEGACY),
                        Err(e) => log::warn!("ancien profil garde tel quel : {e}"),
                    }
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
        p.migrate();
        p.slot = slot;
        if p.name.is_empty() {
            p.name = DEFAULT_NAME.to_string();
        }
        // anciennes parties trop courtes (0.6.0) : retirees de « Mes parties »
        for list in p.scores.values_mut() {
            list.retain(|r| r.time >= MIN_RUN_TIME);
        }
        Some(p)
    }

    /// Met a jour un profil d'une version precedente.
    fn migrate(&mut self) {
        if self.version < 3 {
            // 0.6.3 : l'ancienne fusion « Master Sword » (epee + receptacle) s'appelle l'Epee de combat ;
            // la Master Sword devient la fusion ultime, offerte a ceux qui ont deja vaincu Ganon.
            if self.evolutions.remove("master_sword") {
                self.evolutions.insert("combat_sword".to_string());
            }
            if let Some(n) = self.armory.remove("master_sword") {
                *self.armory.entry("combat_sword".to_string()).or_default() += n;
            }
            if self.bestiary.get("ganon").is_some_and(|&n| n > 0) {
                self.weapons.insert("master_sword".to_string());
            }
        }
        if self.version < 4 {
            // 0.6.4 : le voleur de rubis n'est plus un Pikit mais un Boo (victoires et figurine gardees)
            if let Some(n) = self.bestiary.remove("pikit") {
                *self.bestiary.entry("boo".to_string()).or_default() += n;
            }
            if self.figurines.remove("pikit") {
                self.figurines.insert("boo".to_string());
            }
        }
        // 0.8 : la couleur rouge de Bowsette a son propre sprite (elle reprenait celui de Bowsette Red).
        // Sans condition de version : un code importe en 0.8.0 etait marque v5 sans cette migration, et
        // l'ancien nom n'est jamais une tenue valide (c'est le sprite de base d'un autre personnage).
        if self.wardrobe.remove("zspr_bowsette_red") {
            self.wardrobe.insert("zspr_bowsette_red_skin".to_string());
        }
        self.version = VERSION;
    }

    pub fn new() -> Profile {
        Profile { version: VERSION, difficulty: d_difficulty(), ..Default::default() }
    }

    /// Ajoute une partie aux meilleurs scores du stage (coop : a ceux de son nombre de joueurs) ; renvoie
    /// sa place (0 = record) si elle est gardee.
    pub fn add_score(&mut self, map: &str, rec: ScoreRecord) -> Option<usize> {
        let list = match rec.team.len() {
            0 | 1 => self.scores.entry(map.to_string()).or_default(),
            n => self.coop_scores.entry(coop_key(map, n)).or_default(),
        };
        let k = list.iter().position(|r| rec.score > r.score).unwrap_or(list.len());
        if k >= KEEP_SCORES {
            return None;
        }
        list.insert(k, rec);
        list.truncate(KEEP_SCORES);
        Some(k)
    }

    /// Meilleures parties d'un stage : solo (`players` 1) ou coop a `players`.
    pub fn score_list(&self, map: &str, players: usize) -> &[ScoreRecord] {
        let l = if players > 1 { self.coop_scores.get(&coop_key(map, players)) } else { self.scores.get(map) };
        l.map_or(&[], |v| v.as_slice())
    }

    pub fn score_list_mut(&mut self, map: &str, players: usize) -> Option<&mut Vec<ScoreRecord>> {
        if players > 1 {
            self.coop_scores.get_mut(&coop_key(map, players))
        } else {
            self.scores.get_mut(map)
        }
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
        if let Err(e) = self.try_save(root) {
            log::warn!("enregistrement du profil impossible : {e}");
        }
    }

    pub fn try_save(&self, root: &Path) -> Result<(), String> {
        storage::write(root, &slot_file(self.slot), &self.to_text())
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
        // code d'une version precedente : memes migrations qu'un profil lu sur le disque
        p.migrate();
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

/// Score d'une partie : ennemis x10 x poids de vie + secondes x10 + boss x1000 + victoire 5000, puis le
/// multiplicateur (difficulte et Sceaux de Ganon). Poids en centiemes, calcul entier : le serveur refait
/// exactement le meme (worker/index.js, expectedScore).
pub fn score(kills: u32, time: f32, bosses: u32, won: bool, mult: f32, kw: Option<f32>) -> u64 {
    let kwc = kw.map_or(100, |w| (w.max(0.0) * 100.0).round() as u64);
    let base = kills as u64 * kwc / 10 + (time.max(0.0) as u64) * 10 + bosses as u64 * 1000 + if won { 5000 } else { 0 };
    (base as f64 * mult.max(0.0) as f64).round() as u64
}

/// Cle des parties de coop : stage et nombre de joueurs.
pub fn coop_key(map: &str, players: usize) -> String {
    format!("{map}/{players}")
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
        assert_eq!(score(100, 600.9, 2, true, 1.0, None), 1000 + 6000 + 2000 + 5000);
        assert_eq!(score(100, 600.0, 2, true, 1.5, None), 21000);
        assert_eq!(score(0, 0.0, 0, false, 2.0, None), 0);
        // poids de vie : Facile 0,7 ; Heros + Sceau de la Force 2 x 1,5 (f32 : 2,9999998)
        assert_eq!(score(100, 0.0, 0, false, 1.0, Some(0.7)), 700);
        assert_eq!(score(101, 0.0, 0, false, 1.0, Some(2.0f32 * 1.5f32)), 3030);
        assert_eq!(score(3, 0.0, 0, false, 1.0, Some(1.4f32 * 1.5f32)), 63);
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

    #[test]
    fn old_master_sword_becomes_combat_sword() {
        let mut p = Profile { version: 2, ..Default::default() };
        p.evolutions.insert("master_sword".into());
        p.armory.insert("master_sword".into(), 4);
        p.bestiary.insert("ganon".into(), 1);
        p.migrate();
        assert!(p.evolutions.contains("combat_sword") && !p.evolutions.contains("master_sword"));
        assert_eq!(p.armory.get("combat_sword"), Some(&4));
        assert!(p.weapons.contains("master_sword"));
        // deja a jour : rien ne bouge (la Master Sword gagnee reste une vraie fusion ultime)
        p.evolutions.insert("master_sword".into());
        p.migrate();
        assert!(p.evolutions.contains("master_sword"));
    }

    /// Code exporte par une version precedente : migre comme un profil sur le disque.
    #[test]
    fn old_export_code_is_migrated() {
        let mut old = Profile { version: 4, ..Default::default() };
        old.wardrobe.insert("zspr_bowsette_red".into());
        old.bestiary.insert("pikit".into(), 3);
        let code = format!("ASTP1:{}", b64::encode(old.to_text().replace("version: 4", "version: 2").as_bytes()));
        let p = Profile::import_code(&code).expect("code valide");
        assert_eq!(p.version, VERSION);
        assert!(p.wardrobe.contains("zspr_bowsette_red_skin") && !p.wardrobe.contains("zspr_bowsette_red"));
        assert_eq!(p.bestiary.get("boo"), Some(&3));
        // deja importe en 0.8.0 (marque v5 sans migration) : corrige au chargement suivant
        let mut bad = Profile { version: VERSION, ..Default::default() };
        bad.wardrobe.insert("zspr_bowsette_red".into());
        bad.migrate();
        assert!(bad.wardrobe.contains("zspr_bowsette_red_skin"));
    }

    #[test]
    fn bowsette_red_color_keeps_its_purchase() {
        let mut p = Profile { version: 4, ..Default::default() };
        p.wardrobe.insert("zspr_bowsette_red".into());
        p.migrate();
        assert!(p.wardrobe.contains("zspr_bowsette_red_skin") && !p.wardrobe.contains("zspr_bowsette_red"));
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

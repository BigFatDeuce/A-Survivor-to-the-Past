//! Progression entre les parties (`data/meta.ron`) : Boutique de Kakariko, Grande Fee, reliques,
//! difficultes et Sceaux de Ganon. Transforme le profil du joueur en modificateurs de partie.
use crate::profile::Profile;
use serde::Deserialize;
use sim::defs::Stat;
use sim::mods::RunMods;
use std::path::Path;

#[derive(Clone, Debug, Deserialize)]
pub struct Difficulty {
    pub id: String,
    pub enemy_hp: f32,
    pub enemy_damage: f32,
    pub spawn_rate: f32,
    pub rupees: f32,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ShopItem {
    pub id: String,
    pub icon: String,
    /// Statistique amelioree (None : Avarice, plus de rubis en banque).
    pub stat: Option<Stat>,
    /// Relances ou passes en plus a la montee de niveau (per_level par niveau achete).
    #[serde(default)]
    pub perk: Option<Perk>,
    pub per_level: f32,
    pub max: u32,
    pub cost: u32,
    pub growth: f32,
}

/// Bonus de boutique hors statistiques : choix de la montee de niveau.
#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
pub enum Perk {
    Reroll,
    Skip,
}

#[derive(Clone, Copy, Debug, Deserialize)]
pub enum BlessingEffect {
    /// Une fee en bouteille de plus.
    Revive,
    /// Experience en plus (+0.15 = +15 %).
    Xp(f32),
    /// Soin des coeurs en plus (+0.5 = +50 %).
    Heal(f32),
    /// La sorciere passe une fois par chaque partie et vend ses potions.
    Witch,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Blessing {
    pub id: String,
    pub icon: String,
    pub cost: u32,
    pub effect: BlessingEffect,
    #[serde(default)]
    pub requires: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Relic {
    pub id: String,
    pub icon: String,
    /// Stage dont la premiere victoire donne la relique (vide pour une relique de rencontre).
    #[serde(default)]
    pub stage: String,
    /// Proposee par les champions errants ; gardee si la partie est gagnee.
    #[serde(default)]
    pub encounter: bool,
    pub stats: Vec<(Stat, f32)>,
    /// Relances / passes en plus a la montee de niveau.
    #[serde(default)]
    pub rerolls: u32,
    #[serde(default)]
    pub skips: u32,
}

fn one() -> f32 {
    1.0
}

#[derive(Clone, Debug, Deserialize)]
pub struct Seal {
    pub id: String,
    pub icon: String,
    #[serde(default = "one")]
    pub enemy_hp: f32,
    #[serde(default = "one")]
    pub enemy_damage: f32,
    #[serde(default = "one")]
    pub enemy_speed: f32,
    #[serde(default = "one")]
    pub spawn_rate: f32,
    #[serde(default = "one")]
    pub heal: f32,
    /// Rubis en banque en plus (+0.25 = +25 %).
    pub bonus: f32,
}

/// Garde-robe (0.6.3) : tenue achetee une fois, choisie ensuite comme une couleur du personnage.
/// `sprite` est une entree de `skins` du personnage.
#[derive(Clone, Debug, Deserialize)]
pub struct WardrobeItem {
    pub character: String,
    pub sprite: String,
    /// Icone de la ligne ; vide : la tenue elle-meme (0.6.4).
    #[serde(default)]
    pub icon: String,
    pub cost: u64,
}

/// Juke-box (0.6.3) : musique a acheter, puis a choisir pour l'ecran titre.
#[derive(Clone, Debug, Deserialize)]
pub struct Track {
    pub id: String,
    pub cost: u64,
}

/// Offrande a la Grande Fee (0.6.3) : titre affiche au classement en ligne (pseudo en couleur).
/// Les titres s'achetent dans l'ordre ; seul le plus haut compte.
#[derive(Clone, Debug, Deserialize)]
pub struct Title {
    pub id: String,
    pub cost: u64,
}

#[derive(Clone, Debug, Deserialize, Default)]
pub struct Meta {
    pub difficulties: Vec<Difficulty>,
    pub shop: Vec<ShopItem>,
    pub blessings: Vec<Blessing>,
    pub max_relics: usize,
    pub relics: Vec<Relic>,
    pub seals: Vec<Seal>,
    #[serde(default)]
    pub wardrobe: Vec<WardrobeItem>,
    #[serde(default)]
    pub jukebox: Vec<Track>,
    #[serde(default)]
    pub titles: Vec<Title>,
    /// Figurines du bestiaire : prix d'un ennemi, d'un boss, du boss final (Ganon).
    #[serde(default)]
    pub figurines: (u64, u64, u64),
}

impl Meta {
    pub fn load(root: &Path) -> Meta {
        let p = root.join("data/meta.ron");
        match engine::vfs::read_string(&p) {
            Some(t) => ron::from_str(&t).unwrap_or_else(|e| panic!("{} : {e}", p.display())),
            None => {
                log::warn!("{} illisible : pas de progression", p.display());
                Meta::default()
            }
        }
    }

    /// Tenue de la garde-robe pas encore achetee (sprite d'une couleur de personnage).
    pub fn skin_locked(&self, profile: &Profile, sprite: &str) -> bool {
        self.wardrobe.iter().any(|w| w.sprite == sprite) && !profile.wardrobe.contains(sprite)
    }

    /// Prix de la figurine d'un ennemi.
    pub fn figurine_cost(&self, enemy: &sim::defs::EnemyDef, id: &str) -> u64 {
        let (normal, boss, last) = self.figurines;
        if id == "ganon" {
            last
        } else if enemy.boss {
            boss
        } else {
            normal
        }
    }

    pub fn difficulty(&self, profile: &Profile) -> Option<&Difficulty> {
        self.difficulties.get(profile.difficulty.min(self.difficulties.len().saturating_sub(1)))
    }

    /// Prix du prochain niveau (None : niveau maximum atteint).
    pub fn shop_cost(&self, item: &ShopItem, level: u32) -> Option<u64> {
        if level >= item.max {
            return None;
        }
        let c = item.cost as f32 * item.growth.powi(level as i32);
        Some(((c / 10.0).round() * 10.0) as u64)
    }

    /// Benediction achetable : pas encore obtenue et prerequis rempli.
    pub fn blessing_open(&self, b: &Blessing, profile: &Profile) -> bool {
        b.requires.as_ref().is_none_or(|r| profile.blessings.contains(r))
    }

    /// Modificateurs de la prochaine partie.
    pub fn run_mods(&self, profile: &Profile) -> RunMods {
        let mut m = RunMods::default();
        for item in &self.shop {
            let lv = profile.shop_level(&item.id);
            if let (Some(s), true) = (item.stat, lv > 0) {
                m.stats.push((s, item.per_level * lv as f32));
            }
            let n = (item.per_level * lv as f32).round() as u32;
            match item.perk {
                Some(Perk::Reroll) => m.rerolls += n,
                Some(Perk::Skip) => m.skips += n,
                None => {}
            }
        }
        for b in &self.blessings {
            if !profile.blessings.contains(&b.id) {
                continue;
            }
            match b.effect {
                BlessingEffect::Revive => m.revives += 1,
                BlessingEffect::Xp(x) => m.xp += x,
                BlessingEffect::Heal(h) => m.heal += h,
                BlessingEffect::Witch => m.witch = true,
            }
        }
        for r in &self.relics {
            if profile.equipped.contains(&r.id) && profile.relics.contains(&r.id) {
                m.stats.extend(r.stats.iter().copied());
                m.rerolls += r.rerolls;
                m.skips += r.skips;
            }
        }
        if let Some(d) = self.difficulty(profile) {
            m.enemy_hp *= d.enemy_hp;
            m.enemy_damage *= d.enemy_damage;
            m.spawn_rate *= d.spawn_rate;
        }
        for s in &self.seals {
            if !profile.seals.contains(&s.id) {
                continue;
            }
            m.enemy_hp *= s.enemy_hp;
            m.enemy_damage *= s.enemy_damage;
            m.enemy_speed *= s.enemy_speed;
            m.spawn_rate *= s.spawn_rate;
            m.heal *= s.heal;
        }
        m
    }

    /// Multiplicateur des rubis mis en banque (difficulte, Sceaux, Avarice).
    pub fn bank_mult(&self, profile: &Profile) -> f32 {
        let d = self.difficulty(profile).map(|d| d.rupees).unwrap_or(1.0);
        let seals: f32 = self.seals.iter().filter(|s| profile.seals.contains(&s.id)).map(|s| s.bonus).sum();
        let greed: f32 = self.shop.iter().filter(|i| i.stat.is_none() && i.perk.is_none()).map(|i| i.per_level * profile.shop_level(&i.id) as f32).sum();
        d * (1.0 + seals) * (1.0 + greed)
    }

    /// Multiplicateur de score : difficulte et Sceaux de Ganon (sans l'Avarice, achetee en boutique).
    pub fn score_mult(&self, profile: &Profile) -> f32 {
        let d = self.difficulty(profile).map(|d| d.rupees).unwrap_or(1.0);
        let seals: f32 = self.seals.iter().filter(|s| profile.seals.contains(&s.id)).map(|s| s.bonus).sum();
        d * (1.0 + seals)
    }

    /// Relique gagnee par la premiere victoire de ce stage.
    pub fn relic_of(&self, stage: &str) -> Option<&Relic> {
        self.relics.iter().find(|r| !r.encounter && r.stage == stage)
    }
}

/// Effets d'une relique : statistiques, puis relances et passes.
pub fn relic_fx(lang: &crate::lang::Lang, r: &Relic) -> Vec<String> {
    let mut fx: Vec<String> = r.stats.iter().map(|&(s, v)| stat_text(lang, s, v)).collect();
    if r.rerolls > 0 {
        fx.push(perk_text(lang, Perk::Reroll, r.rerolls));
    }
    if r.skips > 0 {
        fx.push(perk_text(lang, Perk::Skip, r.skips));
    }
    fx
}

/// « +1 Relance », « +1 Passe ».
pub fn perk_text(lang: &crate::lang::Lang, p: Perk, n: u32) -> String {
    let key = match p {
        Perk::Reroll => "stat.rerolls",
        Perk::Skip => "stat.skips",
    };
    format!("+{n} {}", lang.get(key))
}

/// Texte court d'un bonus de statistique (« +10 % Puissance »), noms `stat.<nom>`.
pub fn stat_text(lang: &crate::lang::Lang, s: Stat, v: f32) -> String {
    let (key, val) = match s {
        Stat::MaxHp => ("stat.max_hp", format!("+{}", v / 10.0)),
        Stat::Armor => ("stat.armor", format!("+{v}")),
        Stat::Amount => ("stat.amount", format!("+{v}")),
        Stat::Regen => ("stat.regen", format!("+{v:.1}")),
        Stat::Luck | Stat::Growth => (if s == Stat::Luck { "stat.luck" } else { "stat.growth" }, format!("{:+} %", (v * 100.0).round() as i32)),
        _ => {
            let k = match s {
                Stat::Area => "stat.area",
                Stat::Cooldown => "stat.cooldown",
                Stat::Duration => "stat.duration",
                Stat::Speed => "stat.speed",
                Stat::MoveSpeed => "stat.move_speed",
                Stat::Magnet => "stat.magnet",
                Stat::DashCooldown => "stat.dash_cd",
                _ => "stat.might",
            };
            (k, format!("{:+} %", (v * 100.0).round() as i32))
        }
    };
    format!("{val} {}", lang.get(key))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn meta_loads_and_applies() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let meta = Meta::load(&root);
        assert_eq!(meta.difficulties.len(), 4);
        let mut p = Profile::new();
        let base = meta.run_mods(&p);
        assert_eq!(base.enemy_hp, 1.0);
        p.shop.insert("heart".into(), 2);
        p.blessings.insert("fairy".into());
        p.seals.insert("seal_might".into());
        p.difficulty = 3;
        let m = meta.run_mods(&p);
        assert_eq!(m.revives, 1);
        assert!(m.stats.iter().any(|&(s, v)| s == Stat::MaxHp && v == 20.0));
        assert!((m.enemy_hp - 3.0).abs() < 1e-4);
        assert!(meta.bank_mult(&p) > 2.0);
        // relances / passes : 2 de base, plus la boutique et les reliques equipees
        assert_eq!((base.rerolls, base.skips), (2, 2));
        let g = meta.bank_mult(&p);
        p.shop.insert("reroll".into(), 2);
        p.shop.insert("skip".into(), 1);
        for id in ["lens_of_truth", "kinstone"] {
            p.relics.insert(id.into());
            p.equipped.push(id.into());
        }
        let m = meta.run_mods(&p);
        assert_eq!((m.rerolls, m.skips), (5, 4));
        assert_eq!(meta.bank_mult(&p), g, "les relances ne comptent pas comme Avarice");
        assert_eq!(meta.shop_cost(&meta.shop[0], 0), Some(150));
        assert_eq!(meta.shop_cost(&meta.shop[0], 5), None);
        for r in meta.relics.iter().filter(|r| !r.encounter) {
            assert!(root.join(format!("data/maps/{}.ron", r.stage)).exists(), "relique {} : stage inconnu", r.id);
        }
    }
}
